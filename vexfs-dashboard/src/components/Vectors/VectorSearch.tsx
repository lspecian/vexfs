import React, { useState } from 'react';
import {
  Card,
  CardContent,
  Typography,
  TextField,
  Button,
  Box,
  Tabs,
  Tab,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Slider,
  Alert,
  CircularProgress,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Chip,
  IconButton,
  Tooltip,
} from '@mui/material';
import {
  Search as SearchIcon,
  Upload as UploadIcon,
  Visibility as ViewIcon,
  ContentCopy as CopyIcon,
} from '@mui/icons-material';
import { useForm, Controller } from 'react-hook-form';
import { formatNumber, truncateText } from '../../utils';
import type {
  VexFSPoint,
  VexFSSearchResult,
  VectorSearchQuery,
} from '../../types';

interface VectorSearchProps {
  collectionName: string;
  vectors: VexFSPoint[];
  onSearch: (query: VectorSearchQuery) => Promise<VexFSSearchResult[]>;
  onViewVector: (vector: VexFSSearchResult) => void;
}

interface SearchFormData {
  manualVector: string;
  selectedVectorId: string;
  k: number;
  threshold: number;
  metadataFilter: string;
}

interface TabPanelProps {
  children?: React.ReactNode;
  index: number;
  value: number;
}

const TabPanel: React.FC<TabPanelProps> = ({ children, value, index }) => {
  return (
    <div hidden={value !== index}>
      {value === index && <Box sx={{ py: 2 }}>{children}</Box>}
    </div>
  );
};

