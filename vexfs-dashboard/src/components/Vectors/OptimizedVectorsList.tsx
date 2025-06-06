/**
 * Optimized Vectors List Component
 * Demonstrates performance optimizations and error handling improvements
 */

import React, { useState, useMemo, useCallback, useEffect } from 'react';
import {
  Box,
  Card,
  CardContent,
  TextField,
  InputAdornment,
  IconButton,
  Typography,
  CircularProgress,
  Alert,
  Chip,
  Tooltip,
} from '@mui/material';
import {
  Search as SearchIcon,
  Visibility as ViewIcon,
  Edit as EditIcon,
  Delete as DeleteIcon,
  Refresh as RefreshIcon,
} from '@mui/icons-material';
import { FixedSizeList as List } from 'react-window';
import { format } from 'date-fns';
import { usePerformanceOptimization, useDebounce, useCancellableRequest } from '../../hooks/usePerformanceOptimization';
import { useEnhancedErrorHandler } from '../../hooks/useEnhancedErrorHandler';
import type { VexFSVector } from '../../types';

interface OptimizedVectorsListProps {
  vectors: VexFSVector[];
  loading: boolean;
  error: string | null;
  onViewDetails: (vector: VexFSVector) => void;
  onEdit: (vector: VexFSVector) => void;
  onDelete: (vector: VexFSVector) => void;
  onRefresh?: () => void;
}

// Memoized row component with performance monitoring
const VectorRow = React.memo<{
  index: number;
  style: React.CSSProperties;
  data: {
    vectors: VexFSVector[];
    onViewDetails: (vector: VexFSVector) => void;
    onEdit: (vector: VexFSVector) => void;
    onDelete: (vector: VexFSVector) => void;
  };
}>(({ index, style, data }) => {
  const { vectors, onViewDetails, onEdit, onDelete } = data;
  const vector = vectors[index];
  const { startMeasurement, endMeasurement } = usePerformanceOptimization('VectorRow');

  useEffect(() => {
    startMeasurement();
    return () => {
      endMeasurement();
    };
  }, [startMeasurement, endMeasurement]);

  const formatDate = useCallback((dateString: string) => {
    try {
      return format(new Date(dateString), 'MMM dd, yyyy HH:mm');
    } catch {
      return 'Invalid date';
    }
  }, []);

  const getSimilarityColor = useCallback((similarity: number) => {
    if (similarity > 0.9) return 'success';
    if (similarity > 0.7) return 'warning';
    return 'error';
  }, []);

  if (!vector) {
    return (
      <Box style={style} sx={{ display: 'flex', alignItems: 'center', px: 2 }}>
        <Typography variant="body2" color="text.secondary">
          Loading...
        </Typography>
      </Box>
    );
  }

  return (
    <Box
      style={style}
      sx={{
        display: 'flex',
        alignItems: 'center',
        px: 2,
        py: 1,
        borderBottom: '1px solid',
        borderColor: 'divider',
        '&:hover': {
          bgcolor: 'action.hover',
        },
      }}
    >
      {/* Vector ID */}
      <Box sx={{ flex: '0 0 120px', minWidth: 0 }}>
        <Typography variant="subtitle2" sx={{ fontWeight: 600 }} noWrap>
          {vector.id}
        </Typography>
      </Box>

      {/* Dimensions */}
      <Box sx={{ flex: '0 0 100px', textAlign: 'center' }}>
        <Typography variant="body2">{vector.vector?.length || 0}</Typography>
      </Box>

      {/* Similarity Score */}
      <Box sx={{ flex: '0 0 120px', textAlign: 'center' }}>
        {vector.similarity !== undefined && (
          <Chip
            label={vector.similarity.toFixed(3)}
            size="small"
            color={getSimilarityColor(vector.similarity) as any}
            variant="outlined"
          />
        )}
      </Box>

      {/* Metadata Count */}
      <Box sx={{ flex: '0 0 100px', textAlign: 'center' }}>
        <Typography variant="body2">
          {vector.metadata ? Object.keys(vector.metadata).length : 0}
        </Typography>
      </Box>

      {/* Created Date */}
      <Box sx={{ flex: '0 0 150px', textAlign: 'center' }}>
        <Typography variant="body2" color="text.secondary">
          {vector.createdAt ? formatDate(vector.createdAt) : 'N/A'}
        </Typography>
      </Box>

      {/* Actions */}
      <Box sx={{ flex: '0 0 120px', display: 'flex', gap: 0.5, justifyContent: 'center' }}>
        <Tooltip title="View Details">
          <IconButton
            size="small"
            onClick={() => onViewDetails(vector)}
            color="primary"
          >
            <ViewIcon fontSize="small" />
          </IconButton>
        </Tooltip>
        <Tooltip title="Edit Vector">
          <IconButton
            size="small"
            onClick={() => onEdit(vector)}
            color="primary"
          >
            <EditIcon fontSize="small" />
          </IconButton>
        </Tooltip>
        <Tooltip title="Delete Vector">
          <IconButton
            size="small"
            onClick={() => onDelete(vector)}
            color="error"
          >
            <DeleteIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      </Box>
    </Box>
  );
});

