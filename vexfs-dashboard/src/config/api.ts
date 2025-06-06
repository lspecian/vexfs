/**
 * API Configuration
 * Defines API endpoints, timeouts, and request configurations
 */

import { environment } from './environment';

export interface ApiEndpoints {
  // Authentication
  auth: {
    login: string;
    logout: string;
    refresh: string;
    profile: string;
  };

  // Collections
  collections: {
    list: string;
    create: string;
    get: (name: string) => string;
    update: (name: string) => string;
    delete: (name: string) => string;
    schema: (name: string) => string;
  };

  // Points/Vectors
  points: {
    list: (collectionName: string) => string;
    upsert: (collectionName: string) => string;
    delete: (collectionName: string) => string;
    get: (collectionName: string, pointId: string | number) => string;
    update: (collectionName: string, pointId: string | number) => string;
    search: (collectionName: string) => string;
    suggestions: (collectionName: string) => string;
    textToVector: (collectionName: string) => string;
  };

  // Search
  search: {
    advanced: string;
    history: string;
    saved: string;
    analytics: string;
    export: (searchId: string) => string;
  };

  // Monitoring
  monitoring: {
    system: string;
    performance: string;
    health: string;
    alerts: string;
    acknowledgeAlert: (alertId: string) => string;
    metricsHistory: string;
  };

  // General
  stats: string;
  health: string;
}

export const apiEndpoints: ApiEndpoints = {
  auth: {
    login: '/api/auth/login',
    logout: '/api/auth/logout',
    refresh: '/api/auth/refresh',
    profile: '/api/auth/profile',
  },

  collections: {
    list: '/api/v1/collections',
    create: '/api/v1/collections',
    get: (name: string) => `/api/v1/collections/${encodeURIComponent(name)}`,
    update: (name: string) => `/api/v1/collections/${encodeURIComponent(name)}`,
    delete: (name: string) => `/api/v1/collections/${encodeURIComponent(name)}`,
    schema: (name: string) =>
      `/api/v1/collections/${encodeURIComponent(name)}/schema`,
  },

  points: {
    list: (collectionName: string) =>
      `/api/v1/collections/${encodeURIComponent(collectionName)}/vectors`,
    upsert: (collectionName: string) =>
      `/api/v1/collections/${encodeURIComponent(collectionName)}/vectors`,
    delete: (collectionName: string) =>
      `/api/v1/collections/${encodeURIComponent(collectionName)}/vectors/delete`,
    get: (collectionName: string, pointId: string | number) =>
      `/api/v1/collections/${encodeURIComponent(collectionName)}/vectors/${encodeURIComponent(String(pointId))}`,
    update: (collectionName: string, pointId: string | number) =>
      `/api/v1/collections/${encodeURIComponent(collectionName)}/vectors/${encodeURIComponent(String(pointId))}`,
    search: (collectionName: string) =>
      `/api/v1/collections/${encodeURIComponent(collectionName)}/search`,
    suggestions: (collectionName: string) =>
      `/api/v1/collections/${encodeURIComponent(collectionName)}/search/suggestions`,
    textToVector: (collectionName: string) =>
      `/api/v1/collections/${encodeURIComponent(collectionName)}/text-to-vector`,
  },

  search: {
    advanced: '/api/search/advanced',
    history: '/api/search/history',
    saved: '/api/search/saved',
    analytics: '/api/search/analytics',
    export: (searchId: string) =>
      `/api/search/export/${encodeURIComponent(searchId)}`,
  },

  monitoring: {
    system: '/api/monitoring/system',
    performance: '/api/monitoring/performance',
    health: '/api/monitoring/health',
    alerts: '/api/monitoring/alerts',
    acknowledgeAlert: (alertId: string) =>
      `/api/monitoring/alerts/${encodeURIComponent(alertId)}/acknowledge`,
    metricsHistory: '/api/monitoring/metrics/history',
  },

  stats: '/api/v1/stats',
  health: '/api/v1/version',
};

// Request configuration
export interface RequestConfig {
  timeout: number;
  retryAttempts: number;
  retryDelay: number;
  retryCondition: (error: unknown) => boolean;
}

export const defaultRequestConfig: RequestConfig = {
  timeout: environment.apiTimeout,
  retryAttempts: environment.retryAttempts,
  retryDelay: environment.retryDelay,
  retryCondition: (error: unknown) => {
    // Retry on network errors, timeouts, and 5xx server errors
    if (!error || typeof error !== 'object') return false;

    const axiosError = error as {
      code?: string;
      response?: { status?: number };
    };

    // Network errors
    if (axiosError.code === 'ECONNABORTED' || axiosError.code === 'ENOTFOUND') {
      return true;
    }

    // Server errors (5xx)
    if (
      axiosError.response?.status &&
      axiosError.response.status >= 500 &&
      axiosError.response.status < 600
    ) {
      return true;
    }

    return false;
  },
};

// HTTP status codes
export const HTTP_STATUS = {
  OK: 200,
  CREATED: 201,
  NO_CONTENT: 204,
  BAD_REQUEST: 400,
  UNAUTHORIZED: 401,
  FORBIDDEN: 403,
  NOT_FOUND: 404,
  CONFLICT: 409,
  UNPROCESSABLE_ENTITY: 422,
  TOO_MANY_REQUESTS: 429,
  INTERNAL_SERVER_ERROR: 500,
  BAD_GATEWAY: 502,
  SERVICE_UNAVAILABLE: 503,
  GATEWAY_TIMEOUT: 504,
} as const;

// Error types
export interface ApiError {
  message: string;
  code?: string;
  status?: number;
  details?: unknown;
}

export class VexFSApiError extends Error implements ApiError {
  public readonly code?: string;
  public readonly status?: number;
  public readonly details?: unknown;

  constructor(
    message: string,
    code?: string,
    status?: number,
    details?: unknown
  ) {
    super(message);
    this.name = 'VexFSApiError';
    this.code = code;
    this.status = status;
    this.details = details;
  }

  static fromAxiosError(error: unknown): VexFSApiError {
    if (!error || typeof error !== 'object') {
      return new VexFSApiError('Unknown error occurred');
    }

    const axiosError = error as {
      message?: string;
      code?: string;
      response?: {
        status?: number;
        data?: { message?: string; error?: string };
      };
    };

    const message =
      axiosError.response?.data?.message ||
      axiosError.response?.data?.error ||
      axiosError.message ||
      'API request failed';

    const status = axiosError.response?.status;
    const code = axiosError.code;

    return new VexFSApiError(message, code, status, axiosError.response?.data);
  }
}
