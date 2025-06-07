import React, { useState, useMemo, useCallback } from 'react';
import {
  Box,
  Card,
  CardContent,
  TextField,
  InputAdornment,
  IconButton,
  Chip,
  Typography,
  Tooltip,
  CircularProgress,
  Alert,
  TableContainer,
  Table,
  TableHead,
  TableRow,
  TableCell,
  TableSortLabel,
} from '@mui/material';
import {
  Search as SearchIcon,
  Visibility as ViewIcon,
  Edit as EditIcon,
  Delete as DeleteIcon,
  FilterList as FilterIcon,
} from '@mui/icons-material';
import { FixedSizeList as List } from 'react-window';
import { format } from 'date-fns';
import type { VexFSCollection } from '../../types';

interface VirtualizedCollectionsListProps {
  collections: VexFSCollection[];
  loading: boolean;
  error: string | null;
  onViewDetails: (collection: VexFSCollection) => void;
  onEdit: (collection: VexFSCollection) => void;
  onDelete: (collection: VexFSCollection) => void;
}

type SortField = 'name' | 'pointsCount' | 'createdAt' | 'vectorSize';
type SortDirection = 'asc' | 'desc';

// Memoized row component for virtualization
const CollectionRow = React.memo<{
  index: number;
  style: React.CSSProperties;
  data: {
    collections: VexFSCollection[];
    onViewDetails: (collection: VexFSCollection) => void;
    onEdit: (collection: VexFSCollection) => void;
    onDelete: (collection: VexFSCollection) => void;
  };
}>(({ index, style, data }) => {
  const { collections, onViewDetails, onEdit, onDelete } = data;
  const collection = collections[index];

  const formatDate = useCallback((dateString: string) => {
    try {
      return format(new Date(dateString), 'MMM dd, yyyy HH:mm');
    } catch {
      return 'Invalid date';
    }
  }, []);

  const getDistanceMetricColor = useCallback((distance: string) => {
    switch (distance) {
      case 'cosine':
        return 'primary';
      case 'euclidean':
        return 'secondary';
      case 'dot':
        return 'success';
      default:
        return 'default';
    }
  }, []);

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
      {/* Name */}
      <Box sx={{ flex: '1 1 200px', minWidth: 0 }}>
        <Typography variant="subtitle2" sx={{ fontWeight: 600 }} noWrap>
          {collection.name}
        </Typography>
      </Box>

      {/* Description */}
      <Box sx={{ flex: '1 1 250px', minWidth: 0, mx: 2 }}>
        <Typography
          variant="body2"
          color="text.secondary"
          sx={{
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {collection.description || 'No description'}
        </Typography>
      </Box>

      {/* Vector Count */}
      <Box sx={{ flex: '0 0 120px', textAlign: 'center' }}>
        <Typography variant="body2">
          {collection.pointsCount.toLocaleString()}
        </Typography>
      </Box>

      {/* Dimensions */}
      <Box sx={{ flex: '0 0 100px', textAlign: 'center' }}>
        <Typography variant="body2">{collection.vectorSize}</Typography>
      </Box>

      {/* Distance Metric */}
      <Box sx={{ flex: '0 0 120px', textAlign: 'center' }}>
        <Chip
          label={collection.distance}
          size="small"
          color={getDistanceMetricColor(collection.distance) as any}
          variant="outlined"
        />
      </Box>

      {/* Created Date */}
      <Box sx={{ flex: '0 0 150px', textAlign: 'center' }}>
        <Typography variant="body2" color="text.secondary">
          {formatDate(collection.createdAt)}
        </Typography>
      </Box>

      {/* Actions */}
      <Box sx={{ flex: '0 0 120px', display: 'flex', gap: 0.5, justifyContent: 'center' }}>
        <Tooltip title="View Details">
          <IconButton
            size="small"
            onClick={() => onViewDetails(collection)}
            color="primary"
          >
            <ViewIcon fontSize="small" />
          </IconButton>
        </Tooltip>
        <Tooltip title="Edit Collection">
          <IconButton
            size="small"
            onClick={() => onEdit(collection)}
            color="primary"
          >
            <EditIcon fontSize="small" />
          </IconButton>
        </Tooltip>
        <Tooltip title="Delete Collection">
          <IconButton
            size="small"
            onClick={() => onDelete(collection)}
            color="error"
          >
            <DeleteIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      </Box>
    </Box>
  );
});

CollectionRow.displayName = 'CollectionRow';

const VirtualizedCollectionsList: React.FC<VirtualizedCollectionsListProps> = ({
  collections,
  loading,
  error,
  onViewDetails,
  onEdit,
  onDelete,
}) => {
  const [searchTerm, setSearchTerm] = useState('');
  const [sortField, setSortField] = useState<SortField>('createdAt');
  const [sortDirection, setSortDirection] = useState<SortDirection>('desc');

  // Memoized filtered and sorted collections
  const filteredAndSortedCollections = useMemo(() => {
    const filtered = collections.filter(
      collection =>
        collection &&
        collection.name &&
        (collection.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
          (collection.description &&
            collection.description
              .toLowerCase()
              .includes(searchTerm.toLowerCase())))
    );

    // Sort collections
    filtered.sort((a, b) => {
      let aValue: string | number;
      let bValue: string | number;

      switch (sortField) {
        case 'name':
          aValue = a.name.toLowerCase();
          bValue = b.name.toLowerCase();
          break;
        case 'pointsCount':
          aValue = a.pointsCount;
          bValue = b.pointsCount;
          break;
        case 'createdAt':
          aValue = new Date(a.createdAt).getTime();
          bValue = new Date(b.createdAt).getTime();
          break;
        case 'vectorSize':
          aValue = a.vectorSize;
          bValue = b.vectorSize;
          break;
        default:
          return 0;
      }

      if (aValue < bValue) return sortDirection === 'asc' ? -1 : 1;
      if (aValue > bValue) return sortDirection === 'asc' ? 1 : -1;
      return 0;
    });

    return filtered;
  }, [collections, searchTerm, sortField, sortDirection]);

  // Memoized handlers
  const handleSort = useCallback((field: SortField) => {
    if (sortField === field) {
      setSortDirection(sortDirection === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDirection('asc');
    }
  }, [sortField, sortDirection]);

  const handleSearchChange = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
    setSearchTerm(event.target.value);
  }, []);

  // Memoized data for virtualized list
  const listData = useMemo(() => ({
    collections: filteredAndSortedCollections,
    onViewDetails,
    onEdit,
    onDelete,
  }), [filteredAndSortedCollections, onViewDetails, onEdit, onDelete]);

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
        Failed to load collections: {error}
      </Alert>
    );
  }

  if (collections.length === 0) {
    return (
      <Card>
        <CardContent>
          <Box sx={{ textAlign: 'center', py: 8 }}>
            <Typography variant="h6" color="text.secondary" gutterBottom>
              No Collections Found
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
              Create your first collection to get started with VexFS.
            </Typography>
          </Box>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardContent>
        {/* Search and Filter Bar */}
        <Box sx={{ mb: 3, display: 'flex', gap: 2, alignItems: 'center' }}>
          <TextField
            placeholder="Search collections..."
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
          <Tooltip title="Advanced filters coming soon">
            <IconButton disabled>
              <FilterIcon />
            </IconButton>
          </Tooltip>
        </Box>

        {/* Results Summary */}
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          Showing {filteredAndSortedCollections.length} collections
          {searchTerm && ` matching "${searchTerm}"`}
        </Typography>

        {/* Table Header */}
        <TableContainer>
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell sx={{ width: 200 }}>
                  <TableSortLabel
                    active={sortField === 'name'}
                    direction={sortField === 'name' ? sortDirection : 'asc'}
                    onClick={() => handleSort('name')}
                  >
                    Name
                  </TableSortLabel>
                </TableCell>
                <TableCell sx={{ width: 250 }}>Description</TableCell>
                <TableCell align="center" sx={{ width: 120 }}>
                  <TableSortLabel
                    active={sortField === 'pointsCount'}
                    direction={
                      sortField === 'pointsCount' ? sortDirection : 'asc'
                    }
                    onClick={() => handleSort('pointsCount')}
                  >
                    Vector Count
                  </TableSortLabel>
                </TableCell>
                <TableCell align="center" sx={{ width: 100 }}>
                  <TableSortLabel
                    active={sortField === 'vectorSize'}
                    direction={
                      sortField === 'vectorSize' ? sortDirection : 'asc'
                    }
                    onClick={() => handleSort('vectorSize')}
                  >
                    Dimensions
                  </TableSortLabel>
                </TableCell>
                <TableCell align="center" sx={{ width: 120 }}>Distance Metric</TableCell>
                <TableCell align="center" sx={{ width: 150 }}>
                  <TableSortLabel
                    active={sortField === 'createdAt'}
                    direction={
                      sortField === 'createdAt' ? sortDirection : 'asc'
                    }
                    onClick={() => handleSort('createdAt')}
                  >
                    Created
                  </TableSortLabel>
                </TableCell>
                <TableCell align="center" sx={{ width: 120 }}>Actions</TableCell>
              </TableRow>
            </TableHead>
          </Table>
        </TableContainer>

        {/* Virtualized List */}
        <Box sx={{ height: 400, mt: 1 }}>
          <List
            height={400}
            width="100%"
            itemCount={filteredAndSortedCollections.length}
            itemSize={60}
            itemData={listData}
          >
            {CollectionRow}
          </List>
        </Box>
      </CardContent>
    </Card>
  );
};

export default React.memo(VirtualizedCollectionsList);