VectorRow.displayName = 'VectorRow';

const OptimizedVectorsList: React.FC<OptimizedVectorsListProps> = ({
  vectors,
  loading,
  error,
  onViewDetails,
  onEdit,
  onDelete,
  onRefresh,
}) => {
  const [searchTerm, setSearchTerm] = useState('');
  const { startMeasurement, endMeasurement, metrics } = usePerformanceOptimization('OptimizedVectorsList');
  const { handleError, handleSyncError } = useEnhancedErrorHandler();
  const { createRequest, cancelRequest } = useCancellableRequest();

  // Debounce search term to prevent excessive filtering
  const debouncedSearchTerm = useDebounce(searchTerm, 300);

  // Start performance measurement on mount
  useEffect(() => {
    startMeasurement();
    return () => {
      endMeasurement();
    };
  }, [startMeasurement, endMeasurement]);

  // Memoized filtered vectors with performance optimization
  const filteredVectors = useMemo(() => {
    if (!debouncedSearchTerm) return vectors;

    const start = performance.now();
    const filtered = vectors.filter(vector => {
      const searchLower = debouncedSearchTerm.toLowerCase();
      return (
        vector.id.toLowerCase().includes(searchLower) ||
        (vector.metadata && 
          Object.values(vector.metadata).some(value => 
            String(value).toLowerCase().includes(searchLower)
          ))
      );
    });
    const end = performance.now();

    if (end - start > 10) {
      console.warn(`Slow vector filtering: ${(end - start).toFixed(2)}ms for ${vectors.length} vectors`);
    }

    return filtered;
  }, [vectors, debouncedSearchTerm]);

  // Memoized handlers with error handling
  const handleSearchChange = useCallback(async (event: React.ChangeEvent<HTMLInputElement>) => {
    try {
      setSearchTerm(event.target.value);
    } catch (error) {
      handleSyncError(error as Error, 'OptimizedVectorsList', 'handleSearchChange');
    }
  }, [handleSyncError]);

  const handleRefresh = useCallback(async () => {
    if (!onRefresh) return;

    const result = await handleError(
      async () => {
        const signal = createRequest();
        // Simulate API call with cancellation support
        await new Promise((resolve, reject) => {
          const timeout = setTimeout(resolve, 1000);
          signal.addEventListener('abort', () => {
            clearTimeout(timeout);
            reject(new Error('Request cancelled'));
          });
        });
        onRefresh();
      },
      'OptimizedVectorsList',
      'handleRefresh'
    );

    if (!result) {
      console.warn('Refresh operation failed or was cancelled');
    }
  }, [onRefresh, handleError, createRequest]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      cancelRequest();
    };
  }, [cancelRequest]);

  // Memoized data for virtualized list
  const listData = useMemo(() => ({
    vectors: filteredVectors,
    onViewDetails,
    onEdit,
    onDelete,
  }), [filteredVectors, onViewDetails, onEdit, onDelete]);

  if (loading) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
        <CircularProgress />
      </Box>
    );
  }

  if (error) {
    return (
      <Alert severity="error" sx={{ mb: 3 }}>
        Failed to load vectors: {error}
      </Alert>
    );
  }

  if (vectors.length === 0) {
    return (
      <Card>
        <CardContent>
          <Box sx={{ textAlign: 'center', py: 8 }}>
            <Typography variant="h6" color="text.secondary" gutterBottom>
              No Vectors Found
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
              Add vectors to this collection to get started.
            </Typography>
          </Box>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardContent>
        {/* Search and Controls */}
        <Box sx={{ mb: 3, display: 'flex', gap: 2, alignItems: 'center' }}>
          <TextField
            placeholder="Search vectors..."
            value={searchTerm}
            onChange={handleSearchChange}
            InputProps={{
              startAdornment: (
                <InputAdornment position="start">
                  <SearchIcon />
                </InputAdornment>
              ),
            }}
            sx={{ flexGrow: 1 }}
            size="small"
          />
          {onRefresh && (
            <Tooltip title="Refresh vectors">
              <IconButton onClick={handleRefresh} color="primary">
                <RefreshIcon />
              </IconButton>
            </Tooltip>
          )}
        </Box>

        {/* Performance Metrics (Development Only) */}
        {import.meta.env.DEV && metrics.renderTime > 0 && (
          <Alert severity="info" sx={{ mb: 2 }}>
            <Typography variant="caption">
              Performance: {metrics.renderTime.toFixed(2)}ms render, 
              {metrics.memoryUsage.toFixed(1)}MB memory, 
              {metrics.rerenderCount} re-renders
            </Typography>
          </Alert>
        )}

        {/* Results Summary */}
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          Showing {filteredVectors.length} of {vectors.length} vectors
          {debouncedSearchTerm && ` matching "${debouncedSearchTerm}"`}
        </Typography>

        {/* Table Header */}
        <Box
          sx={{
            display: 'flex',
            alignItems: 'center',
            px: 2,
            py: 1,
            bgcolor: 'grey.50',
            borderRadius: 1,
            mb: 1,
          }}
        >
          <Box sx={{ flex: '0 0 120px' }}>
            <Typography variant="subtitle2" fontWeight="bold">
              Vector ID
            </Typography>
          </Box>
          <Box sx={{ flex: '0 0 100px', textAlign: 'center' }}>
            <Typography variant="subtitle2" fontWeight="bold">
              Dimensions
            </Typography>
          </Box>
          <Box sx={{ flex: '0 0 120px', textAlign: 'center' }}>
            <Typography variant="subtitle2" fontWeight="bold">
              Similarity
            </Typography>
          </Box>
          <Box sx={{ flex: '0 0 100px', textAlign: 'center' }}>
            <Typography variant="subtitle2" fontWeight="bold">
              Metadata
            </Typography>
          </Box>
          <Box sx={{ flex: '0 0 150px', textAlign: 'center' }}>
            <Typography variant="subtitle2" fontWeight="bold">
              Created
            </Typography>
          </Box>
          <Box sx={{ flex: '0 0 120px', textAlign: 'center' }}>
            <Typography variant="subtitle2" fontWeight="bold">
              Actions
            </Typography>
          </Box>
        </Box>

        {/* Virtualized List */}
        <Box sx={{ height: 400, border: '1px solid', borderColor: 'divider', borderRadius: 1 }}>
          <List
            height={400}
            width="100%"
            itemCount={filteredVectors.length}
            itemSize={60}
            itemData={listData}
            overscanCount={5} // Render 5 extra items for smooth scrolling
          >
            {VectorRow}
          </List>
        </Box>
      </CardContent>
    </Card>
  );
};

export default React.memo(OptimizedVectorsList);