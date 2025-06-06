import { useEffect, useRef, useCallback } from 'react';

interface PerformanceMetrics {
  componentName: string;
  renderTime: number;
  timestamp: number;
}

interface UsePerformanceMonitorOptions {
  enabled?: boolean;
  threshold?: number; // Log only if render time exceeds threshold (ms)
  onMetric?: (metric: PerformanceMetrics) => void;
}

/**
 * Hook to monitor component performance and render times
 */
export const usePerformanceMonitor = (
  componentName: string,
  options: UsePerformanceMonitorOptions = {}
) => {
  const {
    enabled = import.meta.env.DEV,
    threshold = 16, // 16ms = 60fps
    onMetric,
  } = options;

  const renderStartTime = useRef<number>(0);
  const renderCount = useRef<number>(0);
  const totalRenderTime = useRef<number>(0);

  // Start timing before render
  const startTiming = useCallback(() => {
    if (!enabled) return;
    renderStartTime.current = performance.now();
  }, [enabled]);

  // End timing after render
  const endTiming = useCallback(() => {
    if (!enabled || renderStartTime.current === 0) return;

    const renderTime = performance.now() - renderStartTime.current;
    renderCount.current += 1;
    totalRenderTime.current += renderTime;

    const metric: PerformanceMetrics = {
      componentName,
      renderTime,
      timestamp: Date.now(),
    };

    // Log if render time exceeds threshold
    if (renderTime > threshold) {
      console.warn(
        `🐌 Slow render detected in ${componentName}: ${renderTime.toFixed(2)}ms`
      );
    }

    // Call custom metric handler
    onMetric?.(metric);

    renderStartTime.current = 0;
  }, [enabled, componentName, threshold, onMetric]);

  // Get average render time
  const getAverageRenderTime = useCallback(() => {
    if (renderCount.current === 0) return 0;
    return totalRenderTime.current / renderCount.current;
  }, []);

  // Get render statistics
  const getStats = useCallback(() => {
    return {
      componentName,
      renderCount: renderCount.current,
      totalRenderTime: totalRenderTime.current,
      averageRenderTime: getAverageRenderTime(),
    };
  }, [componentName, getAverageRenderTime]);

  // Reset statistics
  const resetStats = useCallback(() => {
    renderCount.current = 0;
    totalRenderTime.current = 0;
  }, []);

  return {
    startTiming,
    endTiming,
    getStats,
    resetStats,
    getAverageRenderTime,
  };
};

/**
 * Hook to measure function execution time
 */
export const useFunctionTimer = (enabled = import.meta.env.DEV) => {
  const timeFunction = useCallback(
    <T extends any[], R>(
      fn: (...args: T) => R,
      functionName: string
    ): ((...args: T) => R) => {
      if (!enabled) return fn;

      return (...args: T): R => {
        const start = performance.now();
        const result = fn(...args);
        const end = performance.now();
        const duration = end - start;

        if (duration > 1) {
          // Log functions that take more than 1ms
          console.log(`⏱️ ${functionName}: ${duration.toFixed(2)}ms`);
        }

        return result;
      };
    },
    [enabled]
  );

  const timeAsyncFunction = useCallback(
    <T extends any[], R>(
      fn: (...args: T) => Promise<R>,
      functionName: string
    ): ((...args: T) => Promise<R>) => {
      if (!enabled) return fn;

      return async (...args: T): Promise<R> => {
        const start = performance.now();
        const result = await fn(...args);
        const end = performance.now();
        const duration = end - start;

        if (duration > 10) {
          // Log async functions that take more than 10ms
          console.log(`⏱️ ${functionName} (async): ${duration.toFixed(2)}ms`);
        }

        return result;
      };
    },
    [enabled]
  );

  return {
    timeFunction,
    timeAsyncFunction,
  };
};

/**
 * Hook to track component mount/unmount times
 */
export const useComponentLifecycle = (componentName: string) => {
  const mountTime = useRef<number>(0);

  useEffect(() => {
    mountTime.current = performance.now();
    if (import.meta.env.DEV) {
      console.log(`🚀 ${componentName} mounted`);
    }

    return () => {
      if (import.meta.env.DEV) {
        const unmountTime = performance.now();
        const lifespan = unmountTime - mountTime.current;
        console.log(`💀 ${componentName} unmounted after ${lifespan.toFixed(2)}ms`);
      }
    };
  }, [componentName]);
};

/**
 * Hook to monitor re-renders and their causes
 */
export const useRenderTracker = (componentName: string, props: Record<string, any>) => {
  const prevProps = useRef<Record<string, any> | undefined>(undefined);
  const renderCount = useRef(0);

  useEffect(() => {
    renderCount.current += 1;

    if (import.meta.env.DEV && prevProps.current) {
      const changedProps = Object.keys(props).filter(
        key => prevProps.current![key] !== props[key]
      );

      if (changedProps.length > 0) {
        console.log(
          `🔄 ${componentName} re-rendered (${renderCount.current}) due to props:`,
          changedProps
        );
      }
    }

    prevProps.current = props;
  });

  return renderCount.current;
};

export default usePerformanceMonitor;