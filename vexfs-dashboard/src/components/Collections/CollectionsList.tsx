import React, { useState, useMemo } from 'react';
import {
  Box,
  Card,
  CardContent,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TablePagination,
  TableSortLabel,
  TextField,
  InputAdornment,
  IconButton,
  Chip,
  Typography,
  Tooltip,
  CircularProgress,
  Alert,
} from '@mui/material';
import {
  Search as SearchIcon,
  Visibility as ViewIcon,
  Edit as EditIcon,
  Delete as DeleteIcon,
  FilterList as FilterIcon,
} from '@mui/icons-material';
import { format } from 'date-fns';
import type { VexFSCollection } from '../../types';

interface CollectionsListProps {
  collections: VexFSCollection[];
  loading: boolean;
  error: string | null;
  onViewDetails: (collection: VexFSCollection) => void;
  onEdit: (collection: VexFSCollection) => void;
  onDelete: (collection: VexFSCollection) => void;
}

type SortField = 'name' | 'pointsCount' | 'createdAt' | 'vectorSize';
type SortDirection = 'asc' | 'desc';

const CollectionsList: React.FC<CollectionsListProps> = ({
  collections,
  loading,
  error,
  onViewDetails,
  onEdit,
  onDelete,
}) => {
  const [page, setPage] = useState(0);
  const [rowsPerPage, setRowsPerPage] = useState(10);
  const [searchTerm, setSearchTerm] = useState('');
  const [sortField, setSortField] = useState<SortField>('createdAt');
  const [sortDirection, setSortDirection] = useState<SortDirection>('desc');

  // Filter and sort collections
  const filteredAndSortedCollections = useMemo(() => {
    const filtered = collections.filter(
      collection =>
        collection.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        (collection.description &&
          collection.description
            .toLowerCase()
            .includes(searchTerm.toLowerCase()))
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

  // Paginated collections
  const paginatedCollections = useMemo(() => {
    const startIndex = page * rowsPerPage;
    return filteredAndSortedCollections.slice(
      startIndex,
      startIndex + rowsPerPage
    );
  }, [filteredAndSortedCollections, page, rowsPerPage]);

  const handleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDirection(sortDirection === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDirection('asc');
    }
  };

  const handleChangePage = (_: unknown, newPage: number) => {
    setPage(newPage);
  };

  const handleChangeRowsPerPage = (
    event: React.ChangeEvent<HTMLInputElement>
  ) => {
    setRowsPerPage(parseInt(event.target.value, 10));
    setPage(0);
  };

  const formatDate = (dateString: string) => {
    try {
      return format(new Date(dateString), 'MMM dd, yyyy HH:mm');
    } catch {
      return 'Invalid date';
    }
  };

  const getDistanceMetricColor = (distance: string) => {
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
  };

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
            onChange={e => setSearchTerm(e.target.value)}
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
          Showing {paginatedCollections.length} of{' '}
          {filteredAndSortedCollections.length} collections
          {searchTerm && ` matching "${searchTerm}"`}
        </Typography>

        {/* Collections Table */}
        <TableContainer>
          <Table>
            <TableHead>
              <TableRow>
                <TableCell>
                  <TableSortLabel
                    active={sortField === 'name'}
                    direction={sortField === 'name' ? sortDirection : 'asc'}
                    onClick={() => handleSort('name')}
                  >
                    Name
                  </TableSortLabel>
                </TableCell>
                <TableCell>Description</TableCell>
                <TableCell align="center">
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
                <TableCell align="center">
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
                <TableCell align="center">Distance Metric</TableCell>
                <TableCell align="center">
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
                <TableCell align="center">Actions</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {paginatedCollections.map(collection => (
                <TableRow key={collection.id} hover>
                  <TableCell>
                    <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
                      {collection.name}
                    </Typography>
                  </TableCell>
                  <TableCell>
                    <Typography
                      variant="body2"
                      color="text.secondary"
                      sx={{
                        maxWidth: 200,
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {collection.description || 'No description'}
                    </Typography>
                  </TableCell>
                  <TableCell align="center">
                    <Typography variant="body2">
                      {collection.pointsCount.toLocaleString()}
                    </Typography>
                  </TableCell>
                  <TableCell align="center">
                    <Typography variant="body2">
                      {collection.vectorSize}
                    </Typography>
                  </TableCell>
                  <TableCell align="center">
                    <Chip
                      label={collection.distance}
                      size="small"
                      color={getDistanceMetricColor(collection.distance) as any}
                      variant="outlined"
                    />
                  </TableCell>
                  <TableCell align="center">
                    <Typography variant="body2" color="text.secondary">
                      {formatDate(collection.createdAt)}
                    </Typography>
                  </TableCell>
                  <TableCell align="center">
                    <Box
                      sx={{
                        display: 'flex',
                        gap: 0.5,
                        justifyContent: 'center',
                      }}
                    >
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
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>

        {/* Pagination */}
        <TablePagination
          component="div"
          count={filteredAndSortedCollections.length}
          page={page}
          onPageChange={handleChangePage}
          rowsPerPage={rowsPerPage}
          onRowsPerPageChange={handleChangeRowsPerPage}
          rowsPerPageOptions={[5, 10, 25, 50]}
          sx={{ mt: 2 }}
        />
      </CardContent>
    </Card>
  );
};

export default CollectionsList;
