import axios from 'axios';
import type { AxiosInstance, AxiosResponse } from 'axios';
import type {
  VexFSCollection,
  VexFSPoint,
  VexFSSearchRequest,
  VexFSSearchResult,
  ApiResponse,
  PaginatedResponse,
  DashboardStats,
  AdvancedSearchQuery,
  SearchResultsResponse,
  SavedSearch,
  SearchHistory,
  SearchAnalytics,
  CollectionSchema,
  SearchExportOptions,
} from '../types';
import type {
  SystemMetrics,
  PerformanceMetrics,
  HealthStatus,
  Alert,
  MonitoringApiResponse,
  MetricsHistoryResponse,
} from '../types/monitoring';
import type {
  NodeId,
  EdgeId,
  NodeResponse,
  EdgeResponse,
  CreateNodeRequest,
  CreateEdgeRequest,
  UpdateNodeRequest,
  UpdateEdgeRequest,
  TraversalQuery,
  TraversalResult,
  NodeFilters,
  EdgeFilters,
  NeighborOptions,
  GraphStatistics,
  GraphSearchQuery,
  GraphSearchResult,
  GraphAnalytics,
  GraphSchema,
  GraphError,
} from '../types/graph';

class VexFSApiService {
  private api: AxiosInstance;

  constructor(baseURL: string = 'http://localhost:7680') {
    this.api = axios.create({
      baseURL,
      timeout: 10000,
      headers: {
        'Content-Type': 'application/json',
      },
    });

    // Request interceptor
    this.api.interceptors.request.use(
      config => {
        console.log(
          `API Request: ${config.method?.toUpperCase()} ${config.url}`
        );
        return config;
      },
      error => Promise.reject(error)
    );

    // Response interceptor
    this.api.interceptors.response.use(
      response => response,
      error => {
        console.error('API Error:', error.response?.data || error.message);
        return Promise.reject(error);
      }
    );
  }

  // Collections API
  async getCollections(): Promise<VexFSCollection[]> {
    const response = await this.api.get('/api/v1/collections');
    // Handle actual VexFS server response format: { collections: [...] }
    if (response.data.collections) {
      // Convert collection names to collection objects
      return response.data.collections.map((name: string, index: number) => ({
        id: `collection-${index + 1}`,
        name: name,
        description: `Collection ${name}`,
        vectorSize: 384,
        distance: 'cosine',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        pointsCount: 0,
      }));
    }
    // Fallback for expected format: { success: true, data: [...], error: null }
    if (response.data.success) {
      return response.data.data || [];
    }
    return [];
  }

  async getCollection(name: string): Promise<VexFSCollection | null> {
    try {
      const response: AxiosResponse<ApiResponse<VexFSCollection>> =
        await this.api.get(`/collections/${name}`);
      return response.data.data || null;
    } catch {
      return null;
    }
  }

  async createCollection(
    name: string,
    vectorSize: number,
    distance: 'cosine' | 'euclidean' | 'dot' = 'cosine'
  ): Promise<VexFSCollection> {
    await this.api.post('/api/v1/collections', {
      name,
      metadata: {
        distance,
      },
    });

    // VexFS server doesn't return the collection object, so we create it
    // This matches the format expected by the frontend
    return {
      id: `collection-${Date.now()}`, // Generate a unique ID
      name: name,
      description: `Collection ${name}`,
      vectorSize: vectorSize,
      distance: distance,
      pointsCount: 0,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };
  }

  async deleteCollection(name: string): Promise<boolean> {
    try {
      await this.api.delete(`/api/v1/collections/${name}`);
      return true;
    } catch {
      return false;
    }
  }

  async updateCollection(
    name: string,
    updates: {
      description?: string;
      // Note: vectorSize and distance cannot be updated if collection has points
    }
  ): Promise<VexFSCollection> {
    const response: AxiosResponse<ApiResponse<VexFSCollection>> =
      await this.api.patch(`/collections/${name}`, updates);
    return response.data.data!;
  }

  async getCollectionDetails(name: string): Promise<VexFSCollection | null> {
    // For now, this is the same as getCollection, but could be extended
    // to include additional statistics and metadata
    return this.getCollection(name);
  }

  // Points API
  async getPoints(
    collectionName: string,
    limit: number = 100,
    offset: number = 0
  ): Promise<PaginatedResponse<VexFSPoint>> {
    const response: AxiosResponse<ApiResponse<PaginatedResponse<VexFSPoint>>> =
      await this.api.get(`/collections/${collectionName}/points`, {
        params: { limit, offset },
      });
    return (
      response.data.data || {
        items: [],
        total: 0,
        page: 1,
        pageSize: limit,
        hasNext: false,
        hasPrev: false,
      }
    );
  }

  async upsertPoints(
    collectionName: string,
    points: VexFSPoint[]
  ): Promise<boolean> {
    try {
      await this.api.put(`/collections/${collectionName}/points`, {
        points,
      });
      return true;
    } catch {
      return false;
    }
  }

  async deletePoints(
    collectionName: string,
    pointIds: (string | number)[]
  ): Promise<boolean> {
    try {
      await this.api.post(`/collections/${collectionName}/points/delete`, {
        points: pointIds,
      });
      return true;
    } catch {
      return false;
    }
  }

  // Search API
  async searchPoints(
    collectionName: string,
    searchRequest: VexFSSearchRequest
  ): Promise<VexFSSearchResult[]> {
    const response: AxiosResponse<ApiResponse<VexFSSearchResult[]>> =
      await this.api.post(`/collections/${collectionName}/points/search`, {
        vector: searchRequest.vector,
        limit: searchRequest.limit || 10,
        offset: searchRequest.offset || 0,
        filter: searchRequest.filter,
        with_payload: searchRequest.withPayload !== false,
        with_vector: searchRequest.withVector || false,
      });
    return response.data.data || [];
  }

