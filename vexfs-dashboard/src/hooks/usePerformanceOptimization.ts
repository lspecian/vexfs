/**
 * Performance Optimization Hook
 * Provides utilities for monitoring and optimizing component performance
 */

import { useCallback, useEffect, useRef, useState } from 'react';

interface PerformanceMetrics {
  renderTime: number;
  memoryUsage: number;
  componentMounts: number;
  rerenderCount: number;
}

interface UsePerformanceOptimizationOptions {
  enableProfiling?: boolean;
  enableMemoryMonitoring?: boolean;
  logThreshold?: number; // Log if render time exceeds this (ms)
}

export const usePerformanceOptimization = (
  componentName: string,
  options: UsePerformanceOptimizationOptions = {}
) => {
  const {
    enableProfiling = import.meta.env.DEV,
    enableMemoryMonitoring = true,
    logThreshold = 16, // 16ms = 60fps threshold
  } = options;

  const renderStartTime = useRef<number>(0);
  const mountCount = useRef<number>(0);
  const rerenderCount = useRef<number>(0);
  const [metrics, setMetrics] = useState<PerformanceMetrics>({
    renderTime: 0,
    memoryUsage: 0,
    componentMounts: 0,
    rerenderCount: 0,
  });

  // Start performance measurement
  const startMeasurement = useCallback(() => {
    if (enableProfiling) {
      renderStartTime.current = performance.now();
    }
  }, [enableProfiling]);

  // End performance measurement
  const endMeasurement = useCallback(() => {
    if (enableProfiling && renderStartTime.current > 0) {
      const renderTime = performance.now() - renderStartTime.current;
      rerenderCount.current += 1;

      // Get memory usage if available
      let memoryUsage = 0;
      if (enableMemoryMonitoring && 'memory' in performance) {
        const memory = (performance as any).memory;
        memoryUsage = memory.usedJSHeapSize / 1024 / 1024; // Convert to MB
      }

      const newMetrics: PerformanceMetrics = {
        renderTime,
        memoryUsage,
        componentMounts: mountCount.current,
        rerenderCount: rerenderCount.current,
      };

      setMetrics(newMetrics);

      // Log slow renders
      if (renderTime > logThreshold) {
        console.warn(
          `🐌 Slow render detected in ${componentName}:`,
          `${renderTime.toFixed(2)}ms`,
          newMetrics
        );
      }

      renderStartTime.current = 0;
    }
  }, [enableProfiling, enableMemoryMonitoring, logThreshold, componentName]);

  // Track component mounts
  useEffect(() => {
    mountCount.current += 1;
    if (enableProfiling) {
      console.log(`🚀 ${componentName} mounted (${mountCount.current} times)`);
    }

    return () => {
      if (enableProfiling) {
        console.log(`💀 ${componentName} unmounted`);
      }
    };
  }, [componentName, enableProfiling]);

  // Memory leak detection
  useEffect(() => {
    if (!enableMemoryMonitoring) return;

    const checkMemoryLeak = () => {
      if ('memory' in performance) {
        const memory = (performance as any).memory;
        const usedMB = memory.usedJSHeapSize / 1024 / 1024;
        const limitMB = memory.jsHeapSizeLimit / 1024 / 1024;

        if (usedMB > limitMB * 0.8) {
          console.warn(
            `⚠️ High memory usage detected in ${componentName}:`,
            `${usedMB.toFixed(2)}MB / ${limitMB.toFixed(2)}MB`
          );
        }
      }
    };

    const interval = setInterval(checkMemoryLeak, 10000); // Check every 10 seconds
    return () => clearInterval(interval);
  }, [componentName, enableMemoryMonitoring]);

  return {
    startMeasurement,
    endMeasurement,
    metrics,
    isSlowRender: metrics.renderTime > logThreshold,
  };
};

/**
 * Hook for debouncing values to prevent excessive re-renders
 */
export const useDebounce = <T>(value: T, delay: number): T => {
  const [debouncedValue, setDebouncedValue] = useState<T>(value);

  useEffect(() => {
    const handler = setTimeout(() => {
      setDebouncedValue(value);
    }, delay);

    return () => {
      clearTimeout(handler);
    };
  }, [value, delay]);

  return debouncedValue;
};

/**
 * Hook for throttling function calls
 */
export const useThrottle = <T extends (...args: any[]) => any>(
  callback: T,
  delay: number
): T => {
  const lastCall = useRef<number>(0);
  const timeoutRef = useRef<number | undefined>(undefined);

  return useCallback(
    ((...args: Parameters<T>) => {
      const now = Date.now();

      if (now - lastCall.current >= delay) {
        lastCall.current = now;
        return callback(...args);
      } else {
        if (timeoutRef.current) {
          clearTimeout(timeoutRef.current);
        }

        timeoutRef.current = window.setTimeout(
          () => {
            lastCall.current = Date.now();
            callback(...args);
          },
          delay - (now - lastCall.current)
        );
      }
    }) as T,
    [callback, delay]
  );
};

/**
 * Hook for request cancellation
 */
export const useCancellableRequest = () => {
  const abortControllerRef = useRef<AbortController | null>(null);

  const cancelRequest = useCallback(() => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
      abortControllerRef.current = null;
    }
  }, []);

  const createRequest = useCallback(() => {
    // Cancel any existing request
    cancelRequest();

    // Create new abort controller
    abortControllerRef.current = new AbortController();
    return abortControllerRef.current.signal;
  }, [cancelRequest]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      cancelRequest();
    };
  }, [cancelRequest]);

  return {
    createRequest,
    cancelRequest,
    signal: abortControllerRef.current?.signal,
  };
};