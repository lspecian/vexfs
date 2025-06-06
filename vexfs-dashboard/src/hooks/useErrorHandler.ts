/**
 * Error Handler Hook
 * Provides global error handling functionality
 */

import { useState, useCallback } from 'react';
import { log } from '../config/environment';
import { VexFSApiError } from '../config/api';

export interface ErrorState {
  error: string | null;
  isVisible: boolean;
  severity: 'error' | 'warning' | 'info';
  retryAction?: () => void;
}

export interface UseErrorHandlerReturn {
  error: ErrorState;
  showError: (
    error: string | Error,
    severity?: 'error' | 'warning' | 'info',
    retryAction?: () => void
  ) => void;
  clearError: () => void;
  handleApiError: (error: unknown, retryAction?: () => void) => void;
}

export const useErrorHandler = (): UseErrorHandlerReturn => {
  const [error, setError] = useState<ErrorState>({
    error: null,
    isVisible: false,
    severity: 'error',
  });

  const showError = useCallback(
    (
      errorInput: string | Error,
      severity: 'error' | 'warning' | 'info' = 'error',
      retryAction?: () => void
    ) => {
      const errorMessage =
        typeof errorInput === 'string' ? errorInput : errorInput.message;

      log.error('Error displayed to user:', errorMessage);

      setError({
        error: errorMessage,
        isVisible: true,
        severity,
        retryAction,
      });
    },
    []
  );

  const clearError = useCallback(() => {
    setError({
      error: null,
      isVisible: false,
      severity: 'error',
    });
  }, []);

  const handleApiError = useCallback(
    (errorInput: unknown, retryAction?: () => void) => {
      let errorMessage = 'An unexpected error occurred';
      let severity: 'error' | 'warning' | 'info' = 'error';

      if (errorInput instanceof VexFSApiError) {
        errorMessage = errorInput.message;

        // Determine severity based on status code
        if (errorInput.status) {
          if (errorInput.status >= 400 && errorInput.status < 500) {
            severity = 'warning'; // Client errors
          } else if (errorInput.status >= 500) {
            severity = 'error'; // Server errors
          }
        }
      } else if (errorInput instanceof Error) {
        errorMessage = errorInput.message;
      } else if (typeof errorInput === 'string') {
        errorMessage = errorInput;
      }

      // Handle specific error types
      if (errorMessage.toLowerCase().includes('network')) {
        errorMessage =
          'Network connection failed. Please check your internet connection and try again.';
        severity = 'warning';
      } else if (errorMessage.toLowerCase().includes('timeout')) {
        errorMessage =
          'Request timed out. The server may be busy. Please try again.';
        severity = 'warning';
      } else if (errorMessage.toLowerCase().includes('unauthorized')) {
        errorMessage =
          'You are not authorized to perform this action. Please log in again.';
        severity = 'warning';
      } else if (errorMessage.toLowerCase().includes('forbidden')) {
        errorMessage = 'You do not have permission to access this resource.';
        severity = 'warning';
      } else if (errorMessage.toLowerCase().includes('not found')) {
        errorMessage = 'The requested resource was not found.';
        severity = 'warning';
      }

      log.error('API error handled:', errorInput);

      setError({
        error: errorMessage,
        isVisible: true,
        severity,
        retryAction,
      });
    },
    []
  );

  return {
    error,
    showError,
    clearError,
    handleApiError,
  };
};

export default useErrorHandler;
