import axios, { type AxiosInstance, type AxiosResponse } from 'axios';
import type {
  VexFSCollection,
  VexFSPoint,
  SearchResultsResponse,
} from '../types';

// Use a default API base URL if not configured
const API_BASE_URL = 'http://localhost:8080/api';

interface SearchQuery {
  vector?: number[];
  filters?: Record<string, any>;
  limit?: number;
  threshold?: number;
}

interface RetryConfig {
  maxRetries: number;
  baseDelay: number;
  maxDelay: number;
  retryCondition?: (error: any) => boolean;
}

interface ApiError extends Error {
  status?: number;
  code?: string;
  retryable?: boolean;
}

class EnhancedVexFSApi {
  private client: AxiosInstance;
  private retryConfig: RetryConfig;

  constructor() {
    this.retryConfig = {
      maxRetries: 3,
      baseDelay: 1000,
      maxDelay: 10000,
      retryCondition: (error) => {
        // Retry on network errors or 5xx status codes
        return !error.response || (error.response.status >= 500 && error.response.status < 600);
      },
    };

    this.client = axios.create({
      baseURL: API_BASE_URL,
      timeout: 30000,
      headers: {
        'Content-Type': 'application/json',
      },
    });

    this.setupInterceptors();
  }

  private setupInterceptors(): void {
    // Request interceptor for logging and auth
    this.client.interceptors.request.use(
      (config) => {
        if (import.meta.env.DEV) {
          console.log(`🚀 API Request: ${config.method?.toUpperCase()} ${config.url}`);
        }
        return config;
      },
      (error) => {
        console.error('❌ Request Error:', error);
        return Promise.reject(error);
      }
    );

    // Response interceptor for error handling
    this.client.interceptors.response.use(
      (response) => {
        if (import.meta.env.DEV) {
          console.log(`✅ API Response: ${response.status} ${response.config.url}`);
        }
        return response;
      },
      (error) => {
        const apiError = this.createApiError(error);
        console.error('❌ API Error:', apiError);
        return Promise.reject(apiError);
      }
    );
  }

  private createApiError(error: any): ApiError {
    const apiError = new Error() as ApiError;
    
    if (error.response) {
      // Server responded with error status
      apiError.message = error.response.data?.message || `HTTP ${error.response.status}`;
      apiError.status = error.response.status;
      apiError.code = error.response.data?.code || 'HTTP_ERROR';
      apiError.retryable = error.response.status >= 500;
    } else if (error.request) {
      // Network error
      apiError.message = 'Network error - please check your connection';
      apiError.code = 'NETWORK_ERROR';
      apiError.retryable = true;
    } else {
      // Other error
      apiError.message = error.message || 'Unknown error occurred';
      apiError.code = 'UNKNOWN_ERROR';
      apiError.retryable = false;
    }

    return apiError;
  }

  private async sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  private async executeWithRetry<T>(
    operation: () => Promise<AxiosResponse<T>>,
    config?: Partial<RetryConfig>
  ): Promise<T> {
    const finalConfig = { ...this.retryConfig, ...config };
    let lastError: ApiError;

    for (let attempt = 0; attempt <= finalConfig.maxRetries; attempt++) {
      try {
        const response = await operation();
        return response.data;
      } catch (error) {
        lastError = error as ApiError;

        // Don't retry if it's the last attempt or error is not retryable
        if (
          attempt === finalConfig.maxRetries ||
          !finalConfig.retryCondition?.(error) ||
          !lastError.retryable
        ) {
          throw lastError;
        }

        // Calculate delay with exponential backoff and jitter
        const baseDelay = Math.min(
          finalConfig.baseDelay * Math.pow(2, attempt),
          finalConfig.maxDelay
        );
        const jitter = Math.random() * 0.1 * baseDelay;
        const delay = baseDelay + jitter;

        if (import.meta.env.DEV) {
          console.log(`🔄 Retrying API call in ${delay.toFixed(0)}ms (attempt ${attempt + 1}/${finalConfig.maxRetries})`);
        }

        await this.sleep(delay);
      }
    }

    throw lastError!;
  }

  // Collections API
  async getCollections(): Promise<VexFSCollection[]> {
    return this.executeWithRetry(() => this.client.get('/collections'));
  }

  async getCollection(name: string): Promise<VexFSCollection> {
    return this.executeWithRetry(() => this.client.get(`/collections/${name}`));
  }