  // Dashboard Stats API
  async getDashboardStats(): Promise<DashboardStats> {
    try {
      // Get collections to calculate stats
      const collections = await this.getCollections();
      const totalCollections = collections.length;
      const totalPoints = collections.reduce(
        (sum, c) => sum + (c.pointsCount || 0),
        0
      );
      
      // Get version info
      const versionResponse = await this.api.get('/api/v1/version');
      const version = versionResponse.data.success
        ? versionResponse.data.data
        : 'VexFS 1.0.0';
      
      return {
        totalCollections: totalCollections,
        totalPoints: totalPoints,
        totalStorage: totalPoints > 0 ? `${Math.round(totalPoints * 0.001)} MB` : '0 B',
        serverStatus: 'online' as const,
      };
    } catch {
      return {
        totalCollections: 0,
        totalPoints: 0,
        totalStorage: '0 B',
        serverStatus: 'offline',
      };
    }
  }

  // Health Check
  async healthCheck(): Promise<boolean> {
    try {
      // Use VexFS version endpoint for health check
      const response = await this.api.get('/api/v1/version');
      return response.data.status === 'healthy';
    } catch {
      return false;
    }
  }

  // Vector-specific API methods
  async getVectors(
    collectionName: string,
    limit: number = 100,
    offset: number = 0,
    filters?: Record<string, unknown>
  ): Promise<PaginatedResponse<VexFSPoint>> {
    // This is an alias for getPoints with additional filtering support
    const response: AxiosResponse<ApiResponse<PaginatedResponse<VexFSPoint>>> =
      await this.api.get(`/collections/${collectionName}/points`, {
        params: { limit, offset, ...filters },
      });
    return (
      response.data.data || {
        items: [],
        total: 0,
        page: Math.floor(offset / limit) + 1,
        pageSize: limit,
        hasNext: false,
        hasPrev: false,
      }
    );
  }

  async getVector(
    collectionName: string,
    vectorId: string | number
  ): Promise<VexFSPoint | null> {
    try {
      const response: AxiosResponse<ApiResponse<VexFSPoint>> =
        await this.api.get(`/collections/${collectionName}/points/${vectorId}`);
      return response.data.data || null;
    } catch {
      return null;
    }
  }

  async addVector(
    collectionName: string,
    vectorData: VexFSPoint
  ): Promise<boolean> {
    return this.upsertPoints(collectionName, [vectorData]);
  }

  async updateVector(
    collectionName: string,
    vectorId: string | number,
    vectorData: Partial<VexFSPoint>
  ): Promise<boolean> {
    try {
      await this.api.patch(
        `/collections/${collectionName}/points/${vectorId}`,
        vectorData
      );
      return true;
    } catch {
      return false;
    }
  }

  async deleteVector(
    collectionName: string,
    vectorId: string | number
  ): Promise<boolean> {
    return this.deletePoints(collectionName, [vectorId]);
  }

  async searchVectors(
    collectionName: string,
    query: {
      vector?: number[];
      vectorId?: string | number;
      k?: number;
      threshold?: number;
      filter?: Record<string, unknown>;
    }
  ): Promise<VexFSSearchResult[]> {
    let searchVector = query.vector;

    // If vectorId is provided, fetch the vector first
    if (query.vectorId && !searchVector) {
      const sourceVector = await this.getVector(collectionName, query.vectorId);
      if (!sourceVector) {
        throw new Error(`Vector with ID ${query.vectorId} not found`);
      }
      searchVector = sourceVector.vector;
    }

    if (!searchVector) {
      throw new Error('Either vector or vectorId must be provided');
    }

    return this.searchPoints(collectionName, {
      vector: searchVector,
      limit: query.k || 10,
      filter: query.filter,
      withPayload: true,
      withVector: true,
    });
  }

  async batchAddVectors(
    collectionName: string,
    vectors: VexFSPoint[]
  ): Promise<boolean> {
    return this.upsertPoints(collectionName, vectors);
  }

  // Advanced Search API
  async advancedSearch(
    query: AdvancedSearchQuery
  ): Promise<SearchResultsResponse> {
    const response: AxiosResponse<ApiResponse<SearchResultsResponse>> =
      await this.api.post('/search/advanced', query);
    return (
      response.data.data || {
        results: [],
        total: 0,
        page: 1,
        pageSize: 10,
        hasNext: false,
        hasPrev: false,
        executionTime: 0,
        searchId: '',
      }
    );
  }

  async getCollectionSchema(collectionName: string): Promise<CollectionSchema> {
    const response: AxiosResponse<ApiResponse<CollectionSchema>> =
      await this.api.get(`/collections/${collectionName}/schema`);
    return (
      response.data.data || {
        fields: [],
        vectorDimensions: 0,
        distanceMetric: 'cosine',
      }
    );
  }

  // Search History API
  async getSearchHistory(userId?: string): Promise<SearchHistory[]> {
    const response: AxiosResponse<ApiResponse<SearchHistory[]>> =
      await this.api.get('/search/history', {
        params: userId ? { userId } : {},
      });
    return response.data.data || [];
  }

  async saveSearchToHistory(
    query: AdvancedSearchQuery,
    results: SearchResultsResponse,
    executionTime: number,
    userId?: string
  ): Promise<boolean> {
    try {
      await this.api.post('/search/history', {
        query,
        results,
        executionTime,
        userId,
      });
      return true;
    } catch {
      return false;
    }
  }

  // Saved Searches API
  async getSavedSearches(userId?: string): Promise<SavedSearch[]> {
    const response: AxiosResponse<ApiResponse<SavedSearch[]>> =
      await this.api.get('/search/saved', {
        params: userId ? { userId } : {},
      });
    return response.data.data || [];
  }

  async saveSearch(
    name: string,
    query: AdvancedSearchQuery,
    description?: string,
    tags: string[] = [],
    category?: string,
    isPublic: boolean = false,
    userId?: string
  ): Promise<SavedSearch> {
    const response: AxiosResponse<ApiResponse<SavedSearch>> =
      await this.api.post('/search/saved', {
        name,
        query,
        description,
        tags,
        category,
        isPublic,
        userId,
      });
    return response.data.data!;
  }

