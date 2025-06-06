/**
 * Authentication Service
 * Handles user authentication, token management, and session persistence
 */

import axios, { type AxiosInstance } from 'axios';
import { environment, log } from '../config/environment';
import { apiEndpoints, VexFSApiError } from '../config/api';

export interface User {
  id: string;
  username: string;
  email?: string;
  roles: string[];
  permissions: string[];
  lastLogin?: string;
}

export interface LoginCredentials {
  username: string;
  password: string;
}

export interface AuthTokens {
  accessToken: string;
  refreshToken: string;
  expiresAt: number; // Unix timestamp
  tokenType: string;
}

export interface AuthResponse {
  user: User;
  tokens: AuthTokens;
}

export interface AuthState {
  isAuthenticated: boolean;
  user: User | null;
  tokens: AuthTokens | null;
  isLoading: boolean;
  error: string | null;
}

class AuthService {
  private api: AxiosInstance;
  private refreshPromise: Promise<AuthTokens> | null = null;

  constructor() {
    this.api = axios.create({
      baseURL: environment.apiBaseUrl,
      timeout: environment.apiTimeout,
      headers: {
        'Content-Type': 'application/json',
      },
    });

    // Setup interceptors
    this.setupInterceptors();
  }

  private setupInterceptors(): void {
    // Request interceptor to add auth token
    this.api.interceptors.request.use(
      config => {
        const tokens = this.getStoredTokens();
        if (tokens?.accessToken) {
          config.headers.Authorization = `${tokens.tokenType} ${tokens.accessToken}`;
        }
        return config;
      },
      error => Promise.reject(error)
    );

    // Response interceptor to handle token refresh
    this.api.interceptors.response.use(
      response => response,
      async error => {
        const originalRequest = error.config;

        if (
          error.response?.status === 401 &&
          !originalRequest._retry &&
          this.getStoredTokens()?.refreshToken
        ) {
          originalRequest._retry = true;

          try {
            const newTokens = await this.refreshAccessToken();
            originalRequest.headers.Authorization = `${newTokens.tokenType} ${newTokens.accessToken}`;
            return this.api(originalRequest);
          } catch (refreshError) {
            log.error('Token refresh failed:', refreshError);
            this.logout();
            return Promise.reject(refreshError);
          }
        }

        return Promise.reject(error);
      }
    );
  }

  /**
   * Login with username and password
   */
  async login(credentials: LoginCredentials): Promise<AuthResponse> {
    try {
      log.info('Attempting login for user:', credentials.username);

      const response = await this.api.post(
        apiEndpoints.auth.login,
        credentials
      );
      const authResponse: AuthResponse = response.data.data;

      // Store tokens
      this.storeTokens(authResponse.tokens);

      log.info('Login successful for user:', authResponse.user.username);
      return authResponse;
    } catch (error) {
      log.error('Login failed:', error);
      throw VexFSApiError.fromAxiosError(error);
    }
  }

  /**
   * Logout and clear stored tokens
   */
  async logout(): Promise<void> {
    try {
      const tokens = this.getStoredTokens();
      if (tokens?.accessToken) {
        await this.api.post(apiEndpoints.auth.logout);
      }
    } catch (error) {
      log.warn('Logout API call failed:', error);
      // Continue with local logout even if API call fails
    } finally {
      this.clearStoredTokens();
      log.info('User logged out');
    }
  }

  /**
   * Refresh access token using refresh token
   */
  async refreshAccessToken(): Promise<AuthTokens> {
    // Prevent multiple simultaneous refresh requests
    if (this.refreshPromise) {
      return this.refreshPromise;
    }

    this.refreshPromise = this.performTokenRefresh();

    try {
      const tokens = await this.refreshPromise;
      return tokens;
    } finally {
      this.refreshPromise = null;
    }
  }

