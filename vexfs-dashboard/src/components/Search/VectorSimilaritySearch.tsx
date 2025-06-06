import React, { useState, useCallback } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  TextField,
  Button,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Slider,
  Alert,
  Tabs,
  Tab,
  Chip,
  IconButton,
  Tooltip,
  Switch,
  FormControlLabel,
  Autocomplete,
  Divider,
} from '@mui/material';
import {
  Search as SearchIcon,
  Upload as UploadIcon,
  Clear as ClearIcon,
  Tune as TuneIcon,
  Psychology as PsychologyIcon,
} from '@mui/icons-material';
import { useForm, Controller } from 'react-hook-form';
import type {
  VectorSimilarityQuery,
  VexFSPoint,
  VexFSCollection,
} from '../../types';

interface VectorSimilaritySearchProps {
  collections: VexFSCollection[];
  selectedCollection: string | null;
  onCollectionChange: (collectionId: string) => void;
  onSearch: (query: VectorSimilarityQuery) => void;
  vectors: VexFSPoint[];
  loading?: boolean;
  onTextToVector?: (text: string) => Promise<number[] | null>;
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

const VectorSimilaritySearch: React.FC<VectorSimilaritySearchProps> = ({
  collections,
  selectedCollection,
  onCollectionChange,
  onSearch,
  vectors,
  loading = false,
  onTextToVector,
}) => {
  const [activeTab, setActiveTab] = useState(0);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [uploadedFile, setUploadedFile] = useState<File | null>(null);
  const [textToVectorLoading, setTextToVectorLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const {
    control,
    handleSubmit,
    setValue,
    watch,
    formState: { errors },
    reset,
  } = useForm<VectorSimilarityQuery>({
    defaultValues: {
      k: 10,
      threshold: 0.8,
      distanceMetric: 'cosine',
      includeVectorIds: [],
      excludeVectorIds: [],
    },
  });

  const watchedValues = watch();

  const handleTabChange = (_event: React.SyntheticEvent, newValue: number) => {
    setActiveTab(newValue);
    setError(null);
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

  const handleFileUpload = async (
    event: React.ChangeEvent<HTMLInputElement>
  ) => {
    const file = event.target.files?.[0];
    if (!file) return;

    setUploadedFile(file);
    setError(null);

    try {
      const text = await file.text();
      let vector: number[];

      if (file.name.endsWith('.json')) {
        const parsed = JSON.parse(text);
        if (Array.isArray(parsed)) {
          vector = parsed;
        } else if (parsed.vector && Array.isArray(parsed.vector)) {
          vector = parsed.vector;
        } else {
          throw new Error('Invalid JSON format');
        }
      } else {
        throw new Error('Only JSON files are supported');
      }

      if (!vector.every(val => typeof val === 'number')) {
        throw new Error('All vector values must be numbers');
      }

      setValue('vector', vector);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to process file');
      setUploadedFile(null);
    }
  };

  const handleTextToVector = async (text: string) => {
    if (!onTextToVector || !selectedCollection) return;

    setTextToVectorLoading(true);
    setError(null);

    try {
      const vector = await onTextToVector(text);
      if (vector) {
        setValue('vector', vector);
        setValue('textQuery', text);
      } else {
        setError('Failed to convert text to vector');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Text conversion failed');
    } finally {
      setTextToVectorLoading(false);
    }
  };

  const onSubmit = useCallback(
    (data: VectorSimilarityQuery) => {
      if (!selectedCollection) {
        setError('Please select a collection');
        return;
      }

      // Validate that we have a vector from one of the input methods
      if (
        !data.vector &&
        !data.vectorId &&
        !data.textQuery &&
        activeTab !== 2
      ) {
        setError('Please provide a vector input');
        return;
      }

      setError(null);
      onSearch(data);
    },
    [selectedCollection, onSearch, activeTab]
  );

  const clearForm = () => {
    reset();
    setUploadedFile(null);
    setError(null);
  };

  const selectedCollectionData = collections.find(
    c => c.id === selectedCollection
  );

  return (
    <Card>
      <CardContent>
        <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
          <Typography variant="h6" sx={{ flexGrow: 1, fontWeight: 600 }}>
            Vector Similarity Search
          </Typography>
          <FormControlLabel
            control={
              <Switch
                checked={showAdvanced}
                onChange={e => setShowAdvanced(e.target.checked)}
                size="small"
              />
            }
            label="Advanced"
          />
        </Box>

        {/* Collection Selection */}
        <FormControl fullWidth sx={{ mb: 3 }}>
          <InputLabel>Collection</InputLabel>
          <Select
            value={selectedCollection || ''}
            onChange={e => onCollectionChange(e.target.value)}
            label="Collection"
          >
            {collections.map(collection => (
              <MenuItem key={collection.id} value={collection.id}>
                {collection.name} ({collection.vectorSize}D,{' '}
                {collection.distance})
              </MenuItem>
            ))}
          </Select>
        </FormControl>

        {error && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {error}
          </Alert>
        )}

        <form onSubmit={handleSubmit(onSubmit)}>
          {/* Vector Input Methods */}
          <Tabs value={activeTab} onChange={handleTabChange} sx={{ mb: 2 }}>
            <Tab label="Manual Input" />
            <Tab label="Select Vector" />
            <Tab label="Upload File" />
            {onTextToVector && <Tab label="Text to Vector" />}
          </Tabs>

          {/* Manual Vector Input */}
          <TabPanel value={activeTab} index={0}>
            <Controller
              name="vector"
              control={control}
              rules={{
                validate: value => {
                  if (activeTab !== 0) return true;
                  if (!value || value.length === 0) {
                    return 'Vector is required';
                  }
                  if (
                    selectedCollectionData &&
                    value.length !== selectedCollectionData.vectorSize
                  ) {
                    return `Vector must have ${selectedCollectionData.vectorSize} dimensions`;
                  }
                  return true;
                },
              }}
              render={({ field }) => (
                <TextField
                  {...field}
                  value={field.value ? JSON.stringify(field.value) : ''}
                  onChange={e => {
                    const vector = validateVector(e.target.value);
                    field.onChange(vector);
                  }}
                  label="Vector Data"
                  placeholder="[0.1, 0.2, 0.3, ...]"
                  multiline
                  rows={4}
                  fullWidth
                  error={!!errors.vector}
                  helperText={
                    errors.vector?.message ||
                    `JSON array of ${selectedCollectionData?.vectorSize || 'N'} numbers`
                  }
                />
              )}
            />
          </TabPanel>

          {/* Select Existing Vector */}
          <TabPanel value={activeTab} index={1}>
            <Controller
              name="vectorId"
              control={control}
              rules={{
                required: activeTab === 1 && 'Please select a vector',
              }}
              render={({ field }) => (
                <Autocomplete
                  {...field}
                  options={vectors}
                  getOptionLabel={option =>
                    `${option.id} (${option.vector.length}D)`
                  }
                  renderInput={params => (
                    <TextField
                      {...params}
                      label="Select Vector"
                      error={!!errors.vectorId}
                      helperText={errors.vectorId?.message}
                    />
                  )}
                  onChange={(_, value) => field.onChange(value?.id)}
                  value={vectors.find(v => v.id === field.value) || null}
                />
              )}
            />
          </TabPanel>

          {/* File Upload */}
          <TabPanel value={activeTab} index={2}>
            <Box
              sx={{
                border: '2px dashed',
                borderColor: uploadedFile ? 'success.main' : 'grey.300',
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
                  onDelete={() => {
                    setUploadedFile(null);
                    setValue('vector', undefined);
                  }}
                  color="success"
                />
              )}
            </Box>
          </TabPanel>

          {/* Text to Vector */}
          {onTextToVector && (
            <TabPanel value={activeTab} index={3}>
              <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                <Controller
                  name="textQuery"
                  control={control}
                  render={({ field }) => (
                    <TextField
                      {...field}
                      label="Text Query"
                      placeholder="Enter text to convert to vector..."
                      multiline
                      rows={3}
                      fullWidth
                      helperText="Text will be converted to a vector using the collection's embedding model"
                    />
                  )}
                />
                <Button
                  variant="outlined"
                  startIcon={<PsychologyIcon />}
                  onClick={() =>
                    watchedValues.textQuery &&
                    handleTextToVector(watchedValues.textQuery)
                  }
                  disabled={
                    !watchedValues.textQuery ||
                    textToVectorLoading ||
                    !selectedCollection
                  }
                  sx={{ alignSelf: 'flex-start' }}
                >
                  {textToVectorLoading ? 'Converting...' : 'Convert to Vector'}
                </Button>
              </Box>
            </TabPanel>
          )}

          <Divider sx={{ my: 3 }} />

          {/* Search Parameters */}
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
              <TuneIcon fontSize="small" />
              <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
                Search Parameters
              </Typography>
            </Box>

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
                    inputProps={{ min: 1, max: 1000 }}
                  />
                )}
              />

              {showAdvanced && (
                <Controller
                  name="distanceMetric"
                  control={control}
                  render={({ field }) => (
                    <FormControl size="small" sx={{ minWidth: 150 }}>
                      <InputLabel>Distance Metric</InputLabel>
                      <Select {...field} label="Distance Metric">
                        <MenuItem value="cosine">Cosine</MenuItem>
                        <MenuItem value="euclidean">Euclidean</MenuItem>
                        <MenuItem value="dot_product">Dot Product</MenuItem>
                      </Select>
                    </FormControl>
                  )}
                />
              )}
            </Box>

            <Box sx={{ minWidth: 200 }}>
              <Typography variant="caption" color="text.secondary">
                Similarity Threshold: {(watchedValues.threshold || 0.8) * 100}%
              </Typography>
              <Controller
                name="threshold"
                control={control}
                render={({ field }) => (
                  <Slider
                    {...field}
                    min={0}
                    max={1}
                    step={0.05}
                    marks={[
                      { value: 0, label: '0%' },
                      { value: 0.5, label: '50%' },
                      { value: 1, label: '100%' },
                    ]}
                    valueLabelDisplay="auto"
                    valueLabelFormat={value => `${(value * 100).toFixed(0)}%`}
                  />
                )}
              />
            </Box>

            {showAdvanced && (
              <>
                <Controller
                  name="metadataPreFilter"
                  control={control}
                  render={({ field }) => (
                    <TextField
                      {...field}
                      value={
                        field.value ? JSON.stringify(field.value, null, 2) : ''
                      }
                      onChange={e => {
                        try {
                          const parsed = JSON.parse(e.target.value || '{}');
                          field.onChange(parsed);
                        } catch {
                          // Invalid JSON, don't update
                        }
                      }}
                      label="Metadata Pre-filter (JSON)"
                      placeholder='{"category": "example"}'
                      size="small"
                      fullWidth
                      helperText="Filter vectors by metadata before similarity search"
                    />
                  )}
                />

                <Box sx={{ display: 'flex', gap: 2 }}>
                  <Controller
                    name="includeVectorIds"
                    control={control}
                    render={({ field }) => (
                      <Autocomplete
                        {...field}
                        multiple
                        options={vectors.map(v => String(v.id))}
                        renderInput={params => (
                          <TextField
                            {...params}
                            label="Include Vector IDs"
                            size="small"
                            helperText="Only search within these vectors"
                          />
                        )}
                        onChange={(_, value) => field.onChange(value)}
                        value={field.value || []}
                        sx={{ flex: 1 }}
                      />
                    )}
                  />

                  <Controller
                    name="excludeVectorIds"
                    control={control}
                    render={({ field }) => (
                      <Autocomplete
                        {...field}
                        multiple
                        options={vectors.map(v => String(v.id))}
                        renderInput={params => (
                          <TextField
                            {...params}
                            label="Exclude Vector IDs"
                            size="small"
                            helperText="Exclude these vectors from results"
                          />
                        )}
                        onChange={(_, value) => field.onChange(value)}
                        value={field.value || []}
                        sx={{ flex: 1 }}
                      />
                    )}
                  />
                </Box>
              </>
            )}
          </Box>

          {/* Action Buttons */}
          <Box sx={{ display: 'flex', gap: 2, mt: 3 }}>
            <Button
              type="submit"
              variant="contained"
              startIcon={<SearchIcon />}
              disabled={loading || !selectedCollection}
              sx={{ borderRadius: 2 }}
            >
              {loading ? 'Searching...' : 'Search Vectors'}
            </Button>
            <Tooltip title="Clear form">
              <IconButton onClick={clearForm} disabled={loading}>
                <ClearIcon />
              </IconButton>
            </Tooltip>
          </Box>
        </form>
      </CardContent>
    </Card>
  );
};

export default VectorSimilaritySearch;