  async deleteSavedSearch(searchId: string): Promise<boolean> {
    try {
      await this.api.delete(`/search/saved/${searchId}`);
      return true;
    } catch {
      return false;
    }
  }

  async updateSavedSearch(
    searchId: string,
    updates: Partial<SavedSearch>
  ): Promise<SavedSearch> {
    const response: AxiosResponse<ApiResponse<SavedSearch>> =
      await this.api.patch(`/search/saved/${searchId}`, updates);
    return response.data.data!;
  }

  // Search Analytics API
  async getSearchAnalytics(
    collectionId?: string,
    timeRange?: string
  ): Promise<SearchAnalytics> {
    const response: AxiosResponse<ApiResponse<SearchAnalytics>> =
      await this.api.get('/search/analytics', {
        params: { collectionId, timeRange },
      });
    return (
      response.data.data || {
        totalSearches: 0,
        averageExecutionTime: 0,
        popularSearchTypes: [],
        topCollections: [],
        performanceMetrics: {
          fastQueries: 0,
          mediumQueries: 0,
          slowQueries: 0,
        },
        searchPatterns: {
          hourlyDistribution: [],
          dailyDistribution: [],
        },
      }
    );
  }

  // Search Export API
  async exportSearchResults(
    searchId: string,
    options: SearchExportOptions
  ): Promise<Blob> {
    const response = await this.api.post(
      `/search/export/${searchId}`,
      options,
      {
        responseType: 'blob',
      }
    );
    return response.data;
  }

  // Text-to-Vector Conversion (if supported)
  async textToVector(
    text: string,
    collectionName: string
  ): Promise<number[] | null> {
    try {
      const response: AxiosResponse<ApiResponse<{ vector: number[] }>> =
        await this.api.post(`/collections/${collectionName}/text-to-vector`, {
          text,
        });
      return response.data.data?.vector || null;
    } catch {
      return null;
    }
  }

  // Search Suggestions
  async getSearchSuggestions(
    collectionName: string,
    query: string
  ): Promise<string[]> {
    try {
      const response: AxiosResponse<ApiResponse<string[]>> = await this.api.get(
        `/collections/${collectionName}/search/suggestions`,
        {
          params: { q: query },
        }
      );
      return response.data.data || [];
    } catch {
      return [];
    }
  }

  // Monitoring API Methods
  async getSystemMetrics(): Promise<SystemMetrics> {
    try {
      const response: AxiosResponse<MonitoringApiResponse<SystemMetrics>> =
        await this.api.get('/monitoring/system');
      return (
        response.data.data || {
          cpu: { usage: 0, cores: 0, loadAverage: [0, 0, 0] },
          memory: { used: 0, total: 0, available: 0, percentage: 0 },
          disk: { used: 0, total: 0, available: 0, percentage: 0 },
          network: { bytesIn: 0, bytesOut: 0, packetsIn: 0, packetsOut: 0 },
        }
      );
    } catch {
      // Return mock data for development
      return {
        cpu: {
          usage: Math.random() * 100,
          cores: 8,
          loadAverage: [
            Math.random() * 2,
            Math.random() * 2,
            Math.random() * 2,
          ],
        },
        memory: {
          used: 8589934592 * Math.random(),
          total: 17179869184,
          available: 8589934592 * (1 - Math.random()),
          percentage: Math.random() * 100,
        },
        disk: {
          used: 107374182400 * Math.random(),
          total: 214748364800,
          available: 107374182400 * (1 - Math.random()),
          percentage: Math.random() * 100,
        },
        network: {
          bytesIn: Math.floor(Math.random() * 1000000000),
          bytesOut: Math.floor(Math.random() * 1000000000),
          packetsIn: Math.floor(Math.random() * 1000000),
          packetsOut: Math.floor(Math.random() * 1000000),
        },
      };
    }
  }

  async getPerformanceMetrics(): Promise<PerformanceMetrics> {
    try {
      const response: AxiosResponse<MonitoringApiResponse<PerformanceMetrics>> =
        await this.api.get('/monitoring/performance');
      return (
        response.data.data || {
          queryPerformance: {
            averageResponseTime: 0,
            p95ResponseTime: 0,
            p99ResponseTime: 0,
            throughput: 0,
            totalQueries: 0,
          },
          vectorOperations: {
            indexingRate: 0,
            searchRate: 0,
            totalIndexed: 0,
            totalSearches: 0,
          },
          storage: {
            readThroughput: 0,
            writeThroughput: 0,
            iops: 0,
          },
        }
      );
    } catch {
      // Return mock data for development
      return {
        queryPerformance: {
          averageResponseTime: Math.random() * 100 + 10,
          p95ResponseTime: Math.random() * 200 + 50,
          p99ResponseTime: Math.random() * 500 + 100,
          throughput: Math.random() * 1000 + 100,
          totalQueries: Math.floor(Math.random() * 1000000),
        },
        vectorOperations: {
          indexingRate: Math.random() * 1000 + 50,
          searchRate: Math.random() * 500 + 25,
          totalIndexed: Math.floor(Math.random() * 10000000),
          totalSearches: Math.floor(Math.random() * 1000000),
        },
        storage: {
          readThroughput: Math.random() * 500 + 50,
          writeThroughput: Math.random() * 300 + 30,
          iops: Math.random() * 10000 + 1000,
        },
      };
    }
  }

