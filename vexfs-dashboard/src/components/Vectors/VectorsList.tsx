import React, { useState, useEffect, useMemo } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TablePagination,
  TextField,
  InputAdornment,
  IconButton,
  Button,
  Chip,
  Menu,
  MenuItem,
  CircularProgress,
  Alert,
  Tooltip,
} from '@mui/material';
import {
  Search as SearchIcon,
  FilterList as FilterIcon,
  Sort as SortIcon,
  Visibility as ViewIcon,
  Edit as EditIcon,
  Delete as DeleteIcon,
  ContentCopy as CopyIcon,
  Add as AddIcon,
} from '@mui/icons-material';
import { formatNumber, truncateText } from '../../utils';
import type { VexFSPoint, VectorListFilters } from '../../types';

interface VectorsListProps {
  collectionName: string;
  vectors: VexFSPoint[];
  loading?: boolean;
  error?: string | null;
  totalCount: number;
  page: number;
  pageSize: number;
  onPageChange: (page: number) => void;
  onPageSizeChange: (pageSize: number) => void;
  onFiltersChange: (filters: VectorListFilters) => void;
  onViewVector: (vector: VexFSPoint) => void;
  onEditVector: (vector: VexFSPoint) => void;
  onDeleteVector: (vector: VexFSPoint) => void;
  onCopyVector: (vector: VexFSPoint) => void;
  onAddVector: () => void;
}

