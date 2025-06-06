/**
 * Enhanced Error Handler Hook
 * Provides comprehensive error handling with retry logic, logging, and user feedback
 */

import { useCallback, useRef, useState } from 'react';
import { useSnackbar } from 'notistack';

interface ErrorContext {
  component?: string;
  action?: string;
  userId?: string;
  timestamp: number;
  userAgent: string;
  url: string;
}

interface RetryConfig {
  maxRetries: number;
  retryDelay: number;
  backoffMultiplier: number;
  retryCondition?: (error: Error) => boolean;
}

interface ErrorHandlerOptions {
  enableRetry?: boolean;
  retryConfig?: Partial<RetryConfig>;
  enableLogging?: boolean;
  enableUserNotification?: boolean;
  logToService?: boolean;
}

const DEFAULT_RETRY_CONFIG: RetryConfig = {
  maxRetries: 3,
  retryDelay: 1000,
  backoffMultiplier: 2,
  retryCondition: (error: Error) => {
    // Retry on network errors, timeouts, and 5xx server errors
    return (
      error.name === 'NetworkError' ||
      error.name === 'TimeoutError' ||
      Boolean(error.message && error.message.includes('500'))
    );
  },
};

export const useEnhancedErrorHandler = (options: ErrorHandlerOptions = {}) => {
  const {
    enableRetry = true,
    retryConfig = {},
    enableLogging = true,
    enableUserNotification = true,
    logToService = false,
  } = options;

  const { enqueueSnackbar } = useSnackbar();
  const retryAttempts = useRef<Map<string, number>>(new Map());
  const [isRetrying, setIsRetrying] = useState<boolean>(false);

  const finalRetryConfig = { ...DEFAULT_RETRY_CONFIG, ...retryConfig };

  // Create error context
  const createErrorContext = useCallback(
    (component?: string, action?: string): ErrorContext => {
      return {
        component,
        action,
        timestamp: Date.now(),
        userAgent: navigator.userAgent,
        url: window.location.href,
      };
    },
    []
  );

  // Log error to console and external service
  const logError = useCallback(
    async (error: Error, context: ErrorContext) => {
      if (!enableLogging) return;

      const errorData = {
        message: error.message,
        stack: error.stack,
        name: error.name,
        context,
      };

      // Log to console in development
      if (import.meta.env.DEV) {
        console.group('🚨 Error Handler');
        console.error('Error:', error);
        console.log('Context:', context);
        console.groupEnd();
      }

      // Log to external service in production
      if (logToService && !import.meta.env.DEV) {
        try {
          // Replace with your error logging service
          await fetch('/api/errors', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(errorData),
          });
        } catch (loggingError) {
          console.error('Failed to log error to service:', loggingError);
        }
      }
    },
    [enableLogging, logToService]
  );

  // Show user notification
  const showUserNotification = useCallback(
    (error: Error, isRetrying: boolean = false) => {
      if (!enableUserNotification) return;

      const message = isRetrying
        ? 'Retrying operation...'
        : `Error: ${error.message || 'Something went wrong'}`;

      const variant = isRetrying ? 'info' : 'error';

      enqueueSnackbar(message, {
        variant,
        autoHideDuration: isRetrying ? 2000 : 5000,
        preventDuplicate: true,
      });
    },
    [enableUserNotification, enqueueSnackbar]
  );

  // Execute with retry logic
  const executeWithRetry = useCallback(
    async <T>(
      operation: () => Promise<T>,
      operationId: string,
      context: ErrorContext
    ): Promise<T> => {
      const currentAttempts = retryAttempts.current.get(operationId) || 0;

      try {
        const result = await operation();
        // Reset retry count on success
        retryAttempts.current.delete(operationId);
        setIsRetrying(false);
        return result;
      } catch (error) {
        const err = error as Error;
        await logError(err, context);

        // Check if we should retry
        if (
          enableRetry &&
          currentAttempts < finalRetryConfig.maxRetries &&
          finalRetryConfig.retryCondition?.(err)
        ) {
          const nextAttempt = currentAttempts + 1;
          retryAttempts.current.set(operationId, nextAttempt);
          setIsRetrying(true);

          showUserNotification(err, true);

          // Calculate delay with exponential backoff
          const delay =
            finalRetryConfig.retryDelay *
            Math.pow(finalRetryConfig.backoffMultiplier, currentAttempts);

          // Wait before retry
          await new Promise(resolve => setTimeout(resolve, delay));

          // Recursive retry
          return executeWithRetry(operation, operationId, context);
        } else {
          // Max retries reached or error not retryable
          retryAttempts.current.delete(operationId);
          setIsRetrying(false);
          showUserNotification(err, false);
          throw err;
        }
      }
    },
    [enableRetry, finalRetryConfig, logError, showUserNotification]
  );

  // Main error handler function
  const handleError = useCallback(
    async <T>(
      operation: () => Promise<T>,
      component?: string,
      action?: string
    ): Promise<T | null> => {
      const context = createErrorContext(component, action);
      const operationId = `${component || 'unknown'}-${action || 'unknown'}-${Date.now()}`;

      try {
        return await executeWithRetry(operation, operationId, context);
      } catch (error) {
        // Final error handling - operation failed after all retries
        console.error('Operation failed after all retries:', error);
        return null;
      }
    },
    [createErrorContext, executeWithRetry]
  );

  // Synchronous error handler
  const handleSyncError = useCallback(
    (error: Error, component?: string, action?: string) => {
      const context = createErrorContext(component, action);
      logError(error, context);
      showUserNotification(error, false);
    },
    [createErrorContext, logError, showUserNotification]
  );

  // Clear retry attempts for a specific operation
  const clearRetryAttempts = useCallback((operationId: string) => {
    retryAttempts.current.delete(operationId);
  }, []);

  // Get retry status
  const getRetryStatus = useCallback(
    (operationId: string) => {
      return {
        attempts: retryAttempts.current.get(operationId) || 0,
        maxRetries: finalRetryConfig.maxRetries,
        isRetrying,
      };
    },
    [finalRetryConfig.maxRetries, isRetrying]
  );

  return {
    handleError,
    handleSyncError,
    clearRetryAttempts,
    getRetryStatus,
    isRetrying,
  };
};

// Error boundary hook for class components
export const useErrorBoundaryHandler = () => {
  const { handleSyncError } = useEnhancedErrorHandler({
    enableRetry: false,
    enableUserNotification: true,
    enableLogging: true,
  });

  return {
    onError: (error: Error, errorInfo: { componentStack: string }) => {
      handleSyncError(error, 'ErrorBoundary', 'componentDidCatch');

      // Additional error boundary specific logging
      if (import.meta.env.DEV) {
        console.group('🛡️ Error Boundary Caught Error');
        console.error('Error:', error);
        console.log('Component Stack:', errorInfo.componentStack);
        console.groupEnd();
      }
    },
  };
};