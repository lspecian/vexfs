import React, { useState, useCallback } from 'react';
import {
  Box,
  Paper,
  Typography,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  TextField,
  Autocomplete,
  Chip,
  Grid,
  Card,
  CardContent,
  Switch,
  FormControlLabel,
  IconButton,
  Button,
  Divider,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  Alert,
} from '@mui/material';
import {
  FilterList as FilterIcon,
  Add as AddIcon,
  Remove as RemoveIcon,
  ExpandMore as ExpandMoreIcon,
  AccountTree as NodeIcon,
  Timeline as EdgeIcon,
} from '@mui/icons-material';

import type {
  NodeResponse,
  EdgeResponse,
  NodeType,
  EdgeType,
  NodeFilters,
  EdgeFilters,
} from '../../types/graph';
import type { QueryBuilderQuery } from './QueryBuilder';

export interface FilterBuilderProps {
  query: QueryBuilderQuery;
  nodes: NodeResponse[];
  edges: EdgeResponse[];
  onQueryUpdate: (updates: Partial<QueryBuilderQuery>) => void;
  disabled?: boolean;
}

interface PropertyFilter {
  id: string;
  property: string;
  operator: 'equals' | 'contains' | 'startsWith' | 'endsWith' | 'gt' | 'lt' | 'gte' | 'lte';
  value: string;
}

const NODE_TYPES: NodeType[] = ['File', 'Directory', 'Symlink', 'Device', 'Custom'];
const EDGE_TYPES: EdgeType[] = ['Contains', 'References', 'DependsOn', 'SimilarTo', 'Custom'];

const FILTER_OPERATORS = [
  { value: 'equals', label: 'Equals', description: 'Exact match' },
  { value: 'contains', label: 'Contains', description: 'Contains substring' },
  { value: 'startsWith', label: 'Starts With', description: 'Begins with text' },
  { value: 'endsWith', label: 'Ends With', description: 'Ends with text' },
  { value: 'gt', label: 'Greater Than', description: 'Numeric greater than' },
  { value: 'lt', label: 'Less Than', description: 'Numeric less than' },
  { value: 'gte', label: 'Greater or Equal', description: 'Numeric greater than or equal' },
  { value: 'lte', label: 'Less or Equal', description: 'Numeric less than or equal' },
];