const VectorsList: React.FC<VectorsListProps> = ({
  collectionName,
  vectors,
  loading = false,
  error = null,
  totalCount,
  page,
  pageSize,
  onPageChange,
  onPageSizeChange,
  onFiltersChange,
  onViewVector,
  onEditVector,
  onDeleteVector,
  onCopyVector,
  onAddVector,
}) => {
  const [searchTerm, setSearchTerm] = useState('');
  const [sortBy, setSortBy] = useState<'id' | 'createdAt' | 'similarity'>('id');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('asc');
  const [filterMenuAnchor, setFilterMenuAnchor] = useState<null | HTMLElement>(
    null
  );

  // Update filters when search or sort changes
  useEffect(() => {
    const filters: VectorListFilters = {
      search: searchTerm || undefined,
      sortBy,
      sortOrder,
    };
    onFiltersChange(filters);
  }, [searchTerm, sortBy, sortOrder, onFiltersChange]);

  const handleSortChange = (newSortBy: 'id' | 'createdAt' | 'similarity') => {
    if (sortBy === newSortBy) {
      setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
    } else {
      setSortBy(newSortBy);
      setSortOrder('asc');
    }
    setFilterMenuAnchor(null);
  };

  const getSortIcon = (field: 'id' | 'createdAt' | 'similarity') => {
    if (sortBy !== field) return null;
    return sortOrder === 'asc' ? '↑' : '↓';
  };

  const formatVectorId = (id: string | number) => {
    const idStr = String(id);
    return idStr.length > 20 ? truncateText(idStr, 20) : idStr;
  };

  const formatMetadata = (payload?: Record<string, unknown>) => {
    if (!payload || Object.keys(payload).length === 0) {
      return 'No metadata';
    }
    const keys = Object.keys(payload);
    if (keys.length === 1) {
      return `${keys[0]}: ${String(payload[keys[0]]).substring(0, 30)}`;
    }
    return `${keys.length} fields`;
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch (err) {
      console.error('Failed to copy to clipboard:', err);
    }
  };

  const handleCopyVector = (vector: VexFSPoint) => {
    const vectorData = JSON.stringify(vector, null, 2);
    copyToClipboard(vectorData);
    onCopyVector(vector);
  };

  if (error) {
    return (
      <Alert severity="error" sx={{ mb: 3 }}>
        Failed to load vectors: {error}
      </Alert>
    );
  }

  return (
    <Card>
      <CardContent>
        {/* Header */}
        <Box
          sx={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            mb: 3,
          }}
        >
          <Typography variant="h6" sx={{ fontWeight: 600 }}>
            Vectors in {collectionName}
          </Typography>
          <Button
            variant="contained"
            startIcon={<AddIcon />}
            onClick={onAddVector}
            sx={{ borderRadius: 2 }}
          >
            Add Vector
          </Button>
        </Box>

        {/* Search and Filters */}
        <Box
          sx={{
            display: 'flex',
            gap: 2,
            mb: 3,
            alignItems: 'center',
            flexWrap: 'wrap',
          }}
        >
          <TextField
            placeholder="Search vectors by ID or metadata..."
            value={searchTerm}
            onChange={e => setSearchTerm(e.target.value)}
            size="small"
            sx={{ flexGrow: 1, minWidth: 300 }}
            InputProps={{
              startAdornment: (
                <InputAdornment position="start">
                  <SearchIcon />
                </InputAdornment>
              ),
            }}
          />
          <Button
            variant="outlined"
            startIcon={<FilterIcon />}
            endIcon={<SortIcon />}
            onClick={e => setFilterMenuAnchor(e.currentTarget)}
            sx={{ borderRadius: 2 }}
          >
            Sort: {sortBy} {getSortIcon(sortBy)}
          </Button>
        </Box>

        {/* Sort Menu */}
        <Menu
          anchorEl={filterMenuAnchor}
          open={Boolean(filterMenuAnchor)}
          onClose={() => setFilterMenuAnchor(null)}
        >
          <MenuItem onClick={() => handleSortChange('id')}>
            Sort by ID {getSortIcon('id')}
          </MenuItem>
          <MenuItem onClick={() => handleSortChange('createdAt')}>
            Sort by Created Date {getSortIcon('createdAt')}
          </MenuItem>
          <MenuItem onClick={() => handleSortChange('similarity')}>
            Sort by Similarity {getSortIcon('similarity')}
          </MenuItem>
        </Menu>

        {/* Loading State */}
        {loading && (
          <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
            <CircularProgress />
          </Box>
        )}

        {/* Empty State */}
        {!loading && vectors.length === 0 && (
          <Box sx={{ textAlign: 'center', py: 6 }}>
            <Typography variant="h6" color="text.secondary" gutterBottom>
              No vectors found
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
              {searchTerm
                ? 'Try adjusting your search criteria'
                : 'This collection is empty. Add some vectors to get started.'}
            </Typography>
            <Button
              variant="contained"
              startIcon={<AddIcon />}
              onClick={onAddVector}
            >
              Add First Vector
            </Button>
          </Box>
        )}

        {/* Vectors Table */}
        {!loading && vectors.length > 0 && (
          <>
            <TableContainer>
              <Table>
                <TableHead>
                  <TableRow>
                    <TableCell>Vector ID</TableCell>
                    <TableCell>Dimensions</TableCell>
                    <TableCell>Metadata</TableCell>
                    <TableCell>Similarity Score</TableCell>
                    <TableCell align="right">Actions</TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {vectors.map((vector, index) => (
                    <TableRow key={`${vector.id}-${index}`} hover>
                      <TableCell>
                        <Box sx={{ display: 'flex', alignItems: 'center' }}>
                          <Typography
                            variant="body2"
                            sx={{ fontFamily: 'monospace' }}
                          >
                            {formatVectorId(vector.id)}
                          </Typography>
                          <Tooltip title="Copy Vector ID">
                            <IconButton
                              size="small"
                              onClick={() => copyToClipboard(String(vector.id))}
                              sx={{ ml: 1 }}
                            >
                              <CopyIcon fontSize="small" />
                            </IconButton>
                          </Tooltip>
                        </Box>
                      </TableCell>
                      <TableCell>
                        <Chip
                          label={formatNumber(vector.vector.length)}
                          size="small"
                          variant="outlined"
                        />
                      </TableCell>
                      <TableCell>
                        <Typography variant="body2" color="text.secondary">
                          {formatMetadata(vector.payload)}
                        </Typography>
                      </TableCell>
                      <TableCell>
                        {/* Similarity score would come from search results */}
                        <Typography variant="body2" color="text.secondary">
                          -
                        </Typography>
                      </TableCell>
                      <TableCell align="right">
                        <Box sx={{ display: 'flex', gap: 0.5 }}>
                          <Tooltip title="View Details">
                            <IconButton
                              size="small"
                              onClick={() => onViewVector(vector)}
                            >
                              <ViewIcon fontSize="small" />
                            </IconButton>
                          </Tooltip>
                          <Tooltip title="Edit Vector">
                            <IconButton
                              size="small"
                              onClick={() => onEditVector(vector)}
                            >
                              <EditIcon fontSize="small" />
                            </IconButton>
                          </Tooltip>
                          <Tooltip title="Copy Vector">
                            <IconButton
                              size="small"
                              onClick={() => handleCopyVector(vector)}
                            >
                              <CopyIcon fontSize="small" />
                            </IconButton>
                          </Tooltip>
                          <Tooltip title="Delete Vector">
                            <IconButton
                              size="small"
                              color="error"
                              onClick={() => onDeleteVector(vector)}
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
              count={totalCount}
              page={page}
              onPageChange={(_, newPage) => onPageChange(newPage)}
              rowsPerPage={pageSize}
              onRowsPerPageChange={e =>
                onPageSizeChange(parseInt(e.target.value, 10))
              }
              rowsPerPageOptions={[10, 25, 50, 100]}
              showFirstButton
              showLastButton
            />
          </>
        )}
      </CardContent>
    </Card>
  );
};

export default VectorsList;
