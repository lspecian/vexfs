import { useState, useEffect, useCallback } from 'react';
import { vexfsApi } from '../services/api';
import type {
  VexFSCollection,
  VexFSPoint,
  VexFSSearchRequest,
  VexFSSearchResult,
  DashboardStats,
  PaginatedResponse,
} from '../types';

// Hook for managing collections
export const useCollections = () => {
  const [collections, setCollections] = useState<VexFSCollection[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchCollections = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await vexfsApi.getCollections();
      setCollections(data);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : 'Failed to fetch collections'
      );
    } finally {
      setLoading(false);
    }
  }, []);

  const createCollection = useCallback(
    async (
      name: string,
      vectorSize: number,
      distance: 'cosine' | 'euclidean' | 'dot' = 'cosine'
    ) => {
      setLoading(true);
      setError(null);
      try {
        const newCollection = await vexfsApi.createCollection(
          name,
          vectorSize,
          distance
        );
        setCollections(prev => [...prev, newCollection]);
        return newCollection;
      } catch (err) {
        setError(
          err instanceof Error ? err.message : 'Failed to create collection'
        );
        throw err;
      } finally {
        setLoading(false);
      }
    },
    []
  );

  const deleteCollection = useCallback(async (name: string) => {
    setLoading(true);
    setError(null);
    try {
      const success = await vexfsApi.deleteCollection(name);
      if (success) {
        setCollections(prev => prev.filter(col => col.name !== name));
      }
      return success;
    } catch (err) {
      setError(
        err instanceof Error ? err.message : 'Failed to delete collection'
      );
      throw err;
    } finally {
      setLoading(false);
    }
  }, []);

  const updateCollection = useCallback(
    async (name: string, updates: { description?: string }) => {
      setLoading(true);
      setError(null);
      try {
        const updatedCollection = await vexfsApi.updateCollection(
          name,
          updates
        );
        setCollections(prev =>
          prev.map(col => (col.name === name ? updatedCollection : col))
        );
        return updatedCollection;
      } catch (err) {
        setError(
          err instanceof Error ? err.message : 'Failed to update collection'
        );
        throw err;
      } finally {
        setLoading(false);
      }
    },
    []
  );

  useEffect(() => {
    fetchCollections();
  }, [fetchCollections]);

  return {
    collections,
    loading,
    error,
    fetchCollections,
    createCollection,
    updateCollection,
    deleteCollection,
  };
};

// Hook for managing points in a collection
export const usePoints = (collectionName: string) => {
  const [points, setPoints] = useState<PaginatedResponse<VexFSPoint>>({
    items: [],
    total: 0,
    page: 1,
    pageSize: 100,
    hasNext: false,
    hasPrev: false,
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchPoints = useCallback(
    async (limit = 100, offset = 0) => {
      if (!collectionName) return;

      setLoading(true);
      setError(null);
      try {
        const data = await vexfsApi.getPoints(collectionName, limit, offset);
        setPoints(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to fetch points');
      } finally {
        setLoading(false);
      }
    },
    [collectionName]
  );

  const upsertPoints = useCallback(
    async (newPoints: VexFSPoint[]) => {
      if (!collectionName) return false;

      setLoading(true);
      setError(null);
      try {
        const success = await vexfsApi.upsertPoints(collectionName, newPoints);
        if (success) {
          // Refresh points after successful upsert
          await fetchPoints();
        }
        return success;
      } catch (err) {
        setError(
          err instanceof Error ? err.message : 'Failed to upsert points'
        );
        throw err;
      } finally {
        setLoading(false);
      }
    },
    [collectionName, fetchPoints]
  );

  const deletePoints = useCallback(
    async (pointIds: (string | number)[]) => {
      if (!collectionName) return false;

      setLoading(true);
      setError(null);
      try {
        const success = await vexfsApi.deletePoints(collectionName, pointIds);
        if (success) {
          // Refresh points after successful deletion
          await fetchPoints();
        }
        return success;
      } catch (err) {
        setError(
          err instanceof Error ? err.message : 'Failed to delete points'
        );
        throw err;
      } finally {
        setLoading(false);
      }
    },
    [collectionName, fetchPoints]
  );

  useEffect(() => {
    fetchPoints();
  }, [fetchPoints]);

  return {
    points,
    loading,
    error,
    fetchPoints,
    upsertPoints,
    deletePoints,
  };
};

// Hook for vector search
export const useVectorSearch = () => {
  const [results, setResults] = useState<VexFSSearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const search = useCallback(
    async (collectionName: string, searchRequest: VexFSSearchRequest) => {
      setLoading(true);
      setError(null);
      try {
        const searchResults = await vexfsApi.searchPoints(
          collectionName,
          searchRequest
        );
        setResults(searchResults);
        return searchResults;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Search failed');
        throw err;
      } finally {
        setLoading(false);
      }
    },
    []
  );

  return {
    results,
    loading,
    error,
    search,
  };
};

// Hook for dashboard stats
export const useDashboardStats = () => {
  const [stats, setStats] = useState<DashboardStats>({
    totalCollections: 0,
    totalPoints: 0,
    totalStorage: '0 B',
    serverStatus: 'offline',
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchStats = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await vexfsApi.getDashboardStats();
      setStats(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch stats');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchStats();
    // Set up periodic refresh every 30 seconds
    const interval = setInterval(fetchStats, 30000);
    return () => clearInterval(interval);
  }, [fetchStats]);

  return {
    stats,
    loading,
    error,
    fetchStats,
  };
};

// Hook for server health check
export const useServerHealth = () => {
  const [isHealthy, setIsHealthy] = useState(false);
  const [loading, setLoading] = useState(false);

  const checkHealth = useCallback(async () => {
    setLoading(true);
    try {
      const healthy = await vexfsApi.healthCheck();
      setIsHealthy(healthy);
      return healthy;
    } catch {
      setIsHealthy(false);
      return false;
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    checkHealth();
    // Check health every 10 seconds
    const interval = setInterval(checkHealth, 10000);
    return () => clearInterval(interval);
  }, [checkHealth]);

  return {
    isHealthy,
    loading,
    checkHealth,
  };
};
