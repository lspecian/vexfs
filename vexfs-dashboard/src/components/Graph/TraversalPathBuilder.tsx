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
  Slider,
  Alert,
  Divider,
} from '@mui/material';
import {
  Timeline as PathIcon,
  PlayArrow as StartIcon,
  Stop as EndIcon,
  Settings as ConfigIcon,
} from '@mui/icons-material';

import type {
  NodeResponse,
  EdgeResponse,
  TraversalAlgorithm,
} from '../../types/graph';
import type { QueryBuilderQuery } from './QueryBuilder';

export interface TraversalPathBuilderProps {
  query: QueryBuilderQuery;
  nodes: NodeResponse[];
  edges: EdgeResponse[];
  onQueryUpdate: (updates: Partial<QueryBuilderQuery>) => void;
  disabled?: boolean;
}

const TRAVERSAL_ALGORITHMS: { value: TraversalAlgorithm; label: string; description: string }[] = [
  {
    value: 'BreadthFirstSearch',
    label: 'Breadth-First Search (BFS)',
    description: 'Explores nodes level by level, finding shortest paths in unweighted graphs',
  },
  {
    value: 'DepthFirstSearch',
    label: 'Depth-First Search (DFS)',
    description: 'Explores as far as possible along each branch before backtracking',
  },
  {
    value: 'Dijkstra',
    label: 'Dijkstra\'s Algorithm',
    description: 'Finds shortest paths in weighted graphs with non-negative edge weights',
  },
  {
    value: 'TopologicalSort',
    label: 'Topological Sort',
    description: 'Orders nodes in a directed acyclic graph based on dependencies',
  },
];

