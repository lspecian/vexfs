/**
 * Authentication Hook
 * Provides authentication state and methods to React components
 */

import { useState, useEffect, useCallback } from 'react';
import {
  authService,
  type User,
  type LoginCredentials,
  type AuthState,
} from '../services/auth';
import { log } from '../config/environment';

export interface UseAuthReturn extends AuthState {
  login: (credentials: LoginCredentials) => Promise<void>;
  logout: () => Promise<void>;
  refreshToken: () => Promise<void>;
  getCurrentUser: () => Promise<User | null>;
}

export const useAuth = (): UseAuthReturn => {
  const [state, setState] = useState<AuthState>({
    isAuthenticated: false,
    user: null,
    tokens: null,
    isLoading: true,
    error: null,
  });

  // Initialize authentication state
  useEffect(() => {
    const initializeAuth = async () => {
      try {
        setState(prev => ({ ...prev, isLoading: true, error: null }));

        const isAuthenticated = authService.isAuthenticated();

        if (isAuthenticated) {
          // Check if token needs refresh
          if (authService.shouldRefreshToken()) {
            try {
              await authService.refreshAccessToken();
            } catch (error) {
              log.warn('Token refresh failed during initialization:', error);
              // Continue with existing token if refresh fails
            }
          }

          // Get current user
          try {
            const user = await authService.getCurrentUser();
            const tokens = authService.getStoredTokens();

            setState({
              isAuthenticated: true,
              user,
              tokens,
              isLoading: false,
              error: null,
            });
          } catch (error) {
            log.error('Failed to get current user:', error);
            setState({
              isAuthenticated: false,
              user: null,
              tokens: null,
              isLoading: false,
              error: 'Failed to load user profile',
            });
          }
        } else {
          setState({
            isAuthenticated: false,
            user: null,
            tokens: null,
            isLoading: false,
            error: null,
          });
        }
      } catch (error) {
        log.error('Auth initialization failed:', error);
        setState({
          isAuthenticated: false,
          user: null,
          tokens: null,
          isLoading: false,
          error: 'Authentication initialization failed',
        });
      }
    };

    initializeAuth();
  }, []);

  // Login function
  const login = useCallback(async (credentials: LoginCredentials) => {
    try {
      setState(prev => ({ ...prev, isLoading: true, error: null }));

      const authResponse = await authService.login(credentials);

      setState({
        isAuthenticated: true,
        user: authResponse.user,
        tokens: authResponse.tokens,
        isLoading: false,
        error: null,
      });

      log.info('User logged in successfully');
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : 'Login failed';
      log.error('Login failed:', error);

      setState(prev => ({
        ...prev,
        isLoading: false,
        error: errorMessage,
      }));

      throw error;
    }
  }, []);

  // Logout function
  const logout = useCallback(async () => {
    try {
      setState(prev => ({ ...prev, isLoading: true, error: null }));

      await authService.logout();

      setState({
        isAuthenticated: false,
        user: null,
        tokens: null,
        isLoading: false,
        error: null,
      });

      log.info('User logged out successfully');
    } catch (error) {
      log.error('Logout failed:', error);

      // Even if logout fails, clear local state
      setState({
        isAuthenticated: false,
        user: null,
        tokens: null,
        isLoading: false,
        error: null,
      });
    }
  }, []);

  // Refresh token function
  const refreshToken = useCallback(async () => {
    try {
      const newTokens = await authService.refreshAccessToken();

      setState(prev => ({
        ...prev,
        tokens: newTokens,
        error: null,
      }));

      log.info('Token refreshed successfully');
    } catch (error) {
      log.error('Token refresh failed:', error);

      // If refresh fails, logout user
      setState({
        isAuthenticated: false,
        user: null,
        tokens: null,
        isLoading: false,
        error: 'Session expired. Please log in again.',
      });

      throw error;
    }
  }, []);

  // Get current user function
  const getCurrentUser = useCallback(async (): Promise<User | null> => {
    try {
      if (!state.isAuthenticated) {
        return null;
      }

      const user = await authService.getCurrentUser();

      setState(prev => ({
        ...prev,
        user,
        error: null,
      }));

      return user;
    } catch (error) {
      log.error('Failed to get current user:', error);

      setState(prev => ({
        ...prev,
        error: 'Failed to load user profile',
      }));

      return null;
    }
  }, [state.isAuthenticated]);

  return {
    ...state,
    login,
    logout,
    refreshToken,
    getCurrentUser,
  };
};

export default useAuth;