export const FilterBuilder: React.FC<FilterBuilderProps> = ({
  query,
  nodes,
  edges,
  onQueryUpdate,
  disabled = false,
}) => {
  const [nodePropertyFilters, setNodePropertyFilters] = useState<PropertyFilter[]>([]);
  const [edgePropertyFilters, setEdgePropertyFilters] = useState<PropertyFilter[]>([]);
  const [showAdvancedFilters, setShowAdvancedFilters] = useState(false);

  // Get unique property names from nodes and edges
  const nodeProperties = React.useMemo(() => {
    const props = new Set<string>();
    nodes.forEach(node => {
      Object.keys(node.properties || {}).forEach(prop => props.add(prop));
    });
    return Array.from(props).sort();
  }, [nodes]);

  const edgeProperties = React.useMemo(() => {
    const props = new Set<string>();
    edges.forEach(edge => {
      Object.keys(edge.properties || {}).forEach(prop => props.add(prop));
    });
    return Array.from(props).sort();
  }, [edges]);

  const handleNodeTypeChange = useCallback((nodeType: NodeType | null) => {
    const nodeFilters: NodeFilters = {
      ...query.nodeFilters,
      node_type: nodeType || undefined,
    };
    onQueryUpdate({ nodeFilters });
  }, [query.nodeFilters, onQueryUpdate]);

  const handleEdgeTypeChange = useCallback((edgeType: EdgeType | null) => {
    const edgeFilters: EdgeFilters = {
      ...query.edgeFilters,
      edge_type: edgeType || undefined,
    };
    onQueryUpdate({ edgeFilters });
  }, [query.edgeFilters, onQueryUpdate]);

  const handleInodeNumberChange = useCallback((inodeNumber: string) => {
    const nodeFilters: NodeFilters = {
      ...query.nodeFilters,
      inode_number: inodeNumber ? parseInt(inodeNumber) : undefined,
    };
    onQueryUpdate({ nodeFilters });
  }, [query.nodeFilters, onQueryUpdate]);

  const handleWeightRangeChange = useCallback((field: 'weight_min' | 'weight_max', value: string) => {
    const edgeFilters: EdgeFilters = {
      ...query.edgeFilters,
      [field]: value ? parseFloat(value) : undefined,
    };
    onQueryUpdate({ edgeFilters });
  }, [query.edgeFilters, onQueryUpdate]);

  const handleDateRangeChange = useCallback((field: 'created_after' | 'created_before', value: string) => {
    const nodeFilters: NodeFilters = {
      ...query.nodeFilters,
      [field]: value || undefined,
    };
    onQueryUpdate({ nodeFilters });
  }, [query.nodeFilters, onQueryUpdate]);

  const addNodePropertyFilter = useCallback(() => {
    const newFilter: PropertyFilter = {
      id: `node-prop-${Date.now()}`,
      property: '',
      operator: 'equals',
      value: '',
    };
    setNodePropertyFilters(prev => [...prev, newFilter]);
  }, []);

  const addEdgePropertyFilter = useCallback(() => {
    const newFilter: PropertyFilter = {
      id: `edge-prop-${Date.now()}`,
      property: '',
      operator: 'equals',
      value: '',
    };
    setEdgePropertyFilters(prev => [...prev, newFilter]);
  }, []);

  const removeNodePropertyFilter = useCallback((filterId: string) => {
    setNodePropertyFilters(prev => prev.filter(f => f.id !== filterId));
  }, []);

  const removeEdgePropertyFilter = useCallback((filterId: string) => {
    setEdgePropertyFilters(prev => prev.filter(f => f.id !== filterId));
  }, []);

  const updateNodePropertyFilter = useCallback((filterId: string, updates: Partial<PropertyFilter>) => {
    setNodePropertyFilters(prev => prev.map(f => 
      f.id === filterId ? { ...f, ...updates } : f
    ));
  }, []);

  const updateEdgePropertyFilter = useCallback((filterId: string, updates: Partial<PropertyFilter>) => {
    setEdgePropertyFilters(prev => prev.map(f => 
      f.id === filterId ? { ...f, ...updates } : f
    ));
  }, []);

  // Apply property filters to query
  React.useEffect(() => {
    const nodeProps: Record<string, any> = {};
    nodePropertyFilters.forEach(filter => {
      if (filter.property && filter.value) {
        nodeProps[filter.property] = {
          operator: filter.operator,
          value: filter.value,
        };
      }
    });

    const edgeProps: Record<string, any> = {};
    edgePropertyFilters.forEach(filter => {
      if (filter.property && filter.value) {
        edgeProps[filter.property] = {
          operator: filter.operator,
          value: filter.value,
        };
      }
    });

    onQueryUpdate({
      nodeFilters: { ...query.nodeFilters, properties: Object.keys(nodeProps).length > 0 ? nodeProps : undefined },
      edgeFilters: { ...query.edgeFilters, properties: Object.keys(edgeProps).length > 0 ? edgeProps : undefined },
    });
  }, [nodePropertyFilters, edgePropertyFilters, query.nodeFilters, query.edgeFilters, onQueryUpdate]);

  const hasActiveFilters = !!(
    query.nodeFilters.node_type ||
    query.nodeFilters.inode_number ||
    query.nodeFilters.created_after ||
    query.nodeFilters.created_before ||
    query.edgeFilters.edge_type ||
    query.edgeFilters.weight_min ||
    query.edgeFilters.weight_max ||
    nodePropertyFilters.length > 0 ||
    edgePropertyFilters.length > 0
  );

  return (
    <Card>
      <CardContent>
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
          <Box sx={{ display: 'flex', alignItems: 'center' }}>
            <FilterIcon color="primary" sx={{ mr: 1 }} />
            <Typography variant="h6" component="div">
              Filters & Conditions
            </Typography>
          </Box>
          {hasActiveFilters && (
            <Chip label="Filters Active" color="primary" size="small" />
          )}
        </Box>

        <Grid container spacing={3}>
          {/* Node Filters */}
          <Grid item xs={12}>
            <Accordion>
              <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                <Box sx={{ display: 'flex', alignItems: 'center' }}>
                  <NodeIcon color="primary" sx={{ mr: 1 }} />
                  <Typography variant="subtitle1">Node Filters</Typography>
                  {(query.nodeFilters.node_type || query.nodeFilters.inode_number) && (
                    <Chip label="Active" color="primary" size="small" sx={{ ml: 1 }} />
                  )}
                </Box>
              </AccordionSummary>
              <AccordionDetails>
                <Grid container spacing={2}>
                  <Grid item xs={12} md={6}>
                    <FormControl fullWidth disabled={disabled}>
                      <InputLabel>Node Type</InputLabel>
                      <Select
                        value={query.nodeFilters.node_type || ''}
                        onChange={(e) => handleNodeTypeChange(e.target.value as NodeType || null)}
                        label="Node Type"
                      >
                        <MenuItem value="">
                          <em>Any Type</em>
                        </MenuItem>
                        {NODE_TYPES.map((type) => (
                          <MenuItem key={type} value={type}>
                            {type}
                          </MenuItem>
                        ))}
                      </Select>
                    </FormControl>
                  </Grid>

                  <Grid item xs={12} md={6}>
                    <TextField
                      label="Inode Number"
                      type="number"
                      value={query.nodeFilters.inode_number || ''}
                      onChange={(e) => handleInodeNumberChange(e.target.value)}
                      disabled={disabled}
                      fullWidth
                      helperText="Filter by specific inode number"
                    />
                  </Grid>

                  <Grid item xs={12} md={6}>
                    <TextField
                      label="Created After"
                      type="datetime-local"
                      value={query.nodeFilters.created_after || ''}
                      onChange={(e) => handleDateRangeChange('created_after', e.target.value)}
                      disabled={disabled}
                      fullWidth
                      InputLabelProps={{ shrink: true }}
                    />
                  </Grid>

                  <Grid item xs={12} md={6}>
                    <TextField
                      label="Created Before"
                      type="datetime-local"
                      value={query.nodeFilters.created_before || ''}
                      onChange={(e) => handleDateRangeChange('created_before', e.target.value)}
                      disabled={disabled}
                      fullWidth
                      InputLabelProps={{ shrink: true }}
                    />
                  </Grid>

                  {/* Node Property Filters */}
                  <Grid item xs={12}>
                    <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 1 }}>
                      <Typography variant="subtitle2">Property Filters</Typography>
                      <Button
                        startIcon={<AddIcon />}
                        onClick={addNodePropertyFilter}
                        disabled={disabled}
                        size="small"
                      >
                        Add Property Filter
                      </Button>
                    </Box>
                    {nodePropertyFilters.map((filter) => (
                      <Box key={filter.id} sx={{ display: 'flex', gap: 1, mb: 1, alignItems: 'center' }}>
                        <Autocomplete
                          value={filter.property}
                          onChange={(_, value) => updateNodePropertyFilter(filter.id, { property: value || '' })}
                          options={nodeProperties}
                          renderInput={(params) => (
                            <TextField {...params} label="Property" size="small" />
                          )}
                          disabled={disabled}
                          sx={{ minWidth: 150 }}
                        />
                        <FormControl size="small" sx={{ minWidth: 120 }}>
                          <InputLabel>Operator</InputLabel>
                          <Select
                            value={filter.operator}
                            onChange={(e) => updateNodePropertyFilter(filter.id, { operator: e.target.value as any })}
                            label="Operator"
                            disabled={disabled}
                          >
                            {FILTER_OPERATORS.map((op) => (
                              <MenuItem key={op.value} value={op.value}>
                                {op.label}
                              </MenuItem>
                            ))}
                          </Select>
                        </FormControl>
                        <TextField
                          label="Value"
                          value={filter.value}
                          onChange={(e) => updateNodePropertyFilter(filter.id, { value: e.target.value })}
                          disabled={disabled}
                          size="small"
                          sx={{ minWidth: 150 }}
                        />
                        <IconButton
                          onClick={() => removeNodePropertyFilter(filter.id)}
                          disabled={disabled}
                          size="small"
                          color="error"
                        >
                          <RemoveIcon />
                        </IconButton>
                      </Box>
                    ))}
                  </Grid>
                </Grid>
              </AccordionDetails>
            </Accordion>
          </Grid>

          {/* Edge Filters */}
          <Grid item xs={12}>
            <Accordion>
              <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                <Box sx={{ display: 'flex', alignItems: 'center' }}>
                  <EdgeIcon color="secondary" sx={{ mr: 1 }} />
                  <Typography variant="subtitle1">Edge Filters</Typography>
                  {(query.edgeFilters.edge_type || query.edgeFilters.weight_min || query.edgeFilters.weight_max) && (
                    <Chip label="Active" color="secondary" size="small" sx={{ ml: 1 }} />
                  )}
                </Box>
              </AccordionSummary>
              <AccordionDetails>
                <Grid container spacing={2}>
                  <Grid item xs={12} md={4}>
                    <FormControl fullWidth disabled={disabled}>
                      <InputLabel>Edge Type</InputLabel>
                      <Select
                        value={query.edgeFilters.edge_type || ''}
                        onChange={(e) => handleEdgeTypeChange(e.target.value as EdgeType || null)}
                        label="Edge Type"
                      >
                        <MenuItem value="">
                          <em>Any Type</em>
                        </MenuItem>
                        {EDGE_TYPES.map((type) => (
                          <MenuItem key={type} value={type}>
                            {type}
                          </MenuItem>
                        ))}
                      </Select>
                    </FormControl>
                  </Grid>

                  <Grid item xs={12} md={4}>
                    <TextField
                      label="Minimum Weight"
                      type="number"
                      value={query.edgeFilters.weight_min || ''}
                      onChange={(e) => handleWeightRangeChange('weight_min', e.target.value)}
                      disabled={disabled}
                      fullWidth
                      inputProps={{ step: 0.1 }}
                    />
                  </Grid>

                  <Grid item xs={12} md={4}>
                    <TextField
                      label="Maximum Weight"
                      type="number"
                      value={query.edgeFilters.weight_max || ''}
                      onChange={(e) => handleWeightRangeChange('weight_max', e.target.value)}
                      disabled={disabled}
                      fullWidth
                      inputProps={{ step: 0.1 }}
                    />
                  </Grid>

                  {/* Edge Property Filters */}
                  <Grid item xs={12}>
                    <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 1 }}>
                      <Typography variant="subtitle2">Property Filters</Typography>
                      <Button
                        startIcon={<AddIcon />}
                        onClick={addEdgePropertyFilter}
                        disabled={disabled}
                        size="small"
                      >
                        Add Property Filter
                      </Button>
                    </Box>
                    {edgePropertyFilters.map((filter) => (
                      <Box key={filter.id} sx={{ display: 'flex', gap: 1, mb: 1, alignItems: 'center' }}>
                        <Autocomplete
                          value={filter.property}
                          onChange={(_, value) => updateEdgePropertyFilter(filter.id, { property: value || '' })}
                          options={edgeProperties}
                          renderInput={(params) => (
                            <TextField {...params} label="Property" size="small" />
                          )}
                          disabled={disabled}
                          sx={{ minWidth: 150 }}
                        />
                        <FormControl size="small" sx={{ minWidth: 120 }}>
                          <InputLabel>Operator</InputLabel>
                          <Select
                            value={filter.operator}
                            onChange={(e) => updateEdgePropertyFilter(filter.id, { operator: e.target.value as any })}
                            label="Operator"
                            disabled={disabled}
                          >
                            {FILTER_OPERATORS.map((op) => (
                              <MenuItem key={op.value} value={op.value}>
                                {op.label}
                              </MenuItem>
                            ))}
                          </Select>
                        </FormControl>
                        <TextField
                          label="Value"
                          value={filter.value}
                          onChange={(e) => updateEdgePropertyFilter(filter.id, { value: e.target.value })}
                          disabled={disabled}
                          size="small"
                          sx={{ minWidth: 150 }}
                        />
                        <IconButton
                          onClick={() => removeEdgePropertyFilter(filter.id)}
                          disabled={disabled}
                          size="small"
                          color="error"
                        >
                          <RemoveIcon />
                        </IconButton>
                      </Box>
                    ))}
                  </Grid>
                </Grid>
              </AccordionDetails>
            </Accordion>
          </Grid>
        </Grid>

        {/* Filter Summary */}
        {hasActiveFilters && (
          <Box sx={{ mt: 3, p: 2, bgcolor: 'background.default', borderRadius: 1 }}>
            <Typography variant="subtitle2" gutterBottom>
              Active Filters Summary
            </Typography>
            <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
              {query.nodeFilters.node_type && (
                <Chip label={`Node Type: ${query.nodeFilters.node_type}`} size="small" color="primary" />
              )}
              {query.nodeFilters.inode_number && (
                <Chip label={`Inode: ${query.nodeFilters.inode_number}`} size="small" color="primary" />
              )}
              {query.edgeFilters.edge_type && (
                <Chip label={`Edge Type: ${query.edgeFilters.edge_type}`} size="small" color="secondary" />
              )}
              {query.edgeFilters.weight_min && (
                <Chip label={`Min Weight: ${query.edgeFilters.weight_min}`} size="small" color="secondary" />
              )}
              {query.edgeFilters.weight_max && (
                <Chip label={`Max Weight: ${query.edgeFilters.weight_max}`} size="small" color="secondary" />
              )}
              {nodePropertyFilters.length > 0 && (
                <Chip label={`${nodePropertyFilters.length} Node Property Filter(s)`} size="small" color="primary" />
              )}
              {edgePropertyFilters.length > 0 && (
                <Chip label={`${edgePropertyFilters.length} Edge Property Filter(s)`} size="small" color="secondary" />
              )}
            </Box>
          </Box>
        )}
      </CardContent>
    </Card>
  );
};

export default FilterBuilder;