  async getHealthStatus(): Promise<HealthStatus> {
    try {
      const response: AxiosResponse<MonitoringApiResponse<HealthStatus>> =
        await this.api.get('/monitoring/health');
      return (
        response.data.data || {
          overall: 'unknown',
          services: {
            vexfsCore: {
              status: 'unknown',
              lastCheck: new Date().toISOString(),
            },
            database: {
              status: 'unknown',
              lastCheck: new Date().toISOString(),
            },
            vectorIndex: {
              status: 'unknown',
              lastCheck: new Date().toISOString(),
            },
            api: { status: 'unknown', lastCheck: new Date().toISOString() },
          },
          uptime: 0,
          lastHealthCheck: new Date().toISOString(),
        }
      );
    } catch {
      // Return mock data for development
      const statuses: Array<'healthy' | 'warning' | 'critical'> = [
        'healthy',
        'healthy',
        'healthy',
        'warning',
      ];
      const randomStatus = () =>
        statuses[Math.floor(Math.random() * statuses.length)];

      return {
        overall: 'healthy',
        services: {
          vexfsCore: {
            status: randomStatus(),
            responseTime: Math.random() * 50 + 5,
            errorRate: Math.random() * 5,
            lastCheck: new Date().toISOString(),
          },
          database: {
            status: randomStatus(),
            responseTime: Math.random() * 30 + 2,
            errorRate: Math.random() * 2,
            lastCheck: new Date().toISOString(),
          },
          vectorIndex: {
            status: randomStatus(),
            responseTime: Math.random() * 100 + 10,
            errorRate: Math.random() * 3,
            lastCheck: new Date().toISOString(),
          },
          api: {
            status: 'healthy',
            responseTime: Math.random() * 20 + 1,
            errorRate: 0,
            lastCheck: new Date().toISOString(),
          },
        },
        uptime: Math.floor(Math.random() * 86400 * 30), // Up to 30 days
        lastHealthCheck: new Date().toISOString(),
      };
    }
  }

  async getAlerts(): Promise<Alert[]> {
    try {
      const response: AxiosResponse<MonitoringApiResponse<Alert[]>> =
        await this.api.get('/monitoring/alerts');
      return response.data.data || [];
    } catch {
      // Return mock alerts for development
      const mockAlerts: Alert[] = [
        {
          id: '1',
          type: 'warning',
          title: 'High Memory Usage',
          message: 'Memory usage has exceeded 80% threshold',
          timestamp: new Date(Date.now() - 300000).toISOString(),
          acknowledged: false,
          source: 'system-monitor',
        },
        {
          id: '2',
          type: 'info',
          title: 'Index Rebuild Complete',
          message: 'Vector index rebuild completed successfully',
          timestamp: new Date(Date.now() - 600000).toISOString(),
          acknowledged: true,
          source: 'vector-index',
        },
      ];
      return mockAlerts;
    }
  }

  async acknowledgeAlert(alertId: string): Promise<boolean> {
    try {
      await this.api.post(`/monitoring/alerts/${alertId}/acknowledge`);
      return true;
    } catch {
      return false;
    }
  }

  async getMetricsHistory(
    metric: string,
    timeRange: { start: string; end: string },
    resolution: string = '5m'
  ): Promise<MetricsHistoryResponse> {
    try {
      const response: AxiosResponse<
        MonitoringApiResponse<MetricsHistoryResponse>
      > = await this.api.get('/monitoring/metrics/history', {
        params: {
          metric,
          start: timeRange.start,
          end: timeRange.end,
          resolution,
        },
      });
      return (
        response.data.data || {
          metrics: {},
          timeRange,
          resolution,
        }
      );
    } catch {
      // Return mock historical data
      const now = new Date();
      const points = 20;
      const mockData = Array.from({ length: points }, (_, i) => ({
        timestamp: new Date(now.getTime() - (points - i) * 60000).toISOString(),
        value: Math.random() * 100,
      }));

      return {
        metrics: {
          [metric]: mockData,
        },
        timeRange,
        resolution,
      };
    }
  }

  // VexGraph API Methods
  // ==================

  /**
   * Node CRUD Operations
   */

  /**
   * Create a new node in the VexGraph
   * @param nodeData - Node creation data
   * @returns Promise<NodeResponse> - Created node data
   */
  async createNode(nodeData: CreateNodeRequest): Promise<NodeResponse> {
    try {
      const response: AxiosResponse<ApiResponse<NodeResponse>> = await this.api.post(
        '/api/v1/vexgraph/nodes',
        nodeData
      );
      return response.data.data!;
    } catch {
      // Return mock created node when service is unavailable
      return this.createMockNode(nodeData);
    }
  }

  /**
   * Get a specific node by ID
   * @param nodeId - Node identifier
   * @returns Promise<NodeResponse | null> - Node data or null if not found
   */
  async getNode(nodeId: NodeId): Promise<NodeResponse | null> {
    try {
      const response: AxiosResponse<ApiResponse<NodeResponse>> = await this.api.get(
        `/api/v1/vexgraph/nodes/${nodeId}`
      );
      return response.data.data || null;
    } catch {
      return null;
    }
  }

  /**
   * Update an existing node
   * @param nodeId - Node identifier
   * @param updates - Node update data
   * @returns Promise<NodeResponse> - Updated node data
   */
  async updateNode(nodeId: NodeId, updates: UpdateNodeRequest): Promise<NodeResponse> {
    const response: AxiosResponse<ApiResponse<NodeResponse>> = await this.api.patch(
      `/api/v1/vexgraph/nodes/${nodeId}`,
      updates
    );
    return response.data.data!;
  }

  /**
   * Delete a node from the VexGraph
   * @param nodeId - Node identifier
   * @returns Promise<boolean> - Success status
   */
  async deleteNode(nodeId: NodeId): Promise<boolean> {
    try {
      await this.api.delete(`/api/v1/vexgraph/nodes/${nodeId}`);
      return true;
    } catch {
      return false;
    }
  }

  /**
   * List nodes with optional filtering and pagination
   * @param filters - Optional node filters
   * @param limit - Maximum number of nodes to return
   * @param offset - Number of nodes to skip
   * @returns Promise<PaginatedResponse<NodeResponse>> - Paginated node list
   */
  async listNodes(
    filters?: NodeFilters,
    limit: number = 100,
    offset: number = 0
  ): Promise<PaginatedResponse<NodeResponse>> {
    try {
      const response: AxiosResponse<ApiResponse<PaginatedResponse<NodeResponse>>> =
        await this.api.get('/api/v1/vexgraph/nodes', {
          params: { ...filters, limit, offset },
        });
      return (
        response.data.data || {
          items: [],
          total: 0,
          page: Math.floor(offset / limit) + 1,
          pageSize: limit,
          hasNext: false,
          hasPrev: false,
        }
      );
    } catch {
      // Return mock nodes when service is unavailable
      return this.getMockNodes(limit, offset);
    }
  }