  private async performTokenRefresh(): Promise<AuthTokens> {
    const storedTokens = this.getStoredTokens();
    if (!storedTokens?.refreshToken) {
      throw new VexFSApiError('No refresh token available');
    }

    try {
      const response = await this.api.post(apiEndpoints.auth.refresh, {
        refreshToken: storedTokens.refreshToken,
      });

      const newTokens: AuthTokens = response.data.data;
      this.storeTokens(newTokens);

      log.info('Token refreshed successfully');
      return newTokens;
    } catch (error) {
      log.error('Token refresh failed:', error);
      this.clearStoredTokens();
      throw VexFSApiError.fromAxiosError(error);
    }
  }

  /**
   * Get current user profile
   */
  async getCurrentUser(): Promise<User> {
    try {
      const response = await this.api.get(apiEndpoints.auth.profile);
      return response.data.data;
    } catch (error) {
      log.error('Failed to get current user:', error);
      throw VexFSApiError.fromAxiosError(error);
    }
  }

  /**
   * Check if user is authenticated
   */
  isAuthenticated(): boolean {
    const tokens = this.getStoredTokens();
    if (!tokens?.accessToken) {
      return false;
    }

    // Check if token is expired
    const now = Date.now();
    if (tokens.expiresAt <= now) {
      log.debug('Access token expired');
      return false;
    }

    return true;
  }

  /**
   * Check if token needs refresh
   */
  shouldRefreshToken(): boolean {
    const tokens = this.getStoredTokens();
    if (!tokens?.accessToken || !tokens?.refreshToken) {
      return false;
    }

    const now = Date.now();
    const refreshThreshold = environment.tokenRefreshThreshold * 60 * 1000; // Convert to milliseconds
    const shouldRefresh = tokens.expiresAt - now <= refreshThreshold;

    if (shouldRefresh) {
      log.debug('Token should be refreshed');
    }

    return shouldRefresh;
  }

  /**
   * Get stored authentication tokens
   */
  getStoredTokens(): AuthTokens | null {
    try {
      const accessToken = localStorage.getItem(environment.authTokenKey);
      const refreshToken = localStorage.getItem(environment.refreshTokenKey);

      if (!accessToken || !refreshToken) {
        return null;
      }

      // Parse token to get expiration
      const tokenPayload = this.parseJwtPayload(accessToken);
      const expiresAt = tokenPayload?.exp ? tokenPayload.exp * 1000 : 0;

      return {
        accessToken,
        refreshToken,
        expiresAt,
        tokenType: 'Bearer',
      };
    } catch (error) {
      log.error('Failed to get stored tokens:', error);
      return null;
    }
  }

  /**
   * Store authentication tokens
   */
  private storeTokens(tokens: AuthTokens): void {
    try {
      localStorage.setItem(environment.authTokenKey, tokens.accessToken);
      localStorage.setItem(environment.refreshTokenKey, tokens.refreshToken);
      log.debug('Tokens stored successfully');
    } catch (error) {
      log.error('Failed to store tokens:', error);
      throw new VexFSApiError('Failed to store authentication tokens');
    }
  }

  /**
   * Clear stored authentication tokens
   */
  private clearStoredTokens(): void {
    try {
      localStorage.removeItem(environment.authTokenKey);
      localStorage.removeItem(environment.refreshTokenKey);
      log.debug('Tokens cleared successfully');
    } catch (error) {
      log.error('Failed to clear tokens:', error);
    }
  }

  /**
   * Parse JWT payload without verification
   */
  private parseJwtPayload(token: string): Record<string, unknown> | null {
    try {
      const parts = token.split('.');
      if (parts.length !== 3) {
        return null;
      }

      const payload = parts[1];
      const decoded = atob(payload.replace(/-/g, '+').replace(/_/g, '/'));
      return JSON.parse(decoded);
    } catch (error) {
      log.error('Failed to parse JWT payload:', error);
      return null;
    }
  }

  /**
   * Get axios instance for authenticated requests
   */
  getAuthenticatedApi(): AxiosInstance {
    return this.api;
  }
}

// Export singleton instance
export const authService = new AuthService();
export default AuthService;