  async createCollection(
    name: string,
    vectorSize: number,
    distance: 'cosine' | 'euclidean' | 'dot'
  ): Promise<VexFSCollection> {
    return this.executeWithRetry(() =>
      this.client.post('/collections', { name, vectorSize, distance })
    );
  }

  async updateCollection(
    name: string,
    updates: Partial<VexFSCollection>
  ): Promise<VexFSCollection> {
    return this.executeWithRetry(() =>
      this.client.patch(`/collections/${name}`, updates)
    );
  }

  async deleteCollection(name: string): Promise<void> {
    return this.executeWithRetry(() => this.client.delete(`/collections/${name}`));
  }

  // Vectors API
  async getVectors(
    collectionName: string,
    limit?: number,
    offset?: number
  ): Promise<VexFSPoint[]> {
    const params = new URLSearchParams();
    if (limit) params.append('limit', limit.toString());
    if (offset) params.append('offset', offset.toString());

    return this.executeWithRetry(() =>
      this.client.get(`/collections/${collectionName}/vectors?${params}`)
    );
  }

  async addVector(
    collectionName: string,
    vector: number[],
    metadata?: Record<string, any>
  ): Promise<VexFSPoint> {
    return this.executeWithRetry(() =>
      this.client.post(`/collections/${collectionName}/vectors`, {
        vector,
        metadata,
      })
    );
  }

  async addVectors(
    collectionName: string,
    vectors: Array<{ vector: number[]; metadata?: Record<string, any> }>
  ): Promise<VexFSPoint[]> {
    return this.executeWithRetry(() =>
      this.client.post(`/collections/${collectionName}/vectors/batch`, {
        vectors,
      })
    );
  }

  async updateVector(
    collectionName: string,
    vectorId: string,
    updates: { vector?: number[]; metadata?: Record<string, any> }
  ): Promise<VexFSPoint> {
    return this.executeWithRetry(() =>
      this.client.patch(`/collections/${collectionName}/vectors/${vectorId}`, updates)
    );
  }

  async deleteVector(collectionName: string, vectorId: string): Promise<void> {
    return this.executeWithRetry(() =>
      this.client.delete(`/collections/${collectionName}/vectors/${vectorId}`)
    );
  }

  // Search API
  async search(
    collectionName: string,
    query: SearchQuery
  ): Promise<SearchResultsResponse> {
    return this.executeWithRetry(() =>
      this.client.post(`/collections/${collectionName}/search`, query)
    );
  }

  async vectorSimilaritySearch(
    collectionName: string,
    vector: number[],
    limit: number = 10,
    threshold?: number
  ): Promise<SearchResultsResponse> {
    return this.executeWithRetry(() =>
      this.client.post(`/collections/${collectionName}/search/similarity`, {
        vector,
        limit,
        threshold,
      })
    );
  }

  async metadataSearch(
    collectionName: string,
    filters: Record<string, any>,
    limit: number = 10
  ): Promise<SearchResultsResponse> {
    return this.executeWithRetry(() =>
      this.client.post(`/collections/${collectionName}/search/metadata`, {
        filters,
        limit,
      })
    );
  }

  async hybridSearch(
    collectionName: string,
    vector: number[],
    filters: Record<string, any>,
    vectorWeight: number = 0.7,
    metadataWeight: number = 0.3,
    limit: number = 10
  ): Promise<SearchResultsResponse> {
    return this.executeWithRetry(() =>
      this.client.post(`/collections/${collectionName}/search/hybrid`, {
        vector,
        filters,
        vectorWeight,
        metadataWeight,
        limit,
      })
    );
  }

  // Health and monitoring
  async getHealth(): Promise<{ status: string; timestamp: string }> {
    return this.executeWithRetry(
      () => this.client.get('/health'),
      { maxRetries: 1, baseDelay: 500 } // Faster retry for health checks
    );
  }

  async getMetrics(): Promise<any> {
    return this.executeWithRetry(() => this.client.get('/metrics'));
  }

  // Utility methods
  setAuthToken(token: string): void {
    this.client.defaults.headers.common['Authorization'] = `Bearer ${token}`;
  }

  clearAuthToken(): void {
    delete this.client.defaults.headers.common['Authorization'];
  }

  updateTimeout(timeout: number): void {
    this.client.defaults.timeout = timeout;
  }

  updateRetryConfig(config: Partial<RetryConfig>): void {
    this.retryConfig = { ...this.retryConfig, ...config };
  }
}

// Create and export singleton instance
export const enhancedVexfsApi = new EnhancedVexFSApi();
export default enhancedVexfsApi;