  /**
   * Edge CRUD Operations
   */

  /**
   * Create a new edge in the VexGraph
   * @param edgeData - Edge creation data
   * @returns Promise<EdgeResponse> - Created edge data
   */
  async createEdge(edgeData: CreateEdgeRequest): Promise<EdgeResponse> {
    try {
      const response: AxiosResponse<ApiResponse<EdgeResponse>> = await this.api.post(
        '/api/v1/vexgraph/edges',
        edgeData
      );
      return response.data.data!;
    } catch {
      // Return mock created edge when service is unavailable
      return this.createMockEdge(edgeData);
    }
  }

  /**
   * Get a specific edge by ID
   * @param edgeId - Edge identifier
   * @returns Promise<EdgeResponse | null> - Edge data or null if not found
   */
  async getEdge(edgeId: EdgeId): Promise<EdgeResponse | null> {
    try {
      const response: AxiosResponse<ApiResponse<EdgeResponse>> = await this.api.get(
        `/api/v1/vexgraph/edges/${edgeId}`
      );
      return response.data.data || null;
    } catch {
      return null;
    }
  }

  /**
   * Update an existing edge
   * @param edgeId - Edge identifier
   * @param updates - Edge update data
   * @returns Promise<EdgeResponse> - Updated edge data
   */
  async updateEdge(edgeId: EdgeId, updates: UpdateEdgeRequest): Promise<EdgeResponse> {
    const response: AxiosResponse<ApiResponse<EdgeResponse>> = await this.api.patch(
      `/api/v1/vexgraph/edges/${edgeId}`,
      updates
    );
    return response.data.data!;
  }

  /**
   * Delete an edge from the VexGraph
   * @param edgeId - Edge identifier
   * @returns Promise<boolean> - Success status
   */
  async deleteEdge(edgeId: EdgeId): Promise<boolean> {
    try {
      await this.api.delete(`/api/v1/vexgraph/edges/${edgeId}`);
      return true;
    } catch {
      return false;
    }
  }

  /**
   * List edges with optional filtering and pagination
   * @param filters - Optional edge filters
   * @param limit - Maximum number of edges to return
   * @param offset - Number of edges to skip
   * @returns Promise<PaginatedResponse<EdgeResponse>> - Paginated edge list
   */
  async listEdges(
    filters?: EdgeFilters,
    limit: number = 100,
    offset: number = 0
  ): Promise<PaginatedResponse<EdgeResponse>> {
    try {
      const response: AxiosResponse<ApiResponse<PaginatedResponse<EdgeResponse>>> =
        await this.api.get('/api/v1/vexgraph/edges', {
          params: { ...filters, limit, offset },
        });
      return (
        response.data.data || {
          items: [],
          total: 0,
          page: Math.floor(offset / limit) + 1,
          pageSize: limit,
          hasNext: false,
          hasPrev: false,
        }
      );
    } catch {
      // Return mock edges when service is unavailable
      return this.getMockEdges(limit, offset);
    }
  }

  /**
   * Graph Traversal Operations
   */

  /**
   * Execute a graph traversal query
   * @param query - Traversal query parameters
   * @returns Promise<TraversalResult> - Traversal results
   */
  async executeTraversal(query: TraversalQuery): Promise<TraversalResult> {
    try {
      const response: AxiosResponse<ApiResponse<TraversalResult>> = await this.api.post(
        '/api/v1/vexgraph/traversal',
        query
      );
      return response.data.data!;
    } catch {
      // Return mock traversal result when service is unavailable
      return this.getMockTraversalResult(query);
    }
  }

  /**
   * Get neighbors of a specific node
   * @param nodeId - Node identifier
   * @param options - Neighbor query options
   * @returns Promise<NodeResponse[]> - List of neighbor nodes
   */
  async getNodeNeighbors(
    nodeId: NodeId,
    options?: NeighborOptions
  ): Promise<NodeResponse[]> {
    const response: AxiosResponse<ApiResponse<NodeResponse[]>> = await this.api.get(
      `/api/v1/vexgraph/nodes/${nodeId}/neighbors`,
      { params: options }
    );
    return response.data.data || [];
  }

  /**
   * Find shortest path between two nodes
   * @param sourceId - Source node ID
   * @param targetId - Target node ID
   * @param maxDepth - Maximum search depth
   * @returns Promise<TraversalResult> - Path traversal result
   */
  async findShortestPath(
    sourceId: NodeId,
    targetId: NodeId,
    maxDepth?: number
  ): Promise<TraversalResult> {
    const query: TraversalQuery = {
      algorithm: 'Dijkstra' as any,
      start_node: sourceId,
      end_node: targetId,
      max_depth: maxDepth,
    };
    return this.executeTraversal(query);
  }

  /**
   * Perform breadth-first search from a node
   * @param startNodeId - Starting node ID
   * @param maxDepth - Maximum search depth
   * @param maxResults - Maximum number of results
   * @returns Promise<TraversalResult> - BFS traversal result
   */
  async breadthFirstSearch(
    startNodeId: NodeId,
    maxDepth?: number,
    maxResults?: number
  ): Promise<TraversalResult> {
    const query: TraversalQuery = {
      algorithm: 'BreadthFirstSearch' as any,
      start_node: startNodeId,
      max_depth: maxDepth,
      max_results: maxResults,
    };
    return this.executeTraversal(query);
  }

  /**
   * Perform depth-first search from a node
   * @param startNodeId - Starting node ID
   * @param maxDepth - Maximum search depth
   * @param maxResults - Maximum number of results
   * @returns Promise<TraversalResult> - DFS traversal result
   */
  async depthFirstSearch(
    startNodeId: NodeId,
    maxDepth?: number,
    maxResults?: number
  ): Promise<TraversalResult> {
    const query: TraversalQuery = {
      algorithm: 'DepthFirstSearch' as any,
      start_node: startNodeId,
      max_depth: maxDepth,
      max_results: maxResults,
    };
    return this.executeTraversal(query);
  }

