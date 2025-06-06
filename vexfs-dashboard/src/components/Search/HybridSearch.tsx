import React, { useState, useCallback } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Slider,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Button,
  Alert,
  Divider,
  Switch,
  FormControlLabel,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  Chip,
} from '@mui/material';
import {
  Search as SearchIcon,
  ExpandMore as ExpandMoreIcon,
  Tune as TuneIcon,
  Psychology as PsychologyIcon,
  FilterList as FilterIcon,
} from '@mui/icons-material';
import { useForm, Controller } from 'react-hook-form';
import VectorSimilaritySearch from './VectorSimilaritySearch';
import MetadataFilterSearch from './MetadataFilterSearch';
import type {
  HybridSearchQuery,
  VectorSimilarityQuery,
  MetadataFilterQuery,
  VexFSCollection,
  VexFSPoint,
  CollectionSchema,
} from '../../types';

interface HybridSearchProps {
  collections: VexFSCollection[];
  selectedCollection: string | null;
  onCollectionChange: (collectionId: string) => void;
  onSearch: (query: HybridSearchQuery) => void;
  vectors: VexFSPoint[];
  schema?: CollectionSchema;
  loading?: boolean;
  onTextToVector?: (text: string) => Promise<number[] | null>;
}

const HybridSearch: React.FC<HybridSearchProps> = ({
  collections,
  selectedCollection,
  onCollectionChange,
  onSearch,
  vectors,
  schema,
  loading = false,
  onTextToVector,
}) => {
  const [vectorQuery, setVectorQuery] = useState<VectorSimilarityQuery>({
    k: 10,
    threshold: 0.8,
    distanceMetric: 'cosine',
  });
  const [metadataQuery, setMetadataQuery] = useState<MetadataFilterQuery>({
    conditions: [],
    logicalOperator: 'AND',
  });
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const { control, handleSubmit, watch } = useForm<{
    vectorWeight: number;
    metadataWeight: number;
    fusionMethod: 'rrf' | 'weighted_sum' | 'max_score';
  }>({
    defaultValues: {
      vectorWeight: 0.7,
      metadataWeight: 0.3,
      fusionMethod: 'weighted_sum',
    },
  });

  const watchedValues = watch();

  const handleVectorSearch = useCallback((query: VectorSimilarityQuery) => {
    setVectorQuery(query);
  }, []);

  const handleMetadataSearch = useCallback((query: MetadataFilterQuery) => {
    setMetadataQuery(query);
  }, []);

  const onSubmit = useCallback(
    (data: {
      vectorWeight: number;
      metadataWeight: number;
      fusionMethod: 'rrf' | 'weighted_sum' | 'max_score';
    }) => {
      if (!selectedCollection) {
        setError('Please select a collection');
        return;
      }

      // Validate vector query
      if (
        !vectorQuery.vector &&
        !vectorQuery.vectorId &&
        !vectorQuery.textQuery
      ) {
        setError('Please provide a vector input for similarity search');
        return;
      }

      // Validate metadata query
      const validMetadataConditions = metadataQuery.conditions.filter(
        c => c.field && c.operator
      );

      if (validMetadataConditions.length === 0) {
        setError('Please add at least one metadata filter condition');
        return;
      }

      // Validate weights
      const totalWeight = data.vectorWeight + data.metadataWeight;
      if (Math.abs(totalWeight - 1) > 0.01) {
        setError('Vector and metadata weights must sum to 1.0');
        return;
      }

      setError(null);

      const hybridQuery: HybridSearchQuery = {
        vectorQuery,
        metadataQuery: {
          ...metadataQuery,
          conditions: validMetadataConditions,
        },
        vectorWeight: data.vectorWeight,
        metadataWeight: data.metadataWeight,
        fusionMethod: data.fusionMethod,
      };

      onSearch(hybridQuery);
    },
    [selectedCollection, vectorQuery, metadataQuery, onSearch]
  );

  const selectedCollectionData = collections.find(
    c => c.id === selectedCollection
  );

  return (
    <Card>
      <CardContent>
        <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
          <PsychologyIcon sx={{ mr: 1 }} />
          <Typography variant="h6" sx={{ flexGrow: 1, fontWeight: 600 }}>
            Hybrid Search (Vector + Metadata)
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

        <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
          Combine vector similarity search with metadata filtering for more
          precise results. Adjust the weights to balance between semantic
          similarity and metadata relevance.
        </Typography>

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
                {collection.pointsCount} vectors)
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
          {/* Fusion Configuration */}
          <Card variant="outlined" sx={{ mb: 3 }}>
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
                <TuneIcon sx={{ mr: 1 }} />
                <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
                  Fusion Configuration
                </Typography>
              </Box>

              <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
                {/* Weight Sliders */}
                <Box>
                  <Typography
                    variant="body2"
                    color="text.secondary"
                    sx={{ mb: 1 }}
                  >
                    Vector Similarity Weight:{' '}
                    {(watchedValues.vectorWeight * 100).toFixed(0)}%
                  </Typography>
                  <Controller
                    name="vectorWeight"
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
                        valueLabelFormat={value =>
                          `${(value * 100).toFixed(0)}%`
                        }
                        onChange={(_, value) => {
                          field.onChange(value);
                          // Auto-adjust metadata weight
                          const metadataWeight = 1 - (value as number);
                          // Update metadata weight in form
                          // Note: This would need proper form integration
                        }}
                      />
                    )}
                  />
                </Box>

                <Box>
                  <Typography
                    variant="body2"
                    color="text.secondary"
                    sx={{ mb: 1 }}
                  >
                    Metadata Relevance Weight:{' '}
                    {(watchedValues.metadataWeight * 100).toFixed(0)}%
                  </Typography>
                  <Controller
                    name="metadataWeight"
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
                        valueLabelFormat={value =>
                          `${(value * 100).toFixed(0)}%`
                        }
                        onChange={(_, value) => {
                          field.onChange(value);
                          // Auto-adjust vector weight
                          const vectorWeight = 1 - (value as number);
                          // Update vector weight in form
                          // Note: This would need proper form integration
                        }}
                      />
                    )}
                  />
                </Box>

                {/* Weight Balance Indicator */}
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                  <Typography variant="body2">Balance:</Typography>
                  <Chip
                    label={`${(watchedValues.vectorWeight * 100).toFixed(0)}% Vector`}
                    color="primary"
                    size="small"
                  />
                  <Typography variant="body2">+</Typography>
                  <Chip
                    label={`${(watchedValues.metadataWeight * 100).toFixed(0)}% Metadata`}
                    color="secondary"
                    size="small"
                  />
                  <Typography variant="body2">=</Typography>
                  <Chip
                    label={`${((watchedValues.vectorWeight + watchedValues.metadataWeight) * 100).toFixed(0)}%`}
                    color={
                      Math.abs(
                        watchedValues.vectorWeight +
                          watchedValues.metadataWeight -
                          1
                      ) < 0.01
                        ? 'success'
                        : 'error'
                    }
                    size="small"
                  />
                </Box>

                {showAdvanced && (
                  <Controller
                    name="fusionMethod"
                    control={control}
                    render={({ field }) => (
                      <FormControl size="small" sx={{ minWidth: 200 }}>
                        <InputLabel>Fusion Method</InputLabel>
                        <Select {...field} label="Fusion Method">
                          <MenuItem value="weighted_sum">Weighted Sum</MenuItem>
                          <MenuItem value="rrf">
                            Reciprocal Rank Fusion
                          </MenuItem>
                          <MenuItem value="max_score">Maximum Score</MenuItem>
                        </Select>
                      </FormControl>
                    )}
                  />
                )}
              </Box>
            </CardContent>
          </Card>

          {/* Vector Similarity Search */}
          <Accordion defaultExpanded>
            <AccordionSummary expandIcon={<ExpandMoreIcon />}>
              <Box sx={{ display: 'flex', alignItems: 'center' }}>
                <PsychologyIcon sx={{ mr: 1 }} />
                <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
                  Vector Similarity Search
                </Typography>
                <Chip
                  label={`${(watchedValues.vectorWeight * 100).toFixed(0)}% weight`}
                  color="primary"
                  size="small"
                  sx={{ ml: 2 }}
                />
              </Box>
            </AccordionSummary>
            <AccordionDetails>
              <VectorSimilaritySearch
                collections={collections}
                selectedCollection={selectedCollection}
                onCollectionChange={onCollectionChange}
                onSearch={handleVectorSearch}
                vectors={vectors}
                loading={loading}
                onTextToVector={onTextToVector}
              />
            </AccordionDetails>
          </Accordion>

          {/* Metadata Filter Search */}
          <Accordion defaultExpanded sx={{ mt: 2 }}>
            <AccordionSummary expandIcon={<ExpandMoreIcon />}>
              <Box sx={{ display: 'flex', alignItems: 'center' }}>
                <FilterIcon sx={{ mr: 1 }} />
                <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
                  Metadata Filter Search
                </Typography>
                <Chip
                  label={`${(watchedValues.metadataWeight * 100).toFixed(0)}% weight`}
                  color="secondary"
                  size="small"
                  sx={{ ml: 2 }}
                />
              </Box>
            </AccordionSummary>
            <AccordionDetails>
              <MetadataFilterSearch
                collections={collections}
                selectedCollection={selectedCollection}
                onCollectionChange={onCollectionChange}
                onSearch={handleMetadataSearch}
                schema={schema}
                loading={loading}
              />
            </AccordionDetails>
          </Accordion>

          <Divider sx={{ my: 3 }} />

          {/* Search Button */}
          <Box sx={{ display: 'flex', justifyContent: 'center' }}>
            <Button
              type="submit"
              variant="contained"
              size="large"
              startIcon={<SearchIcon />}
              disabled={loading || !selectedCollection}
              sx={{ borderRadius: 2, px: 4 }}
            >
              {loading ? 'Searching...' : 'Execute Hybrid Search'}
            </Button>
          </Box>
        </form>

        {/* Help Text */}
        {showAdvanced && (
          <Alert severity="info" sx={{ mt: 3 }}>
            <Typography variant="body2">
              <strong>Fusion Methods:</strong>
              <br />• <strong>Weighted Sum:</strong> Combines scores using the
              specified weights
              <br />• <strong>Reciprocal Rank Fusion:</strong> Ranks results
              from both searches and combines rankings
              <br />• <strong>Maximum Score:</strong> Takes the highest score
              from either search method
            </Typography>
          </Alert>
        )}
      </CardContent>
    </Card>
  );
};

export default HybridSearch;
