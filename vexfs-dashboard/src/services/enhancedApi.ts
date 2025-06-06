/**
 * Enhanced VexFS API Service
 * Integrates authentication, error handling, and retry logic
 */

import axios, {
  type AxiosInstance,
  type AxiosResponse,
  type AxiosRequestConfig,
} from 'axios';
import { environment, log } from '../config/environment';
import {
  apiEndpoints,
  defaultRequestConfig,
  VexFSApiError,
  HTTP_STATUS,
} from '../config/api';
import { authService } from './auth';

// Re-export types from the original API service
export type {
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

export type {
  SystemMetrics,
  PerformanceMetrics,
  HealthStatus,
  Alert,
  MonitoringApiResponse,
  MetricsHistoryResponse,
} from '../types/monitoring';

class EnhancedVexFSApiService {
  private api: AxiosInstance;
  private retryCount = new Map<string, number>();

  constructor() {
    this.api = axios.create({
      baseURL: environment.apiBaseUrl,
      timeout: defaultRequestConfig.timeout,
      headers: {
        'Content-Type': 'application/json',
      },
    });

    this.setupInterceptors();
  }

  private setupInterceptors(): void {
    // Request interceptor
    this.api.interceptors.request.use(
      config => {
        // Add authentication token
        const tokens = authService.getStoredTokens();
        if (tokens?.accessToken) {
          config.headers.Authorization = `${tokens.tokenType} ${tokens.accessToken}`;
        }

        // Log request
        if (environment.enableLogging) {
          log.debug(
            `API Request: ${config.method?.toUpperCase()} ${config.url}`,
            {
              params: config.params,
              data: config.data,
            }
          );
        }

        return config;
      },
      error => {
        log.error('Request interceptor error:', error);
        return Promise.reject(VexFSApiError.fromAxiosError(error));
      }
    );

    // Response interceptor
    this.api.interceptors.response.use(
      response => {
        // Log successful response
        if (environment.enableLogging) {
          log.debug(`API Response: ${response.status} ${response.config.url}`, {
            data: response.data,
          });
        }

        // Clear retry count on success
        const requestKey = this.getRequestKey(response.config);
        this.retryCount.delete(requestKey);

        return response;
      },
      async error => {
        const originalRequest = error.config;
        const requestKey = this.getRequestKey(originalRequest);

        // Handle authentication errors
        if (
          error.response?.status === HTTP_STATUS.UNAUTHORIZED &&
          !originalRequest._retry
        ) {
          originalRequest._retry = true;

          try {
            await authService.refreshAccessToken();
            const tokens = authService.getStoredTokens();
            if (tokens?.accessToken) {
              originalRequest.headers.Authorization = `${tokens.tokenType} ${tokens.accessToken}`;
              return this.api(originalRequest);
            }
          } catch (refreshError) {
            log.error('Token refresh failed:', refreshError);
            // Redirect to login or handle auth failure
            return Promise.reject(VexFSApiError.fromAxiosError(error));
          }
        }

        // Handle retry logic
        const currentRetryCount = this.retryCount.get(requestKey) || 0;
        const shouldRetry =
          currentRetryCount < defaultRequestConfig.retryAttempts &&
          defaultRequestConfig.retryCondition(error) &&
          !originalRequest._retry;

        if (shouldRetry) {
          this.retryCount.set(requestKey, currentRetryCount + 1);

          // Exponential backoff
          const delay =
            defaultRequestConfig.retryDelay * Math.pow(2, currentRetryCount);

          log.warn(
            `Retrying request (${currentRetryCount + 1}/${defaultRequestConfig.retryAttempts}) after ${delay}ms:`,
            {
              url: originalRequest.url,
              error: error.message,
            }
          );

          await this.delay(delay);
          return this.api(originalRequest);
        }

        // Clear retry count on final failure
        this.retryCount.delete(requestKey);

        // Log error
        log.error('API Error:', {
          url: originalRequest.url,
          status: error.response?.status,
          message: error.message,
          data: error.response?.data,
        });

        return Promise.reject(VexFSApiError.fromAxiosError(error));
      }
    );
  }

  private getRequestKey(config: AxiosRequestConfig): string {
    return `${config.method}:${config.url}:${JSON.stringify(config.params)}`;
  }

  private delay(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  /**
   * Make a request with fallback to mock data
   */
  private async requestWithFallback<T>(
    requestFn: () => Promise<AxiosResponse<T>>,
    mockDataFn?: () => T
  ): Promise<T> {
    try {
      const response = await requestFn();
      return response.data;
    } catch (error) {
      if (environment.enableMockData && mockDataFn) {
        log.warn('API request failed, using mock data:', error);
        return mockDataFn();
      }
      throw error;
    }
  }

  // Health Check
  async healthCheck(): Promise<boolean> {
    try {
      // Use collections endpoint since /health hangs on Qdrant
      await this.api.get(apiEndpoints.collections.list);
      return true;
    } catch {
      return false;
    }
  }

  // Collections API with enhanced error handling
  async getCollections(): Promise<import('../types').VexFSCollection[]> {
    return this.requestWithFallback(
      () => this.api.get(apiEndpoints.collections.list),
      () => []
    );
  }

  async getCollection(
    name: string
  ): Promise<import('../types').VexFSCollection | null> {
    try {
      const response = await this.api.get(apiEndpoints.collections.get(name));
      return response.data.data || null;
    } catch (error) {
      if (
        error instanceof VexFSApiError &&
        error.status === HTTP_STATUS.NOT_FOUND
      ) {
        return null;
      }
      throw error;
    }
  }

  async createCollection(
    name: string,
    vectorSize: number,
    distance: 'cosine' | 'euclidean' | 'dot' = 'cosine'
  ): Promise<import('../types').VexFSCollection> {
    const response = await this.api.post(apiEndpoints.collections.create, {
      name,
      vectors: {
        size: vectorSize,
        distance,
      },
    });
    return response.data.data;
  }

  async deleteCollection(name: string): Promise<boolean> {
    try {
      await this.api.delete(apiEndpoints.collections.delete(name));
      return true;
    } catch (error) {
      if (
        error instanceof VexFSApiError &&
        error.status === HTTP_STATUS.NOT_FOUND
      ) {
        return true; // Already deleted
      }
      return false;
    }
  }

  // Get the original API instance for backward compatibility
  getApi(): AxiosInstance {
    return this.api;
  }

  // Get authenticated API instance
  getAuthenticatedApi(): AxiosInstance {
    return authService.getAuthenticatedApi();
  }
}

// Export enhanced singleton instance
export const enhancedVexfsApi = new EnhancedVexFSApiService();
export default EnhancedVexFSApiService;