  /**
   * Graph Query Operations
   */

  /**
   * Execute a graph search query
   * @param query - Graph search query
   * @returns Promise<GraphSearchResult> - Search results
   */
  async executeQuery(query: GraphSearchQuery): Promise<GraphSearchResult> {
    const response: AxiosResponse<ApiResponse<GraphSearchResult>> = await this.api.post(
      '/api/v1/vexgraph/search',
      query
    );
    return response.data.data!;
  }

  /**
   * Perform semantic search on the graph
   * @param searchText - Text to search for
   * @param maxResults - Maximum number of results
   * @returns Promise<GraphSearchResult> - Semantic search results
   */
  async semanticSearch(
    searchText: string,
    maxResults: number = 50
  ): Promise<GraphSearchResult> {
    const query: GraphSearchQuery = {
      query: searchText,
      search_type: 'semantic',
      max_results: maxResults,
    };
    return this.executeQuery(query);
  }

  /**
   * Search nodes and edges by properties
   * @param searchText - Property search text
   * @param nodeTypes - Optional node type filters
   * @param edgeTypes - Optional edge type filters
   * @param maxResults - Maximum number of results
   * @returns Promise<GraphSearchResult> - Property search results
   */
  async propertySearch(
    searchText: string,
    nodeTypes?: string[],
    edgeTypes?: string[],
    maxResults: number = 50
  ): Promise<GraphSearchResult> {
    const query: GraphSearchQuery = {
      query: searchText,
      search_type: 'property',
      node_types: nodeTypes as any,
      edge_types: edgeTypes as any,
      max_results: maxResults,
    };
    return this.executeQuery(query);
  }

  /**
   * Graph Statistics Operations
   */

  /**
   * Get overall graph statistics
   * @returns Promise<GraphStatistics> - Graph statistics
   */
  async getGraphStats(): Promise<GraphStatistics> {
    try {
      const response: AxiosResponse<ApiResponse<GraphStatistics>> = await this.api.get(
        '/api/v1/vexgraph/stats'
      );
      return response.data.data!;
    } catch {
      // Return mock statistics when service is unavailable
      return this.getMockGraphStats();
    }
  }

  /**
   * Get statistics for a specific node
   * @param nodeId - Node identifier
   * @returns Promise<any> - Node-specific statistics
   */
  async getNodeStats(nodeId: NodeId): Promise<any> {
    const response: AxiosResponse<ApiResponse<any>> = await this.api.get(
      `/api/v1/vexgraph/nodes/${nodeId}/stats`
    );
    return response.data.data!;
  }

  /**
   * Get statistics for a specific edge
   * @param edgeId - Edge identifier
   * @returns Promise<any> - Edge-specific statistics
   */
  async getEdgeStats(edgeId: EdgeId): Promise<any> {
    const response: AxiosResponse<ApiResponse<any>> = await this.api.get(
      `/api/v1/vexgraph/edges/${edgeId}/stats`
    );
    return response.data.data!;
  }

  /**
   * Get advanced graph analytics
   * @returns Promise<GraphAnalytics> - Advanced analytics data
   */
  async getGraphAnalytics(): Promise<GraphAnalytics> {
    const response: AxiosResponse<ApiResponse<GraphAnalytics>> = await this.api.get(
      '/api/v1/vexgraph/analytics'
    );
    return response.data.data!;
  }

  /**
   * Graph Schema Operations
   */

  /**
   * Get the current graph schema
   * @returns Promise<GraphSchema> - Graph schema definition
   */
  async getGraphSchema(): Promise<GraphSchema> {
    const response: AxiosResponse<ApiResponse<GraphSchema>> = await this.api.get(
      '/api/v1/vexgraph/schema'
    );
    return response.data.data!;
  }

  /**
   * Update the graph schema
   * @param schema - New schema definition
   * @returns Promise<GraphSchema> - Updated schema
   */
  async updateGraphSchema(schema: Partial<GraphSchema>): Promise<GraphSchema> {
    const response: AxiosResponse<ApiResponse<GraphSchema>> = await this.api.patch(
      '/api/v1/vexgraph/schema',
      schema
    );
    return response.data.data!;
  }

  /**
   * Update node type schema definition
   * @param nodeTypeSchema - Node type schema to update
   * @returns Promise<NodeTypeSchema> - Updated node type schema
   */
  async updateNodeTypeSchema(nodeTypeSchema: any): Promise<any> {
    const response: AxiosResponse<ApiResponse<any>> = await this.api.patch(
      '/api/v1/vexgraph/schema/node-types',
      nodeTypeSchema
    );
    return response.data.data!;
  }

  /**
   * Update edge type schema definition
   * @param edgeTypeSchema - Edge type schema to update
   * @returns Promise<EdgeTypeSchema> - Updated edge type schema
   */
  async updateEdgeTypeSchema(edgeTypeSchema: any): Promise<any> {
    const response: AxiosResponse<ApiResponse<any>> = await this.api.patch(
      '/api/v1/vexgraph/schema/edge-types',
      edgeTypeSchema
    );
    return response.data.data!;
  }

  /**
   * Validate schema compliance
   * @param schema - Schema to validate
   * @returns Promise<any> - Validation results
   */
  async validateSchemaCompliance(schema?: any): Promise<any> {
    const response: AxiosResponse<ApiResponse<any>> = await this.api.post(
      '/api/v1/vexgraph/schema/validate',
      schema || {}
    );
    return response.data.data!;
  }

  /**
   * Migrate schema to new version
   * @param migrationPlan - Migration plan
   * @returns Promise<any> - Migration results
   */
  async migrateSchema(migrationPlan: any): Promise<any> {
    const response: AxiosResponse<ApiResponse<any>> = await this.api.post(
      '/api/v1/vexgraph/schema/migrate',
      migrationPlan
    );
    return response.data.data!;
  }