const VectorSearch: React.FC<VectorSearchProps> = ({
  collectionName,
  vectors,
  onSearch,
  onViewVector,
}) => {
  const [activeTab, setActiveTab] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchResults, setSearchResults] = useState<VexFSSearchResult[]>([]);
  const [uploadedFile, setUploadedFile] = useState<File | null>(null);

  const {
    control,
    handleSubmit,
    formState: { errors },
  } = useForm<SearchFormData>({
    defaultValues: {
      manualVector: '',
      selectedVectorId: '',
      k: 10,
      threshold: 0.8,
      metadataFilter: '{}',
    },
  });

  const handleTabChange = (_event: React.SyntheticEvent, newValue: number) => {
    setActiveTab(newValue);
  };

  const validateVector = (vectorStr: string): number[] | null => {
    try {
      const parsed = JSON.parse(vectorStr);
      if (!Array.isArray(parsed)) return null;
      if (!parsed.every(val => typeof val === 'number')) return null;
      return parsed;
    } catch {
      return null;
    }
  };

  const validateMetadataFilter = (
    filterStr: string
  ): Record<string, unknown> | null => {
    try {
      const parsed = JSON.parse(filterStr);
      if (
        typeof parsed !== 'object' ||
        parsed === null ||
        Array.isArray(parsed)
      ) {
        return null;
      }
      return parsed;
    } catch {
      return null;
    }
  };

  const onFormSubmit = async (data: SearchFormData) => {
    setLoading(true);
    setError(null);

    try {
      let searchVector: number[] | undefined;
      let vectorId: string | number | undefined;

      // Determine search method based on active tab
      if (activeTab === 0) {
        // Manual vector input
        const parsed = validateVector(data.manualVector);
        if (!parsed) {
          throw new Error('Invalid vector format');
        }
        searchVector = parsed;
      } else if (activeTab === 1) {
        // Existing vector selection
        if (!data.selectedVectorId) {
          throw new Error('Please select a vector');
        }
        vectorId = data.selectedVectorId;
      }

      const metadataFilter = validateMetadataFilter(data.metadataFilter);
      if (!metadataFilter) {
        throw new Error('Invalid metadata filter format');
      }

      const query: VectorSearchQuery = {
        vector: searchVector,
        vectorId,
        k: data.k,
        threshold: data.threshold,
        filter:
          Object.keys(metadataFilter).length > 0 ? metadataFilter : undefined,
      };

      const results = await onSearch(query);
      setSearchResults(results);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed');
    } finally {
      setLoading(false);
    }
  };

  const handleFileUpload = async (
    event: React.ChangeEvent<HTMLInputElement>
  ) => {
    const file = event.target.files?.[0];
    if (!file) return;

    setUploadedFile(file);
    setLoading(true);
    setError(null);

    try {
      const text = await file.text();
      let searchVector: number[];

      if (file.name.endsWith('.json')) {
        const parsed = JSON.parse(text);
        if (Array.isArray(parsed)) {
          searchVector = parsed;
        } else if (parsed.vector && Array.isArray(parsed.vector)) {
          searchVector = parsed.vector;
        } else {
          throw new Error('Invalid JSON format');
        }
      } else {
        throw new Error('Only JSON files are supported for vector upload');
      }

      if (!searchVector.every(val => typeof val === 'number')) {
        throw new Error('All vector values must be numbers');
      }

      const query: VectorSearchQuery = {
        vector: searchVector,
        k: 10,
        threshold: 0.8,
      };

      const results = await onSearch(query);
      setSearchResults(results);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to process file');
    } finally {
      setLoading(false);
    }
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

  return (
    <Card>
      <CardContent>
        <Typography variant="h6" gutterBottom sx={{ fontWeight: 600 }}>
          Vector Search in {collectionName}
        </Typography>

        {error && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {error}
          </Alert>
        )}

        <Tabs value={activeTab} onChange={handleTabChange} sx={{ mb: 2 }}>
          <Tab label="Manual Input" />
          <Tab label="Select Vector" />
          <Tab label="Upload File" />
        </Tabs>

        <form onSubmit={handleSubmit(onFormSubmit)}>
          {/* Manual Vector Input Tab */}
          <TabPanel value={activeTab} index={0}>
            <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
              <Controller
                name="manualVector"
                control={control}
                rules={{
                  required: activeTab === 0 && 'Vector data is required',
                  validate: value => {
                    if (activeTab !== 0) return true;
                    const parsed = validateVector(value);
                    return parsed !== null || 'Invalid vector format';
                  },
                }}
                render={({ field }) => (
                  <TextField
                    {...field}
                    label="Vector Data"
                    placeholder="[0.1, 0.2, 0.3, ...]"
                    multiline
                    rows={4}
                    fullWidth
                    error={!!errors.manualVector}
                    helperText={
                      errors.manualVector?.message ||
                      'JSON array of numbers representing the query vector'
                    }
                  />
                )}
              />
            </Box>
          </TabPanel>

          {/* Select Existing Vector Tab */}
          <TabPanel value={activeTab} index={1}>
            <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
              <Controller
                name="selectedVectorId"
                control={control}
                rules={{
                  required: activeTab === 1 && 'Please select a vector',
                }}
                render={({ field }) => (
                  <FormControl fullWidth error={!!errors.selectedVectorId}>
                    <InputLabel>Select Vector</InputLabel>
                    <Select {...field} label="Select Vector">
                      {vectors.map(vector => (
                        <MenuItem key={vector.id} value={vector.id}>
                          {truncateText(String(vector.id), 50)} (
                          {formatNumber(vector.vector.length)} dims)
                        </MenuItem>
                      ))}
                    </Select>
                  </FormControl>
                )}
              />
            </Box>
          </TabPanel>

          {/* File Upload Tab */}
          <TabPanel value={activeTab} index={2}>
            <Box
              sx={{
                border: '2px dashed',
                borderColor: 'grey.300',
                borderRadius: 2,
                p: 4,
                textAlign: 'center',
                cursor: 'pointer',
                '&:hover': {
                  borderColor: 'primary.main',
                  bgcolor: 'action.hover',
                },
              }}
              component="label"
            >
              <input
                type="file"
                accept=".json"
                onChange={handleFileUpload}
                style={{ display: 'none' }}
              />
              <UploadIcon sx={{ fontSize: 48, color: 'grey.400', mb: 2 }} />
              <Typography variant="h6" gutterBottom>
                Upload Vector File
              </Typography>
              <Typography variant="body2" color="text.secondary">
                JSON file containing vector array
              </Typography>
              {uploadedFile && (
                <Chip
                  label={uploadedFile.name}
                  sx={{ mt: 2 }}
                  onDelete={() => setUploadedFile(null)}
                />
              )}
            </Box>
          </TabPanel>

          {/* Search Parameters */}
          <Box sx={{ mt: 3, display: 'flex', flexDirection: 'column', gap: 2 }}>
            <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
              Search Parameters
            </Typography>

            <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
              <Controller
                name="k"
                control={control}
                render={({ field }) => (
                  <TextField
                    {...field}
                    label="Number of Results (k)"
                    type="number"
                    size="small"
                    sx={{ minWidth: 150 }}
                    inputProps={{ min: 1, max: 100 }}
                  />
                )}
              />

              <Box sx={{ minWidth: 200 }}>
                <Typography variant="caption" color="text.secondary">
                  Similarity Threshold
                </Typography>
                <Controller
                  name="threshold"
                  control={control}
                  render={({ field }) => (
                    <Slider
                      {...field}
                      min={0}
                      max={1}
                      step={0.1}
                      marks
                      valueLabelDisplay="auto"
                      valueLabelFormat={value => `${(value * 100).toFixed(0)}%`}
                    />
                  )}
                />
              </Box>
            </Box>

            <Controller
              name="metadataFilter"
              control={control}
              rules={{
                validate: value => {
                  const parsed = validateMetadataFilter(value);
                  return parsed !== null || 'Invalid JSON format';
                },
              }}
              render={({ field }) => (
                <TextField
                  {...field}
                  label="Metadata Filter (JSON)"
                  placeholder='{"category": "example", "status": "active"}'
                  size="small"
                  fullWidth
                  error={!!errors.metadataFilter}
                  helperText={
                    errors.metadataFilter?.message ||
                    'Optional JSON object to filter results by metadata'
                  }
                />
              )}
            />
          </Box>

          {/* Search Button */}
          {activeTab !== 2 && (
            <Box sx={{ mt: 3 }}>
              <Button
                type="submit"
                variant="contained"
                startIcon={<SearchIcon />}
                disabled={loading}
                sx={{ borderRadius: 2 }}
              >
                {loading ? 'Searching...' : 'Search Vectors'}
              </Button>
            </Box>
          )}
        </form>

        {/* Loading State */}
        {loading && (
          <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
            <CircularProgress />
          </Box>
        )}

        {/* Search Results */}
        {searchResults.length > 0 && (
          <Box sx={{ mt: 4 }}>
            <Typography variant="h6" gutterBottom sx={{ fontWeight: 600 }}>
              Search Results ({searchResults.length})
            </Typography>
            <TableContainer>
              <Table>
                <TableHead>
                  <TableRow>
                    <TableCell>Vector ID</TableCell>
                    <TableCell>Similarity Score</TableCell>
                    <TableCell>Metadata</TableCell>
                    <TableCell align="right">Actions</TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {searchResults.map((result, index) => (
                    <TableRow key={`${result.id}-${index}`} hover>
                      <TableCell>
                        <Typography
                          variant="body2"
                          sx={{ fontFamily: 'monospace' }}
                        >
                          {truncateText(String(result.id), 20)}
                        </Typography>
                      </TableCell>
                      <TableCell>
                        <Chip
                          label={formatSimilarityScore(result.score)}
                          color={
                            result.score > 0.8
                              ? 'success'
                              : result.score > 0.6
                                ? 'warning'
                                : 'default'
                          }
                          size="small"
                        />
                      </TableCell>
                      <TableCell>
                        <Typography variant="body2" color="text.secondary">
                          {result.payload
                            ? Object.keys(result.payload).length > 0
                              ? `${Object.keys(result.payload).length} fields`
                              : 'No metadata'
                            : 'No metadata'}
                        </Typography>
                      </TableCell>
                      <TableCell align="right">
                        <Box sx={{ display: 'flex', gap: 0.5 }}>
                          <Tooltip title="View Details">
                            <IconButton
                              size="small"
                              onClick={() => onViewVector(result)}
                            >
                              <ViewIcon fontSize="small" />
                            </IconButton>
                          </Tooltip>
                          <Tooltip title="Copy Vector ID">
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
          </Box>
        )}

        {/* Empty Results */}
        {!loading && searchResults.length === 0 && activeTab !== 2 && (
          <Box sx={{ textAlign: 'center', py: 4 }}>
            <Typography variant="body2" color="text.secondary">
              No search results yet. Enter a query vector and click search.
            </Typography>
          </Box>
        )}
      </CardContent>
    </Card>
  );
};

export default VectorSearch;
