import React, { useState, useMemo } from 'react';
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
  Chip,
  IconButton,
  Tooltip,
  Button,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  TextField,
  Grid,
  Paper,
  Divider,
  Menu,
  ListItemIcon,
  ListItemText,
  Alert,
  LinearProgress,
} from '@mui/material';
import {
  Visibility as ViewIcon,
  GetApp as ExportIcon,
  Sort as SortIcon,
  FilterList as FilterIcon,
  ViewList as ListViewIcon,
  ViewModule as GridViewIcon,
  TableChart as TableViewIcon,
  BarChart as ChartIcon,
  ContentCopy as CopyIcon,
  Share as ShareIcon,
} from '@mui/icons-material';
import { formatNumber, truncateText } from '../../utils';
import type {
  AdvancedSearchResult,
  SearchResultsResponse,
  SearchExportOptions,
  SearchSorting,
} from '../../types';

interface SearchResultsProps {
  results: SearchResultsResponse | null;
  loading?: boolean;
  onViewResult: (result: AdvancedSearchResult) => void;
  onExport?: (options: SearchExportOptions) => void;
  onPageChange: (page: number, pageSize: number) => void;
}

type ViewMode = 'list' | 'grid' | 'table';

const SearchResults: React.FC<SearchResultsProps> = ({
  results,
  loading = false,
  onViewResult,
  onExport,
  onPageChange,
}) => {
  const [viewMode, setViewMode] = useState<ViewMode>('table');
  const [sortBy, setSortBy] = useState<SearchSorting>({
    field: 'similarity',
    order: 'desc',
  });
  const [filterThreshold, setFilterThreshold] = useState<number>(0);
  const [exportMenuAnchor, setExportMenuAnchor] = useState<null | HTMLElement>(
    null
  );
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState(25);

  const filteredAndSortedResults = useMemo(() => {
    if (!results?.results) return [];

    const filtered = results.results.filter(
      result => result.score >= filterThreshold
    );

    // Sort results
    filtered.sort((a, b) => {
      let aValue: number | string;
      let bValue: number | string;

      switch (sortBy.field) {
        case 'similarity':
          aValue = a.score;
          bValue = b.score;
          break;
        case 'id':
          aValue = String(a.id);
          bValue = String(b.id);
          break;
        case 'metadata':
          aValue = a.metadata?.rank || 0;
          bValue = b.metadata?.rank || 0;
          break;
        default:
          aValue = a.score;
          bValue = b.score;
      }

      if (typeof aValue === 'string' && typeof bValue === 'string') {
        return sortBy.order === 'asc'
          ? aValue.localeCompare(bValue)
          : bValue.localeCompare(aValue);
      }

      return sortBy.order === 'asc'
        ? (aValue as number) - (bValue as number)
        : (bValue as number) - (aValue as number);
    });

    return filtered;
  }, [results?.results, filterThreshold, sortBy]);

  const handlePageChange = (
    _event: React.MouseEvent<HTMLButtonElement> | null,
    newPage: number
  ) => {
    setPage(newPage);
    onPageChange(newPage + 1, pageSize);
  };

  const handlePageSizeChange = (
    event: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>
  ) => {
    const newPageSize = parseInt(event.target.value, 10);
    setPageSize(newPageSize);
    setPage(0);
    onPageChange(1, newPageSize);
  };

  const handleExport = (format: SearchExportOptions['format']) => {
    if (!onExport || !results) return;

    const options: SearchExportOptions = {
      format,
      includeVectors: true,
      includeMetadata: true,
      includeScores: true,
      maxResults: filteredAndSortedResults.length,
    };

    onExport(options);
    setExportMenuAnchor(null);
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch (err) {
      console.error('Failed to copy to clipboard:', err);
    }
  };

  const formatSimilarityScore = (score: number) => {
    return (score * 100).toFixed(1) + '%';
  };

  const getScoreColor = (score: number) => {
    if (score > 0.8) return 'success';
    if (score > 0.6) return 'warning';
    if (score > 0.4) return 'info';
    return 'default';
  };

  const renderListView = () => (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
      {filteredAndSortedResults.map((result, index) => (
        <Card key={`${result.id}-${index}`} variant="outlined">
          <CardContent>
            <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
              <Typography
                variant="h6"
                sx={{ flexGrow: 1, fontFamily: 'monospace' }}
              >
                {truncateText(String(result.id), 30)}
              </Typography>
              <Chip
                label={formatSimilarityScore(result.score)}
                color={getScoreColor(result.score)}
                size="small"
              />
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
              <Box>
                <Typography variant="body2" color="text.secondary">
                  Metadata:{' '}
                  {result.payload
                    ? Object.keys(result.payload).length > 0
                      ? `${Object.keys(result.payload).length} fields`
                      : 'None'
                    : 'None'}
                </Typography>
                {result.vector && (
                  <Typography variant="body2" color="text.secondary">
                    Vector: {result.vector.length} dimensions
                  </Typography>
                )}
              </Box>
              <Box sx={{ display: 'flex', gap: 1 }}>
                <IconButton size="small" onClick={() => onViewResult(result)}>
                  <ViewIcon fontSize="small" />
                </IconButton>
                <IconButton
                  size="small"
                  onClick={() => copyToClipboard(String(result.id))}
                >
                  <CopyIcon fontSize="small" />
                </IconButton>
              </Box>
            </Box>
          </CardContent>
        </Card>
      ))}
    </Box>
  );

  const renderGridView = () => (
    <Grid container spacing={2}>
      {filteredAndSortedResults.map((result, index) => (
        <Grid item xs={12} sm={6} md={4} lg={3} key={`${result.id}-${index}`}>
          <Card variant="outlined" sx={{ height: '100%' }}>
            <CardContent>
              <Typography
                variant="subtitle2"
                sx={{ fontFamily: 'monospace', mb: 1 }}
              >
                {truncateText(String(result.id), 20)}
              </Typography>
              <Chip
                label={formatSimilarityScore(result.score)}
                color={getScoreColor(result.score)}
                size="small"
                sx={{ mb: 2 }}
              />
              <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                {result.payload
                  ? Object.keys(result.payload).length > 0
                    ? `${Object.keys(result.payload).length} metadata fields`
                    : 'No metadata'
                  : 'No metadata'}
              </Typography>
              <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                <IconButton size="small" onClick={() => onViewResult(result)}>
                  <ViewIcon fontSize="small" />
                </IconButton>
                <IconButton
                  size="small"
                  onClick={() => copyToClipboard(String(result.id))}
                >
                  <CopyIcon fontSize="small" />
                </IconButton>
              </Box>
            </CardContent>
          </Card>
        </Grid>
      ))}
    </Grid>
  );

  const renderTableView = () => (
    <TableContainer>
      <Table>
        <TableHead>
          <TableRow>
            <TableCell>Vector ID</TableCell>
            <TableCell>Similarity Score</TableCell>
            <TableCell>Metadata</TableCell>
            <TableCell>Vector Dims</TableCell>
            <TableCell align="right">Actions</TableCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {filteredAndSortedResults.map((result, index) => (
            <TableRow key={`${result.id}-${index}`} hover>
              <TableCell>
                <Typography variant="body2" sx={{ fontFamily: 'monospace' }}>
                  {truncateText(String(result.id), 25)}
                </Typography>
              </TableCell>
              <TableCell>
                <Chip
                  label={formatSimilarityScore(result.score)}
                  color={getScoreColor(result.score)}
                  size="small"
                />
              </TableCell>
              <TableCell>
                <Typography variant="body2" color="text.secondary">
                  {result.payload
                    ? Object.keys(result.payload).length > 0
                      ? `${Object.keys(result.payload).length} fields`
                      : 'None'
                    : 'None'}
                </Typography>
              </TableCell>
              <TableCell>
                <Typography variant="body2" color="text.secondary">
                  {result.vector ? formatNumber(result.vector.length) : 'N/A'}
                </Typography>
              </TableCell>
              <TableCell align="right">
                <Box sx={{ display: 'flex', gap: 0.5 }}>
                  <Tooltip title="View Details">
                    <IconButton
                      size="small"
                      onClick={() => onViewResult(result)}
                    >
                      <ViewIcon fontSize="small" />
                    </IconButton>
                  </Tooltip>
                  <Tooltip title="Copy ID">
                    <IconButton
                      size="small"
                      onClick={() => copyToClipboard(String(result.id))}
                    >
                      <CopyIcon fontSize="small" />
                    </IconButton>
                  </Tooltip>
                </Box>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </TableContainer>
  );

  if (loading) {
    return (
      <Card>
        <CardContent>
          <Typography variant="h6" gutterBottom>
            Searching...
          </Typography>
          <LinearProgress />
        </CardContent>
      </Card>
    );
  }

  if (!results) {
    return (
      <Card>
        <CardContent>
          <Typography variant="body1" color="text.secondary">
            No search performed yet. Use the search forms above to find vectors.
          </Typography>
        </CardContent>
      </Card>
    );
  }

  if (results.results.length === 0) {
    return (
      <Card>
        <CardContent>
          <Alert severity="info">
            No results found for your search query. Try adjusting your search
            parameters or filters.
          </Alert>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardContent>
        {/* Header */}
        <Box sx={{ display: 'flex', alignItems: 'center', mb: 3 }}>
          <Typography variant="h6" sx={{ flexGrow: 1, fontWeight: 600 }}>
            Search Results ({formatNumber(filteredAndSortedResults.length)} of{' '}
            {formatNumber(results.total)})
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mr: 2 }}>
            {results.executionTime}ms
          </Typography>
        </Box>

        {/* Controls */}
        <Box
          sx={{
            display: 'flex',
            alignItems: 'center',
            gap: 2,
            mb: 3,
            flexWrap: 'wrap',
          }}
        >
          {/* View Mode */}
          <Box sx={{ display: 'flex', gap: 0.5 }}>
            <Tooltip title="List View">
              <IconButton
                size="small"
                onClick={() => setViewMode('list')}
                color={viewMode === 'list' ? 'primary' : 'default'}
              >
                <ListViewIcon />
              </IconButton>
            </Tooltip>
            <Tooltip title="Grid View">
              <IconButton
                size="small"
                onClick={() => setViewMode('grid')}
                color={viewMode === 'grid' ? 'primary' : 'default'}
              >
                <GridViewIcon />
              </IconButton>
            </Tooltip>
            <Tooltip title="Table View">
              <IconButton
                size="small"
                onClick={() => setViewMode('table')}
                color={viewMode === 'table' ? 'primary' : 'default'}
              >
                <TableViewIcon />
              </IconButton>
            </Tooltip>
          </Box>

          <Divider orientation="vertical" flexItem />

          {/* Sort */}
          <FormControl size="small" sx={{ minWidth: 150 }}>
            <InputLabel>Sort By</InputLabel>
            <Select
              value={`${sortBy.field}-${sortBy.order}`}
              onChange={e => {
                const [field, order] = e.target.value.split('-');
                setSortBy({
                  field: field as SearchSorting['field'],
                  order: order as SearchSorting['order'],
                });
              }}
              label="Sort By"
              startAdornment={<SortIcon sx={{ mr: 1 }} />}
            >
              <MenuItem value="similarity-desc">
                Similarity (High to Low)
              </MenuItem>
              <MenuItem value="similarity-asc">
                Similarity (Low to High)
              </MenuItem>
              <MenuItem value="id-asc">ID (A to Z)</MenuItem>
              <MenuItem value="id-desc">ID (Z to A)</MenuItem>
            </Select>
          </FormControl>

          {/* Filter */}
          <TextField
            size="small"
            label="Min Score"
            type="number"
            value={filterThreshold}
            onChange={e => setFilterThreshold(Number(e.target.value))}
            inputProps={{ min: 0, max: 1, step: 0.1 }}
            sx={{ width: 120 }}
            InputProps={{
              startAdornment: <FilterIcon sx={{ mr: 1 }} />,
            }}
          />

          <Box sx={{ flexGrow: 1 }} />

          {/* Export */}
          {onExport && (
            <>
              <Button
                variant="outlined"
                startIcon={<ExportIcon />}
                onClick={e => setExportMenuAnchor(e.currentTarget)}
                size="small"
              >
                Export
              </Button>
              <Menu
                anchorEl={exportMenuAnchor}
                open={Boolean(exportMenuAnchor)}
                onClose={() => setExportMenuAnchor(null)}
              >
                <MenuItem onClick={() => handleExport('json')}>
                  <ListItemIcon>
                    <ExportIcon fontSize="small" />
                  </ListItemIcon>
                  <ListItemText>JSON</ListItemText>
                </MenuItem>
                <MenuItem onClick={() => handleExport('csv')}>
                  <ListItemIcon>
                    <ExportIcon fontSize="small" />
                  </ListItemIcon>
                  <ListItemText>CSV</ListItemText>
                </MenuItem>
                <MenuItem onClick={() => handleExport('pdf')}>
                  <ListItemIcon>
                    <ExportIcon fontSize="small" />
                  </ListItemIcon>
                  <ListItemText>PDF</ListItemText>
                </MenuItem>
              </Menu>
            </>
          )}
        </Box>

        {/* Results */}
        <Box sx={{ mb: 3 }}>
          {viewMode === 'list' && renderListView()}
          {viewMode === 'grid' && renderGridView()}
          {viewMode === 'table' && renderTableView()}
        </Box>

        {/* Pagination */}
        <TablePagination
          component="div"
          count={results.total}
          page={page}
          onPageChange={handlePageChange}
          rowsPerPage={pageSize}
          onRowsPerPageChange={handlePageSizeChange}
          rowsPerPageOptions={[10, 25, 50, 100]}
        />
      </CardContent>
    </Card>
  );
};

export default SearchResults;
