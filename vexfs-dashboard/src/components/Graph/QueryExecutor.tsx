import React, { useState, useCallback } from 'react';
import {
  Box,
  Paper,
  Typography,
  Button,
  Card,
  CardContent,
  Grid,
  Alert,
  Chip,
  LinearProgress,
  Divider,
  IconButton,
  Tooltip,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  FormControlLabel,
  Switch,
} from '@mui/material';
import {
  PlayArrow as ExecuteIcon,
  Visibility as PreviewIcon,
  Code as CodeIcon,
  Download as ExportIcon,
  Settings as SettingsIcon,
  Timer as TimerIcon,
  Memory as MemoryIcon,
  Speed as SpeedIcon,
} from '@mui/icons-material';

import type { QueryBuilderQuery } from './QueryBuilder';

export interface QueryExecutorProps {
  query: QueryBuilderQuery;
  onExecute: () => void;
  onPreview: () => void;
  isExecuting: boolean;
  isPreviewMode: boolean;
  disabled?: boolean;
}

export const QueryExecutor: React.FC<QueryExecutorProps> = ({
  query,
  onExecute,
  onPreview,
  isExecuting,
  isPreviewMode,
  disabled = false,
}) => {
  const [showQueryCode, setShowQueryCode] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [executionSettings, setExecutionSettings] = useState({
    enableProfiling: false,
    enableCaching: true,
    enableOptimization: true,
    verboseLogging: false,
  });

  const isQueryValid = !!(query.startNode && query.algorithm);
  const requiresEndNode = query.algorithm === 'Dijkstra';
  const isEndNodeValid = !requiresEndNode || !!query.endNode;

  const handleExecute = useCallback(() => {
    if (isQueryValid && isEndNodeValid) {
      onExecute();
    }
  }, [isQueryValid, isEndNodeValid, onExecute]);

  const handlePreview = useCallback(() => {
    if (isQueryValid && isEndNodeValid) {
      onPreview();
    }
  }, [isQueryValid, isEndNodeValid, onPreview]);

  const generateQueryCode = useCallback(() => {
    const queryObj = {
      algorithm: query.algorithm,
      start_node: query.startNode,
      ...(query.endNode && { end_node: query.endNode }),
      ...(query.maxDepth && { max_depth: query.maxDepth }),
      ...(query.maxResults && { max_results: query.maxResults }),
      ...(query.nodeFilters.node_type && { node_filter: query.nodeFilters.node_type }),
      ...(query.edgeFilters.edge_type && { edge_filter: query.edgeFilters.edge_type }),
      ...(query.weightThreshold && { weight_threshold: query.weightThreshold }),
      ...(query.timeoutMs && { timeout_ms: query.timeoutMs }),
    };

    return JSON.stringify(queryObj, null, 2);
  }, [query]);

  const exportQuery = useCallback(() => {
    const queryCode = generateQueryCode();
    const blob = new Blob([queryCode], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `vexgraph-query-${query.name.replace(/\s+/g, '-')}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, [generateQueryCode, query.name]);

  const getValidationErrors = useCallback(() => {
    const errors: string[] = [];
    
    if (!query.startNode) {
      errors.push('Starting node is required');
    }
    
    if (requiresEndNode && !query.endNode) {
      errors.push(`Target node is required for ${query.algorithm} algorithm`);
    }
    
    if (query.maxDepth && query.maxDepth < 1) {
      errors.push('Maximum depth must be at least 1');
    }
    
    if (query.maxResults && query.maxResults < 1) {
      errors.push('Maximum results must be at least 1');
    }
    
    if (query.timeoutMs && query.timeoutMs < 1000) {
      errors.push('Timeout must be at least 1000ms');
    }

    return errors;
  }, [query, requiresEndNode]);

  const validationErrors = getValidationErrors();
  const hasErrors = validationErrors.length > 0;

  return (
    <Card>
      <CardContent>
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
          <Typography variant="h6" component="div">
            Query Execution
          </Typography>
          <Box sx={{ display: 'flex', gap: 1 }}>
            <Tooltip title="View Query Code">
              <IconButton
                onClick={() => setShowQueryCode(true)}
                disabled={disabled}
                size="small"
              >
                <CodeIcon />
              </IconButton>
            </Tooltip>
            <Tooltip title="Export Query">
              <IconButton
                onClick={exportQuery}
                disabled={disabled}
                size="small"
              >
                <ExportIcon />
              </IconButton>
            </Tooltip>
            <Tooltip title="Execution Settings">
              <IconButton
                onClick={() => setShowSettings(true)}
                disabled={disabled}
                size="small"
              >
                <SettingsIcon />
              </IconButton>
            </Tooltip>
          </Box>
        </Box>

        {/* Validation Errors */}
        {hasErrors && (
          <Alert severity="error" sx={{ mb: 2 }}>
            <Typography variant="subtitle2">Query Validation Errors</Typography>
            <ul style={{ margin: '8px 0', paddingLeft: '20px' }}>
              {validationErrors.map((error, index) => (
                <li key={index}>
                  <Typography variant="body2">{error}</Typography>
                </li>
              ))}
            </ul>
          </Alert>
        )}

        {/* Execution Progress */}
        {(isExecuting || isPreviewMode) && (
          <Box sx={{ mb: 2 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
              <Typography variant="body2" color="text.secondary">
                {isPreviewMode ? 'Generating preview...' : 'Executing query...'}
              </Typography>
            </Box>
            <LinearProgress />
          </Box>
        )}

        <Grid container spacing={3}>
          {/* Query Summary */}
          <Grid item xs={12} md={8}>
            <Box sx={{ p: 2, bgcolor: 'background.default', borderRadius: 1 }}>
              <Typography variant="subtitle2" gutterBottom>
                Query Summary
              </Typography>
              <Grid container spacing={2}>
                <Grid item xs={6}>
                  <Typography variant="body2" color="text.secondary">
                    <strong>Algorithm:</strong> {query.algorithm}
                  </Typography>
                </Grid>
                <Grid item xs={6}>
                  <Typography variant="body2" color="text.secondary">
                    <strong>Start Node:</strong> {query.startNode || 'Not selected'}
                  </Typography>
                </Grid>
                {query.endNode && (
                  <Grid item xs={6}>
                    <Typography variant="body2" color="text.secondary">
                      <strong>Target Node:</strong> {query.endNode}
                    </Typography>
                  </Grid>
                )}
                <Grid item xs={6}>
                  <Typography variant="body2" color="text.secondary">
                    <strong>Max Depth:</strong> {query.maxDepth || 3}
                  </Typography>
                </Grid>
                <Grid item xs={6}>
                  <Typography variant="body2" color="text.secondary">
                    <strong>Max Results:</strong> {query.maxResults || 100}
                  </Typography>
                </Grid>
                <Grid item xs={6}>
                  <Typography variant="body2" color="text.secondary">
                    <strong>Timeout:</strong> {(query.timeoutMs || 30000) / 1000}s
                  </Typography>
                </Grid>
              </Grid>

              {/* Active Filters */}
              <Box sx={{ mt: 2 }}>
                <Typography variant="body2" color="text.secondary" gutterBottom>
                  <strong>Active Filters:</strong>
                </Typography>
                <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
                  {query.nodeFilters.node_type && (
                    <Chip label={`Node: ${query.nodeFilters.node_type}`} size="small" color="primary" />
                  )}
                  {query.edgeFilters.edge_type && (
                    <Chip label={`Edge: ${query.edgeFilters.edge_type}`} size="small" color="secondary" />
                  )}
                  {query.weightThreshold && (
                    <Chip label={`Weight ≥ ${query.weightThreshold}`} size="small" />
                  )}
                  {!query.nodeFilters.node_type && !query.edgeFilters.edge_type && !query.weightThreshold && (
                    <Chip label="No filters" size="small" variant="outlined" />
                  )}
                </Box>
              </Box>
            </Box>
          </Grid>

          {/* Execution Controls */}
          <Grid item xs={12} md={4}>
            <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
              <Button
                variant="contained"
                size="large"
                startIcon={isExecuting ? <TimerIcon /> : <ExecuteIcon />}
                onClick={handleExecute}
                disabled={disabled || hasErrors || isExecuting || isPreviewMode}
                fullWidth
              >
                {isExecuting ? 'Executing...' : 'Execute Query'}
              </Button>

              <Button
                variant="outlined"
                startIcon={isPreviewMode ? <TimerIcon /> : <PreviewIcon />}
                onClick={handlePreview}
                disabled={disabled || hasErrors || isExecuting || isPreviewMode}
                fullWidth
              >
                {isPreviewMode ? 'Previewing...' : 'Preview Query'}
              </Button>

              <Divider />

              {/* Performance Estimates */}
              <Box>
                <Typography variant="subtitle2" gutterBottom>
                  Performance Estimates
                </Typography>
                <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
                  <SpeedIcon fontSize="small" sx={{ mr: 1, color: 'text.secondary' }} />
                  <Typography variant="body2" color="text.secondary">
                    Complexity: {query.maxDepth && query.maxDepth > 5 ? 'High' : 'Medium'}
                  </Typography>
                </Box>
                <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
                  <MemoryIcon fontSize="small" sx={{ mr: 1, color: 'text.secondary' }} />
                  <Typography variant="body2" color="text.secondary">
                    Memory: {query.maxResults && query.maxResults > 500 ? 'High' : 'Low'}
                  </Typography>
                </Box>
                <Box sx={{ display: 'flex', alignItems: 'center' }}>
                  <TimerIcon fontSize="small" sx={{ mr: 1, color: 'text.secondary' }} />
                  <Typography variant="body2" color="text.secondary">
                    Est. Time: {query.maxDepth && query.maxDepth > 5 ? '10-30s' : '1-5s'}
                  </Typography>
                </Box>
              </Box>
            </Box>
          </Grid>
        </Grid>
      </CardContent>

      {/* Query Code Dialog */}
      <Dialog open={showQueryCode} onClose={() => setShowQueryCode(false)} maxWidth="md" fullWidth>
        <DialogTitle>Query Code</DialogTitle>
        <DialogContent>
          <Typography variant="body2" color="text.secondary" gutterBottom>
            JSON representation of the traversal query:
          </Typography>
          <TextField
            multiline
            rows={15}
            value={generateQueryCode()}
            fullWidth
            variant="outlined"
            InputProps={{
              readOnly: true,
              style: { fontFamily: 'monospace', fontSize: '0.875rem' },
            }}
          />
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setShowQueryCode(false)}>Close</Button>
          <Button onClick={exportQuery} variant="contained">
            Export
          </Button>
        </DialogActions>
      </Dialog>

      {/* Execution Settings Dialog */}
      <Dialog open={showSettings} onClose={() => setShowSettings(false)} maxWidth="sm" fullWidth>
        <DialogTitle>Execution Settings</DialogTitle>
        <DialogContent>
          <Box sx={{ pt: 1 }}>
            <FormControlLabel
              control={
                <Switch
                  checked={executionSettings.enableProfiling}
                  onChange={(e) => setExecutionSettings(prev => ({
                    ...prev,
                    enableProfiling: e.target.checked
                  }))}
                />
              }
              label="Enable Performance Profiling"
            />
            <Typography variant="caption" display="block" color="text.secondary" sx={{ mb: 2 }}>
              Collect detailed performance metrics during execution
            </Typography>

            <FormControlLabel
              control={
                <Switch
                  checked={executionSettings.enableCaching}
                  onChange={(e) => setExecutionSettings(prev => ({
                    ...prev,
                    enableCaching: e.target.checked
                  }))}
                />
              }
              label="Enable Result Caching"
            />
            <Typography variant="caption" display="block" color="text.secondary" sx={{ mb: 2 }}>
              Cache query results for faster repeated execution
            </Typography>

            <FormControlLabel
              control={
                <Switch
                  checked={executionSettings.enableOptimization}
                  onChange={(e) => setExecutionSettings(prev => ({
                    ...prev,
                    enableOptimization: e.target.checked
                  }))}
                />
              }
              label="Enable Query Optimization"
            />
            <Typography variant="caption" display="block" color="text.secondary" sx={{ mb: 2 }}>
              Apply automatic optimizations to improve performance
            </Typography>

            <FormControlLabel
              control={
                <Switch
                  checked={executionSettings.verboseLogging}
                  onChange={(e) => setExecutionSettings(prev => ({
                    ...prev,
                    verboseLogging: e.target.checked
                  }))}
                />
              }
              label="Verbose Logging"
            />
            <Typography variant="caption" display="block" color="text.secondary">
              Enable detailed logging for debugging purposes
            </Typography>
          </Box>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setShowSettings(false)}>Close</Button>
          <Button onClick={() => setShowSettings(false)} variant="contained">
            Apply Settings
          </Button>
        </DialogActions>
      </Dialog>
    </Card>
  );
};

export default QueryExecutor;