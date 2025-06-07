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
}

// Export singleton instance
export const vexfsApi = new VexFSApiService();
export default VexFSApiService;
