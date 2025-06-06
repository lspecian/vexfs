import { useState, useEffect, useCallback } from 'react';

interface NetworkStatus {
  isOnline: boolean;
  isSlowConnection: boolean;
  connectionType: string;
  effectiveType: string;
  downlink: number;
  rtt: number;
}

interface UseNetworkStatusOptions {
  onOnline?: () => void;
  onOffline?: () => void;
  onSlowConnection?: () => void;
  slowConnectionThreshold?: number; // RTT threshold in ms
}

/**
 * Hook to monitor network connectivity and connection quality
 */
export const useNetworkStatus = (options: UseNetworkStatusOptions = {}) => {
  const {
    onOnline,
    onOffline,
    onSlowConnection,
    slowConnectionThreshold = 1000,
  } = options;

  const [networkStatus, setNetworkStatus] = useState<NetworkStatus>(() => {
    // Get initial network information
    const navigator = window.navigator as any;
    const connection = navigator.connection || navigator.mozConnection || navigator.webkitConnection;
    
    return {
      isOnline: navigator.onLine,
      isSlowConnection: false,
      connectionType: connection?.type || 'unknown',
      effectiveType: connection?.effectiveType || 'unknown',
      downlink: connection?.downlink || 0,
      rtt: connection?.rtt || 0,
    };
  });

  // Check if connection is slow based on RTT
  const checkSlowConnection = useCallback((rtt: number) => {
    return rtt > slowConnectionThreshold;
  }, [slowConnectionThreshold]);

  // Update network status
  const updateNetworkStatus = useCallback(() => {
    const navigator = window.navigator as any;
    const connection = navigator.connection || navigator.mozConnection || navigator.webkitConnection;
    
    const isOnline = navigator.onLine;
    const rtt = connection?.rtt || 0;
    const isSlowConnection = checkSlowConnection(rtt);

    const newStatus: NetworkStatus = {
      isOnline,
      isSlowConnection,
      connectionType: connection?.type || 'unknown',
      effectiveType: connection?.effectiveType || 'unknown',
      downlink: connection?.downlink || 0,
      rtt,
    };

    setNetworkStatus(prevStatus => {
      // Trigger callbacks on status changes
      if (prevStatus.isOnline !== isOnline) {
        if (isOnline) {
          onOnline?.();
        } else {
          onOffline?.();
        }
      }

      if (!prevStatus.isSlowConnection && isSlowConnection) {
        onSlowConnection?.();
      }

      return newStatus;
    });
  }, [checkSlowConnection, onOnline, onOffline, onSlowConnection]);

  // Test network connectivity by making a request
  const testConnectivity = useCallback(async (): Promise<boolean> => {
    try {
      const response = await fetch('/api/health', {
        method: 'HEAD',
        cache: 'no-cache',
      });
      return response.ok;
    } catch {
      return false;
    }
  }, []);

  // Retry a failed operation with exponential backoff
  const retryWithBackoff = useCallback(
    async <T>(
      operation: () => Promise<T>,
      maxRetries = 3,
      baseDelay = 1000
    ): Promise<T> => {
      let lastError: Error;

      for (let attempt = 0; attempt <= maxRetries; attempt++) {
        try {
          return await operation();
        } catch (error) {
          lastError = error as Error;

          if (attempt === maxRetries) {
            throw lastError;
          }

          // Check if we're still online before retrying
          const isConnected = await testConnectivity();
          if (!isConnected) {
            throw new Error('Network connection lost');
          }

          // Exponential backoff
          const delay = baseDelay * Math.pow(2, attempt);
          await new Promise(resolve => setTimeout(resolve, delay));
        }
      }

      throw lastError!;
    },
    [testConnectivity]
  );

  // Setup event listeners
  useEffect(() => {
    const handleOnline = () => updateNetworkStatus();
    const handleOffline = () => updateNetworkStatus();
    const handleConnectionChange = () => updateNetworkStatus();

    // Listen for online/offline events
    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    // Listen for connection changes (if supported)
    const navigator = window.navigator as any;
    const connection = navigator.connection || navigator.mozConnection || navigator.webkitConnection;
    
    if (connection) {
      connection.addEventListener('change', handleConnectionChange);
    }

    // Initial status update
    updateNetworkStatus();

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
      
      if (connection) {
        connection.removeEventListener('change', handleConnectionChange);
      }
    };
  }, [updateNetworkStatus]);

  return {
    ...networkStatus,
    testConnectivity,
    retryWithBackoff,
  };
};

/**
 * Hook to automatically retry failed API calls when network is restored
 */
export const useNetworkRetry = () => {
  const [retryQueue, setRetryQueue] = useState<Array<() => Promise<any>>>([]);
  
  const { isOnline, retryWithBackoff } = useNetworkStatus({
    onOnline: () => {
      // Retry all queued operations when network is restored
      retryQueue.forEach(operation => {
        operation().catch(console.error);
      });
      setRetryQueue([]);
    },
  });

  const addToRetryQueue = useCallback((operation: () => Promise<any>) => {
    if (!isOnline) {
      setRetryQueue(prev => [...prev, operation]);
    }
  }, [isOnline]);

  const executeWithRetry = useCallback(
    async <T>(operation: () => Promise<T>): Promise<T> => {
      try {
        return await retryWithBackoff(operation);
      } catch (error) {
        if (!isOnline) {
          addToRetryQueue(operation);
        }
        throw error;
      }
    },
    [retryWithBackoff, isOnline, addToRetryQueue]
  );

  return {
    isOnline,
    executeWithRetry,
    queuedOperations: retryQueue.length,
  };
};

export default useNetworkStatus;