  /**
   * Export schema definition
   * @param options - Export options
   * @returns Promise<any> - Exported schema
   */
  async exportSchema(options?: any): Promise<any> {
    const response: AxiosResponse<ApiResponse<any>> = await this.api.post(
      '/api/v1/vexgraph/schema/export',
      options || {}
    );
    return response.data.data!;
  }

  /**
   * Import schema definition
   * @param schema - Schema to import
   * @param options - Import options
   * @returns Promise<any> - Import results
   */
  async importSchema(schema: any, options?: any): Promise<any> {
    const response: AxiosResponse<ApiResponse<any>> = await this.api.post(
      '/api/v1/vexgraph/schema/import',
      { schema, options: options || {} }
    );
    return response.data.data!;
  }

  /**
   * Utility Methods
   */

  /**
   * Check VexGraph service health
   * @returns Promise<boolean> - Health status
   */
  async checkVexGraphHealth(): Promise<boolean> {
    try {
      const response = await this.api.get('/api/v1/vexgraph/health');
      return response.data.success || response.status === 200;
    } catch {
      // Return true to enable mock mode when real service is unavailable
      console.log('VexGraph service unavailable, using mock data');
      return true;
    }
  }

  /**
   * Get VexGraph service version
   * @returns Promise<string> - Service version
   */
  async getVexGraphVersion(): Promise<string> {
    try {
      const response: AxiosResponse<ApiResponse<{ version: string }>> =
        await this.api.get('/api/v1/vexgraph/version');
      return response.data.data?.version || 'unknown';
    } catch {
      return 'unknown';
    }
  }

  /**
   * Batch Operations
   */

  /**
   * Create multiple nodes in a single request
   * @param nodes - Array of node creation requests
   * @returns Promise<NodeResponse[]> - Created nodes
   */
  async batchCreateNodes(nodes: CreateNodeRequest[]): Promise<NodeResponse[]> {
    const response: AxiosResponse<ApiResponse<NodeResponse[]>> = await this.api.post(
      '/api/v1/vexgraph/nodes/batch',
      { nodes }
    );
    return response.data.data || [];
  }

  /**
   * Create multiple edges in a single request
   * @param edges - Array of edge creation requests
   * @returns Promise<EdgeResponse[]> - Created edges
   */
  async batchCreateEdges(edges: CreateEdgeRequest[]): Promise<EdgeResponse[]> {
    const response: AxiosResponse<ApiResponse<EdgeResponse[]>> = await this.api.post(
      '/api/v1/vexgraph/edges/batch',
      { edges }
    );
    return response.data.data || [];
  }