export const TraversalPathBuilder: React.FC<TraversalPathBuilderProps> = ({
  query,
  nodes,
  edges,
  onQueryUpdate,
  disabled = false,
}) => {
  const [showAdvanced, setShowAdvanced] = useState(false);

  const handleAlgorithmChange = useCallback((algorithm: TraversalAlgorithm) => {
    onQueryUpdate({ algorithm });
  }, [onQueryUpdate]);

  const handleStartNodeChange = useCallback((nodeId: string | null) => {
    onQueryUpdate({ startNode: nodeId || undefined });
  }, [onQueryUpdate]);

  const handleEndNodeChange = useCallback((nodeId: string | null) => {
    onQueryUpdate({ endNode: nodeId || undefined });
  }, [onQueryUpdate]);

  const handleMaxDepthChange = useCallback((maxDepth: number) => {
    onQueryUpdate({ maxDepth });
  }, [onQueryUpdate]);

  const handleMaxResultsChange = useCallback((maxResults: number) => {
    onQueryUpdate({ maxResults });
  }, [onQueryUpdate]);

  const handleTimeoutChange = useCallback((timeoutMs: number) => {
    onQueryUpdate({ timeoutMs });
  }, [onQueryUpdate]);

  const selectedAlgorithm = TRAVERSAL_ALGORITHMS.find(alg => alg.value === query.algorithm);
  const requiresEndNode = query.algorithm === 'Dijkstra';

  return (
    <Card>
      <CardContent>
        <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
          <PathIcon color="primary" sx={{ mr: 1 }} />
          <Typography variant="h6" component="div">
            Traversal Path Configuration
          </Typography>
        </Box>

        <Grid container spacing={3}>
          {/* Algorithm Selection */}
          <Grid item xs={12}>
            <FormControl fullWidth disabled={disabled}>
              <InputLabel>Traversal Algorithm</InputLabel>
              <Select
                value={query.algorithm}
                onChange={(e) => handleAlgorithmChange(e.target.value as TraversalAlgorithm)}
                label="Traversal Algorithm"
              >
                {TRAVERSAL_ALGORITHMS.map((algorithm) => (
                  <MenuItem key={algorithm.value} value={algorithm.value}>
                    <Box>
                      <Typography variant="body1">{algorithm.label}</Typography>
                      <Typography variant="caption" color="text.secondary">
                        {algorithm.description}
                      </Typography>
                    </Box>
                  </MenuItem>
                ))}
              </Select>
            </FormControl>
            {selectedAlgorithm && (
              <Alert severity="info" sx={{ mt: 1 }}>
                <Typography variant="body2">
                  <strong>{selectedAlgorithm.label}:</strong> {selectedAlgorithm.description}
                </Typography>
              </Alert>
            )}
          </Grid>

          {/* Starting Node */}
          <Grid item xs={12} md={6}>
            <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
              <StartIcon color="success" sx={{ mr: 1, fontSize: 20 }} />
              <Typography variant="subtitle2">Starting Node</Typography>
            </Box>
            <Autocomplete
              value={nodes.find(node => node.id === query.startNode) || null}
              onChange={(_, node) => handleStartNodeChange(node?.id || null)}
              options={nodes}
              getOptionLabel={(node) => `${node.id} (${node.node_type})`}
              renderOption={(props, node) => (
                <Box component="li" {...props}>
                  <Box>
                    <Typography variant="body2">{node.id}</Typography>
                    <Typography variant="caption" color="text.secondary">
                      Type: {node.node_type} • Inode: {node.inode_number}
                    </Typography>
                  </Box>
                </Box>
              )}
              renderInput={(params) => (
                <TextField
                  {...params}
                  label="Select starting node"
                  placeholder="Choose a node to start traversal"
                  error={!query.startNode}
                  helperText={!query.startNode ? "Starting node is required" : ""}
                />
              )}
              disabled={disabled}
              fullWidth
            />
          </Grid>

          {/* Ending Node (for algorithms that need it) */}
          <Grid item xs={12} md={6}>
            <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
              <EndIcon color="error" sx={{ mr: 1, fontSize: 20 }} />
              <Typography variant="subtitle2">
                Target Node {requiresEndNode && <Chip label="Required" size="small" color="warning" />}
              </Typography>
            </Box>
            <Autocomplete
              value={nodes.find(node => node.id === query.endNode) || null}
              onChange={(_, node) => handleEndNodeChange(node?.id || null)}
              options={nodes.filter(node => node.id !== query.startNode)}
              getOptionLabel={(node) => `${node.id} (${node.node_type})`}
              renderOption={(props, node) => (
                <Box component="li" {...props}>
                  <Box>
                    <Typography variant="body2">{node.id}</Typography>
                    <Typography variant="caption" color="text.secondary">
                      Type: {node.node_type} • Inode: {node.inode_number}
                    </Typography>
                  </Box>
                </Box>
              )}
              renderInput={(params) => (
                <TextField
                  {...params}
                  label="Select target node (optional)"
                  placeholder="Choose a target node"
                  error={requiresEndNode && !query.endNode}
                  helperText={
                    requiresEndNode && !query.endNode
                      ? "Target node is required for this algorithm"
                      : "Leave empty for open-ended traversal"
                  }
                />
              )}
              disabled={disabled}
              fullWidth
            />
          </Grid>

          {/* Traversal Limits */}
          <Grid item xs={12} md={6}>
            <Typography variant="subtitle2" gutterBottom>
              Maximum Depth: {query.maxDepth || 3}
            </Typography>
            <Slider
              value={query.maxDepth || 3}
              onChange={(_, value) => handleMaxDepthChange(value as number)}
              min={1}
              max={10}
              step={1}
              marks={[
                { value: 1, label: '1' },
                { value: 3, label: '3' },
                { value: 5, label: '5' },
                { value: 10, label: '10' },
              ]}
              disabled={disabled}
              valueLabelDisplay="auto"
            />
            <Typography variant="caption" color="text.secondary">
              Maximum number of hops from the starting node
            </Typography>
          </Grid>

          <Grid item xs={12} md={6}>
            <Typography variant="subtitle2" gutterBottom>
              Maximum Results: {query.maxResults || 100}
            </Typography>
            <Slider
              value={query.maxResults || 100}
              onChange={(_, value) => handleMaxResultsChange(value as number)}
              min={10}
              max={1000}
              step={10}
              marks={[
                { value: 10, label: '10' },
                { value: 100, label: '100' },
                { value: 500, label: '500' },
                { value: 1000, label: '1000' },
              ]}
              disabled={disabled}
              valueLabelDisplay="auto"
            />
            <Typography variant="caption" color="text.secondary">
              Maximum number of nodes to return
            </Typography>
          </Grid>

          {/* Advanced Settings */}
          <Grid item xs={12}>
            <FormControlLabel
              control={
                <Switch
                  checked={showAdvanced}
                  onChange={(e) => setShowAdvanced(e.target.checked)}
                  disabled={disabled}
                />
              }
              label="Show Advanced Settings"
            />
          </Grid>

          {showAdvanced && (
            <>
              <Grid item xs={12}>
                <Divider sx={{ my: 1 }} />
                <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
                  <ConfigIcon color="primary" sx={{ mr: 1 }} />
                  <Typography variant="subtitle1">Advanced Configuration</Typography>
                </Box>
              </Grid>

              <Grid item xs={12} md={6}>
                <TextField
                  label="Timeout (milliseconds)"
                  type="number"
                  value={query.timeoutMs || 30000}
                  onChange={(e) => handleTimeoutChange(parseInt(e.target.value) || 30000)}
                  disabled={disabled}
                  fullWidth
                  helperText="Maximum time to wait for query execution"
                  inputProps={{ min: 1000, max: 300000, step: 1000 }}
                />
              </Grid>

              <Grid item xs={12} md={6}>
                <TextField
                  label="Weight Threshold"
                  type="number"
                  value={query.weightThreshold || ''}
                  onChange={(e) => onQueryUpdate({ 
                    weightThreshold: e.target.value ? parseFloat(e.target.value) : undefined 
                  })}
                  disabled={disabled}
                  fullWidth
                  helperText="Minimum edge weight to consider (for weighted algorithms)"
                  inputProps={{ min: 0, step: 0.1 }}
                />
              </Grid>
            </>
          )}
        </Grid>

        {/* Path Summary */}
        <Box sx={{ mt: 3, p: 2, bgcolor: 'background.default', borderRadius: 1 }}>
          <Typography variant="subtitle2" gutterBottom>
            Traversal Summary
          </Typography>
          <Typography variant="body2" color="text.secondary">
            <strong>Algorithm:</strong> {selectedAlgorithm?.label || 'None selected'}
            <br />
            <strong>Start:</strong> {query.startNode || 'Not selected'}
            {query.endNode && (
              <>
                <br />
                <strong>Target:</strong> {query.endNode}
              </>
            )}
            <br />
            <strong>Limits:</strong> Max depth {query.maxDepth || 3}, Max results {query.maxResults || 100}
          </Typography>
        </Box>
      </CardContent>
    </Card>
  );
};

export default TraversalPathBuilder;