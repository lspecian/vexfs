import React, { useState } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  TextField,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Chip,
  Button,
  Grid,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  FormControlLabel,
  Checkbox,
  Slider,
} from '@mui/material';
import {
  ExpandMore as ExpandMoreIcon,
  FilterList as FilterIcon,
  Clear as ClearIcon,
} from '@mui/icons-material';
import { LocalizationProvider } from '@mui/x-date-pickers/LocalizationProvider';
import { AdapterDateFns } from '@mui/x-date-pickers/AdapterDateFns';
import { DatePicker } from '@mui/x-date-pickers/DatePicker';

interface SearchFiltersProps {
  onFiltersChange?: (filters: SearchFilters) => void;
  onClearFilters?: () => void;
  disabled?: boolean;
}

interface SearchFilters {
  nodeTypes: string[];
  edgeTypes: string[];
  propertyFilters: Record<string, any>;
  dateRange: {
    start: Date | null;
    end: Date | null;
  };
  relevanceThreshold: number;
  maxResults: number;
}

const SearchFilters: React.FC<SearchFiltersProps> = ({
  onFiltersChange,
  onClearFilters,
  disabled = false,
}) => {
  const [filters, setFilters] = useState<SearchFilters>({
    nodeTypes: [],
    edgeTypes: [],
    propertyFilters: {},
    dateRange: {
      start: null,
      end: null,
    },
    relevanceThreshold: 0.5,
    maxResults: 50,
  });

  const [expanded, setExpanded] = useState<string | false>('basic');

  const nodeTypeOptions = ['File', 'Directory', 'Symlink', 'Device', 'Custom'];
  const edgeTypeOptions = ['Contains', 'References', 'DependsOn', 'SimilarTo', 'Custom'];

  const handleFilterChange = (newFilters: Partial<SearchFilters>) => {
    const updatedFilters = { ...filters, ...newFilters };
    setFilters(updatedFilters);
    onFiltersChange?.(updatedFilters);
  };

  const handleNodeTypeChange = (nodeType: string, checked: boolean) => {
    const newNodeTypes = checked
      ? [...filters.nodeTypes, nodeType]
      : filters.nodeTypes.filter(type => type !== nodeType);
    
    handleFilterChange({ nodeTypes: newNodeTypes });
  };

  const handleEdgeTypeChange = (edgeType: string, checked: boolean) => {
    const newEdgeTypes = checked
      ? [...filters.edgeTypes, edgeType]
      : filters.edgeTypes.filter(type => type !== edgeType);
    
    handleFilterChange({ edgeTypes: newEdgeTypes });
  };

  const handleClearFilters = () => {
    const clearedFilters: SearchFilters = {
      nodeTypes: [],
      edgeTypes: [],
      propertyFilters: {},
      dateRange: {
        start: null,
        end: null,
      },
      relevanceThreshold: 0.5,
      maxResults: 50,
    };
    
    setFilters(clearedFilters);
    onClearFilters?.();
    onFiltersChange?.(clearedFilters);
  };

  const handleAccordionChange = (panel: string) => (
    event: React.SyntheticEvent,
    isExpanded: boolean
  ) => {
    setExpanded(isExpanded ? panel : false);
  };

  return (
    <LocalizationProvider dateAdapter={AdapterDateFns}>
      <Card>
        <CardContent>
          <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
            <Typography variant="h6" component="div" sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
              <FilterIcon />
              Search Filters
            </Typography>
            <Button
              variant="outlined"
              size="small"
              startIcon={<ClearIcon />}
              onClick={handleClearFilters}
              disabled={disabled}
            >
              Clear All
            </Button>
          </Box>

          {/* Basic Filters */}
          <Accordion expanded={expanded === 'basic'} onChange={handleAccordionChange('basic')}>
            <AccordionSummary expandIcon={<ExpandMoreIcon />}>
              <Typography>Basic Filters</Typography>
            </AccordionSummary>
            <AccordionDetails>
              <Grid container spacing={2}>
                <Grid item xs={12} sm={6}>
                  <Typography variant="subtitle2" gutterBottom>
                    Node Types
                  </Typography>
                  <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
                    {nodeTypeOptions.map(nodeType => (
                      <FormControlLabel
                        key={nodeType}
                        control={
                          <Checkbox
                            checked={filters.nodeTypes.includes(nodeType)}
                            onChange={(e) => handleNodeTypeChange(nodeType, e.target.checked)}
                            disabled={disabled}
                          />
                        }
                        label={nodeType}
                      />
                    ))}
                  </Box>
                </Grid>

                <Grid item xs={12} sm={6}>
                  <Typography variant="subtitle2" gutterBottom>
                    Edge Types
                  </Typography>
                  <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
                    {edgeTypeOptions.map(edgeType => (
                      <FormControlLabel
                        key={edgeType}
                        control={
                          <Checkbox
                            checked={filters.edgeTypes.includes(edgeType)}
                            onChange={(e) => handleEdgeTypeChange(edgeType, e.target.checked)}
                            disabled={disabled}
                          />
                        }
                        label={edgeType}
                      />
                    ))}
                  </Box>
                </Grid>

                <Grid item xs={12} sm={6}>
                  <Typography variant="subtitle2" gutterBottom>
                    Relevance Threshold
                  </Typography>
                  <Slider
                    value={filters.relevanceThreshold}
                    onChange={(_, value) => handleFilterChange({ relevanceThreshold: value as number })}
                    min={0}
                    max={1}
                    step={0.1}
                    marks
                    valueLabelDisplay="auto"
                    disabled={disabled}
                  />
                </Grid>

                <Grid item xs={12} sm={6}>
                  <TextField
                    label="Max Results"
                    type="number"
                    value={filters.maxResults}
                    onChange={(e) => handleFilterChange({ maxResults: parseInt(e.target.value) || 50 })}
                    inputProps={{ min: 1, max: 1000 }}
                    disabled={disabled}
                    fullWidth
                  />
                </Grid>
              </Grid>
            </AccordionDetails>
          </Accordion>

          {/* Date Range Filters */}
          <Accordion expanded={expanded === 'dates'} onChange={handleAccordionChange('dates')}>
            <AccordionSummary expandIcon={<ExpandMoreIcon />}>
              <Typography>Date Range</Typography>
            </AccordionSummary>
            <AccordionDetails>
              <Grid container spacing={2}>
                <Grid item xs={12} sm={6}>
                  <DatePicker
                    label="Start Date"
                    value={filters.dateRange.start}
                    onChange={(date) => handleFilterChange({
                      dateRange: { ...filters.dateRange, start: date }
                    })}
                    disabled={disabled}
                    slotProps={{ textField: { fullWidth: true } }}
                  />
                </Grid>
                <Grid item xs={12} sm={6}>
                  <DatePicker
                    label="End Date"
                    value={filters.dateRange.end}
                    onChange={(date) => handleFilterChange({
                      dateRange: { ...filters.dateRange, end: date }
                    })}
                    disabled={disabled}
                    slotProps={{ textField: { fullWidth: true } }}
                  />
                </Grid>
              </Grid>
            </AccordionDetails>
          </Accordion>

          {/* Property Filters */}
          <Accordion expanded={expanded === 'properties'} onChange={handleAccordionChange('properties')}>
            <AccordionSummary expandIcon={<ExpandMoreIcon />}>
              <Typography>Property Filters</Typography>
            </AccordionSummary>
            <AccordionDetails>
              <Grid container spacing={2}>
                <Grid item xs={12} sm={6}>
                  <TextField
                    label="Property Name"
                    placeholder="e.g., size, name, type"
                    disabled={disabled}
                    fullWidth
                  />
                </Grid>
                <Grid item xs={12} sm={6}>
                  <TextField
                    label="Property Value"
                    placeholder="e.g., > 1024, contains 'config'"
                    disabled={disabled}
                    fullWidth
                  />
                </Grid>
              </Grid>
            </AccordionDetails>
          </Accordion>

          {/* Active Filters Display */}
          {(filters.nodeTypes.length > 0 || filters.edgeTypes.length > 0) && (
            <Box sx={{ mt: 2 }}>
              <Typography variant="subtitle2" gutterBottom>
                Active Filters:
              </Typography>
              <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
                {filters.nodeTypes.map(nodeType => (
                  <Chip
                    key={`node-${nodeType}`}
                    label={`Node: ${nodeType}`}
                    onDelete={() => handleNodeTypeChange(nodeType, false)}
                    color="primary"
                    variant="outlined"
                    size="small"
                    disabled={disabled}
                  />
                ))}
                {filters.edgeTypes.map(edgeType => (
                  <Chip
                    key={`edge-${edgeType}`}
                    label={`Edge: ${edgeType}`}
                    onDelete={() => handleEdgeTypeChange(edgeType, false)}
                    color="secondary"
                    variant="outlined"
                    size="small"
                    disabled={disabled}
                  />
                ))}
              </Box>
            </Box>
          )}
        </CardContent>
      </Card>
    </LocalizationProvider>
  );
};

export default SearchFilters;