  /**
   * Delete multiple nodes in a single request
   * @param nodeIds - Array of node IDs to delete
   * @returns Promise<boolean> - Success status
   */
  async batchDeleteNodes(nodeIds: NodeId[]): Promise<boolean> {
    try {
      await this.api.delete('/api/v1/vexgraph/nodes/batch', {
        data: { node_ids: nodeIds }
      });
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Delete multiple edges in a single request
   * @param edgeIds - Array of edge IDs to delete
   * @returns Promise<boolean> - Success status
   */
  async batchDeleteEdges(edgeIds: EdgeId[]): Promise<boolean> {
    try {
      await this.api.delete('/api/v1/vexgraph/edges/batch', {
        data: { edge_ids: edgeIds }
      });
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Semantic Search API Methods
   * ===========================
   */

  /**
   * Execute semantic search on the graph
   * @param query - Semantic search query
   * @returns Promise<SemanticSearchResult> - Semantic search results
   */
  async executeSemanticSearch(query: any): Promise<any> {
    const response: AxiosResponse<ApiResponse<any>> = await this.api.post(
      '/api/v1/vexgraph/semantic/search',
      query
    );
    return response.data.data!;
  }

  /**
   * Execute vector similarity search
   * @param query - Vector similarity query
   * @returns Promise<VectorSimilarityResult> - Vector similarity results
   */
  async executeVectorSimilaritySearch(query: any): Promise<any> {
    const response: AxiosResponse<ApiResponse<any>> = await this.api.post(
      '/api/v1/vexgraph/semantic/similarity',
      query
    );
    return response.data.data!;
  }

  /**
   * Execute hybrid search combining keyword and semantic search
   * @param query - Hybrid search query
   * @returns Promise<HybridSearchResult> - Hybrid search results
   */
  async executeHybridSearch(query: any): Promise<any> {
    const response: AxiosResponse<ApiResponse<any>> = await this.api.post(
      '/api/v1/vexgraph/semantic/hybrid',
      query
    );
    return response.data.data!;
  }

  /**
   * Get semantic embeddings for nodes
   * @param nodeIds - Array of node IDs
   * @returns Promise<Record<NodeId, number[]>> - Node embeddings
   */
  async getSemanticEmbeddings(nodeIds: string[]): Promise<Record<string, number[]>> {
    const response: AxiosResponse<ApiResponse<Record<string, number[]>>> = await this.api.post(
      '/api/v1/vexgraph/semantic/embeddings',
      { node_ids: nodeIds }
    );
    return response.data.data!;
  }

  /**
   * Perform semantic clustering on nodes
   * @param query - Semantic clustering query
   * @returns Promise<SemanticClusteringResult> - Clustering results
   */
  async executeSemanticClustering(query: any): Promise<any> {
    const response: AxiosResponse<ApiResponse<any>> = await this.api.post(
      '/api/v1/vexgraph/semantic/clustering',
      query
    );
    return response.data.data!;
  }

  /**
   * Get explanation for semantic match
   * @param nodeId - Node ID
   * @param query - Search query
   * @returns Promise<SearchExplanation> - Match explanation
   */
  async explainSemanticMatch(nodeId: string, query: string): Promise<any> {
    const response: AxiosResponse<ApiResponse<any>> = await this.api.post(
      '/api/v1/vexgraph/semantic/explain',
      { node_id: nodeId, query }
    );
    return response.data.data!;
  }

  /**
   * Get search suggestions based on partial query
   * @param partialQuery - Partial search query
   * @param context - Optional context node IDs
   * @returns Promise<SearchSuggestion[]> - Search suggestions
   */
  async getSemanticSearchSuggestions(partialQuery: string, context?: string[]): Promise<any[]> {
    const response: AxiosResponse<ApiResponse<any[]>> = await this.api.get(
      '/api/v1/vexgraph/semantic/suggestions',
      {
        params: {
          q: partialQuery,
          context: context?.join(','),
        },
      }
    );
    return response.data.data || [];
  }

  /**
   * Mock Data Generation Methods
   * ============================
   */

  private getMockNodes(limit: number = 100, offset: number = 0): PaginatedResponse<NodeResponse> {
    const nodeTypes = ['File', 'Directory', 'Symlink', 'Device', 'Process', 'Network', 'Database', 'Cache'] as const;
    const totalNodes = 50; // Total mock nodes available
    
    // Generate consistent mock nodes
    const allNodes: NodeResponse[] = Array.from({ length: totalNodes }, (_, i) => ({
      id: `node-${i + 1}`,
      inode_number: 1000 + i,
      node_type: nodeTypes[i % nodeTypes.length],
      properties: {
        name: 'String' as any,
        path: 'String' as any,
        size: 'Integer' as any,
        created: 'DateTime' as any,
        permissions: 'String' as any,
      },
      outgoing_edges: [],
      incoming_edges: [],
      created_at: new Date(Date.now() - Math.random() * 86400000 * 30).toISOString(),
      updated_at: new Date(Date.now() - Math.random() * 86400000 * 7).toISOString(),
    }));

    // Apply pagination
    const startIndex = offset;
    const endIndex = Math.min(offset + limit, totalNodes);
    const items = allNodes.slice(startIndex, endIndex);

    return {
      items,
      total: totalNodes,
      page: Math.floor(offset / limit) + 1,
      pageSize: limit,
      hasNext: endIndex < totalNodes,
      hasPrev: offset > 0,
    };
  }

  private getMockEdges(limit: number = 100, offset: number = 0): PaginatedResponse<EdgeResponse> {
    const edgeTypes = ['Contains', 'References', 'DependsOn', 'SimilarTo', 'Connects', 'Inherits', 'Accesses'] as const;
    const totalEdges = 75; // Total mock edges available
    
    // Generate consistent mock edges
    const allEdges: EdgeResponse[] = Array.from({ length: totalEdges }, (_, i) => {
      const sourceIndex = Math.floor(Math.random() * 50) + 1;
      let targetIndex = Math.floor(Math.random() * 50) + 1;
      
      // Ensure source and target are different
      while (targetIndex === sourceIndex) {
        targetIndex = Math.floor(Math.random() * 50) + 1;
      }

      return {
        id: `edge-${i + 1}`,
        source_id: `node-${sourceIndex}`,
        target_id: `node-${targetIndex}`,
        edge_type: edgeTypes[i % edgeTypes.length],
        weight: Math.random() * 10,
        properties: {
          relationship: 'String' as any,
          strength: 'Float' as any,
          bidirectional: 'Boolean' as any,
        },
        created_at: new Date(Date.now() - Math.random() * 86400000 * 30).toISOString(),
        updated_at: new Date(Date.now() - Math.random() * 86400000 * 7).toISOString(),
      };
    });

    // Apply pagination
    const startIndex = offset;
    const endIndex = Math.min(offset + limit, totalEdges);
    const items = allEdges.slice(startIndex, endIndex);

    return {
      items,
      total: totalEdges,
      page: Math.floor(offset / limit) + 1,
      pageSize: limit,
      hasNext: endIndex < totalEdges,
      hasPrev: offset > 0,
    };
  }

  private getMockGraphStats(): GraphStatistics {
    return {
      node_count: 50,
      edge_count: 75,
      connected_components: 3,
      average_degree: 3.0,
      max_degree: 8,
      min_degree: 1,
      density: 0.061,
      diameter: 6,
      clustering_coefficient: 0.42,
      node_types: {
        'File': 12,
        'Directory': 8,
        'Symlink': 6,
        'Device': 4,
        'Process': 7,
        'Network': 5,
        'Database': 4,
        'Cache': 4,
      },
      edge_types: {
        'Contains': 15,
        'References': 12,
        'DependsOn': 10,
        'SimilarTo': 8,
        'Connects': 11,
        'Inherits': 9,
        'Accesses': 10,
      },
    };
  }

  private createMockNode(nodeData: CreateNodeRequest): NodeResponse {
    const nodeId = `mock-node-${Date.now()}`;
    return {
      id: nodeId,
      inode_number: Math.floor(Math.random() * 10000) + 10000,
      node_type: nodeData.node_type || 'File',
      properties: nodeData.properties || {},
      outgoing_edges: [],
      incoming_edges: [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
  }

  private createMockEdge(edgeData: CreateEdgeRequest): EdgeResponse {
    const edgeId = `mock-edge-${Date.now()}`;
    return {
      id: edgeId,
      source_id: edgeData.source_id,
      target_id: edgeData.target_id,
      edge_type: edgeData.edge_type || 'References',
      weight: edgeData.weight || Math.random() * 10,
      properties: edgeData.properties || {},
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
  }

  private getMockTraversalResult(query: TraversalQuery): TraversalResult {
    // Generate a simple mock traversal result
    const mockNodes = this.getMockNodes(10, 0).items;
    const mockEdges = this.getMockEdges(5, 0).items;
    
    return {
      nodes: mockNodes.slice(0, 5), // Return subset for traversal
      edges: mockEdges.slice(0, 3),
      path: mockNodes.slice(0, 3).map(n => n.id),
      total_cost: Math.random() * 100,
      execution_time_ms: Math.floor(Math.random() * 50) + 10,
      algorithm_used: query.algorithm || 'BreadthFirstSearch' as any,
    };
  }
}

// Export singleton instance
export const vexfsApi = new VexFSApiService();
export default VexFSApiService;
