import React, { useCallback } from 'react';
import {
  Box,
  Grid,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Chip,
  Slider,
  Typography,
  FormControlLabel,
  Switch,
  TextField,
  OutlinedInput,
  Paper,
  Divider,
} from '@mui/material';
import type { SelectChangeEvent } from '@mui/material';

import type {
  SemanticSearchFilters,
  SearchFiltersProps,
} from '../../types/semantic';
import type { NodeType, EdgeType } from '../../types/graph';

const ITEM_HEIGHT = 48;
const ITEM_PADDING_TOP = 8;
const MenuProps = {
  PaperProps: {
    style: {
      maxHeight: ITEM_HEIGHT * 4.5 + ITEM_PADDING_TOP,
      width: 250,
    },
  },
};

const SearchFilters: React.FC<SearchFiltersProps> = ({
  filters,
  onChange,
  availableNodeTypes,
  availableEdgeTypes,
  className,
}) => {
  // Handle node type changes
  const handleNodeTypesChange = useCallback((event: SelectChangeEvent<NodeType[]>) => {
    const value = event.target.value;
    onChange({
      ...filters,
      node_types: typeof value === 'string' ? value.split(',') as NodeType[] : value,
    });
  }, [filters, onChange]);

  // Handle edge type changes
  const handleEdgeTypesChange = useCallback((event: SelectChangeEvent<EdgeType[]>) => {
    const value = event.target.value;
    onChange({
      ...filters,
      edge_types: typeof value === 'string' ? value.split(',') as EdgeType[] : value,
    });
  }, [filters, onChange]);

  // Handle similarity threshold change
  const handleSimilarityThresholdChange = useCallback((_: Event, value: number | number[]) => {
    onChange({
      ...filters,
      similarity_threshold: Array.isArray(value) ? value[0] : value,
    });
  }, [filters, onChange]);

  // Handle max results change
  const handleMaxResultsChange = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
    const value = parseInt(event.target.value, 10);
    if (!isNaN(value) && value > 0) {
      onChange({
        ...filters,
        max_results: value,
      });
    }
  }, [filters, onChange]);

  // Handle date range changes
  const handleStartDateChange = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
    const date = event.target.value ? new Date(event.target.value) : undefined;
    onChange({
      ...filters,
      date_range: {
        ...filters.date_range,
        start: date,
      },
    });
  }, [filters, onChange]);

  const handleEndDateChange = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
    const date = event.target.value ? new Date(event.target.value) : undefined;
    onChange({
      ...filters,
      date_range: {
        ...filters.date_range,
        end: date,
      },
    });
  }, [filters, onChange]);

  // Handle boolean toggles
  const handleIncludeExplanationsChange = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
    onChange({
      ...filters,
      include_explanations: event.target.checked,
    });
  }, [filters, onChange]);

  const handleClusterResultsChange = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
    onChange({
      ...filters,
      cluster_results: event.target.checked,
    });
  }, [filters, onChange]);

  // Render node type chips
  const renderNodeTypeValue = (selected: NodeType[]) => (
    <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
      {selected.map((value) => (
        <Chip key={value} label={value} size="small" />
      ))}
    </Box>
  );

  // Render edge type chips
  const renderEdgeTypeValue = (selected: EdgeType[]) => (
    <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
      {selected.map((value) => (
        <Chip key={value} label={value} size="small" />
      ))}
    </Box>
  );

  return (
    <Paper
      variant="outlined"
      sx={{ p: 2 }}
      className={className}
    >
        <Typography variant="subtitle2" gutterBottom>
          Search Filters
        </Typography>
        
        <Grid container spacing={2}>
          {/* Node Types Filter */}
          <Grid item xs={12} sm={6}>
            <FormControl fullWidth size="small">
              <InputLabel>Node Types</InputLabel>
              <Select
                multiple
                value={filters.node_types}
                onChange={handleNodeTypesChange}
                input={<OutlinedInput label="Node Types" />}
                renderValue={renderNodeTypeValue}
                MenuProps={MenuProps}
              >
                {availableNodeTypes.map((type) => (
                  <MenuItem key={type} value={type}>
                    {type}
                  </MenuItem>
                ))}
              </Select>
            </FormControl>
          </Grid>

          {/* Edge Types Filter */}
          <Grid item xs={12} sm={6}>
            <FormControl fullWidth size="small">
              <InputLabel>Edge Types</InputLabel>
              <Select
                multiple
                value={filters.edge_types}
                onChange={handleEdgeTypesChange}
                input={<OutlinedInput label="Edge Types" />}
                renderValue={renderEdgeTypeValue}
                MenuProps={MenuProps}
              >
                {availableEdgeTypes.map((type) => (
                  <MenuItem key={type} value={type}>
                    {type}
                  </MenuItem>
                ))}
              </Select>
            </FormControl>
          </Grid>

          {/* Similarity Threshold */}
          <Grid item xs={12} sm={6}>
            <Typography variant="body2" gutterBottom>
              Similarity Threshold: {filters.similarity_threshold.toFixed(2)}
            </Typography>
            <Slider
              value={filters.similarity_threshold}
              onChange={handleSimilarityThresholdChange}
              min={0}
              max={1}
              step={0.01}
              marks={[
                { value: 0, label: '0%' },
                { value: 0.5, label: '50%' },
                { value: 1, label: '100%' },
              ]}
              valueLabelDisplay="auto"
              valueLabelFormat={(value) => `${Math.round(value * 100)}%`}
            />
          </Grid>

          {/* Max Results */}
          <Grid item xs={12} sm={6}>
            <TextField
              fullWidth
              size="small"
              label="Max Results"
              type="number"
              value={filters.max_results}
              onChange={handleMaxResultsChange}
              inputProps={{
                min: 1,
                max: 1000,
              }}
            />
          </Grid>

          {/* Date Range */}
          <Grid item xs={12}>
            <Typography variant="body2" gutterBottom>
              Date Range
            </Typography>
            <Grid container spacing={2}>
              <Grid item xs={12} sm={6}>
                <TextField
                  fullWidth
                  size="small"
                  label="Start Date"
                  type="date"
                  value={filters.date_range.start ? filters.date_range.start.toISOString().split('T')[0] : ''}
                  onChange={handleStartDateChange}
                  InputLabelProps={{
                    shrink: true,
                  }}
                />
              </Grid>
              <Grid item xs={12} sm={6}>
                <TextField
                  fullWidth
                  size="small"
                  label="End Date"
                  type="date"
                  value={filters.date_range.end ? filters.date_range.end.toISOString().split('T')[0] : ''}
                  onChange={handleEndDateChange}
                  InputLabelProps={{
                    shrink: true,
                  }}
                />
              </Grid>
            </Grid>
          </Grid>

          {/* Options */}
          <Grid item xs={12}>
            <Divider sx={{ my: 1 }} />
            <Typography variant="body2" gutterBottom>
              Options
            </Typography>
            <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
              <FormControlLabel
                control={
                  <Switch
                    checked={filters.include_explanations}
                    onChange={handleIncludeExplanationsChange}
                    size="small"
                  />
                }
                label="Include explanations"
              />
              <FormControlLabel
                control={
                  <Switch
                    checked={filters.cluster_results}
                    onChange={handleClusterResultsChange}
                    size="small"
                  />
                }
                label="Cluster results"
              />
            </Box>
          </Grid>
        </Grid>
    </Paper>
  );
};

export default SearchFilters;