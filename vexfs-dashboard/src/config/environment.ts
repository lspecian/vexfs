/**
 * Environment Configuration
 * Manages environment-specific settings and variables
 */

export interface EnvironmentConfig {
  apiBaseUrl: string;
  apiTimeout: number;
  enableMockData: boolean;
  enableLogging: boolean;
  retryAttempts: number;
  retryDelay: number;
  authTokenKey: string;
  refreshTokenKey: string;
  tokenRefreshThreshold: number; // minutes before expiry to refresh
}

const getEnvironmentConfig = (): EnvironmentConfig => {
  // Default configuration
  const defaultConfig: EnvironmentConfig = {
    apiBaseUrl: 'http://localhost:7680',
    apiTimeout: 10000,
    enableMockData: false, // Disable mock data by default - use real API
    enableLogging: true,
    retryAttempts: 3,
    retryDelay: 1000,
    authTokenKey: 'vexfs_auth_token',
    refreshTokenKey: 'vexfs_refresh_token',
    tokenRefreshThreshold: 5, // Refresh token 5 minutes before expiry
  };

  // Override with environment variables if available
  return {
    ...defaultConfig,
    apiBaseUrl: import.meta.env.VITE_API_BASE_URL || defaultConfig.apiBaseUrl,
    apiTimeout: parseInt(
      import.meta.env.VITE_API_TIMEOUT || String(defaultConfig.apiTimeout)
    ),
    enableMockData:
      import.meta.env.VITE_ENABLE_MOCK_DATA === 'true' ||
      import.meta.env.VITE_ENABLE_MOCK_DATA === undefined
        ? defaultConfig.enableMockData
        : false,
    enableLogging: import.meta.env.VITE_ENABLE_LOGGING !== 'false',
    retryAttempts: parseInt(
      import.meta.env.VITE_RETRY_ATTEMPTS || String(defaultConfig.retryAttempts)
    ),
    retryDelay: parseInt(
      import.meta.env.VITE_RETRY_DELAY || String(defaultConfig.retryDelay)
    ),
    authTokenKey:
      import.meta.env.VITE_AUTH_TOKEN_KEY || defaultConfig.authTokenKey,
    refreshTokenKey:
      import.meta.env.VITE_REFRESH_TOKEN_KEY || defaultConfig.refreshTokenKey,
    tokenRefreshThreshold: parseInt(
      import.meta.env.VITE_TOKEN_REFRESH_THRESHOLD ||
        String(defaultConfig.tokenRefreshThreshold)
    ),
  };
};

export const environment = getEnvironmentConfig();

// Environment detection utilities
export const isDevelopment = import.meta.env.DEV;
export const isProduction = import.meta.env.PROD;

// API endpoint helpers
export const getApiUrl = (endpoint: string): string => {
  const baseUrl = environment.apiBaseUrl.replace(/\/$/, ''); // Remove trailing slash
  const cleanEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;
  return `${baseUrl}${cleanEndpoint}`;
};

// Logging utility
export const log = {
  debug: (...args: unknown[]) => {
    if (environment.enableLogging && isDevelopment) {
      console.debug('[VexFS Debug]', ...args);
    }
  },
  info: (...args: unknown[]) => {
    if (environment.enableLogging) {
      console.info('[VexFS Info]', ...args);
    }
  },
  warn: (...args: unknown[]) => {
    if (environment.enableLogging) {
      console.warn('[VexFS Warning]', ...args);
    }
  },
  error: (...args: unknown[]) => {
    if (environment.enableLogging) {
      console.error('[VexFS Error]', ...args);
    }
  },
};
