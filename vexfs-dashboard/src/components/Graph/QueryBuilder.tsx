import React, { useState, useCallback, useEffect } from 'react';
import {
  Box,
  Paper,
  Typography,
  Grid,
  Card,
  CardContent,
  CardActions,
  Button,
  Chip,
  Alert,
  Divider,
  IconButton,
  Tooltip,
  CircularProgress,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  TextField,
  Switch,
  FormControlLabel,
  Tabs,
  Tab,
  List,
  ListItem,
  ListItemText,
  ListItemSecondaryAction,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
} from '@mui/material';
import {
  ExpandMore as ExpandMoreIcon,
  PlayArrow as ExecuteIcon,
  Save as SaveIcon,
  Download as ExportIcon,
  History as HistoryIcon,
  ViewModule as TemplateIcon,
  Clear as ClearIcon,
  Visibility as PreviewIcon,
  Settings as SettingsIcon,
  Delete as DeleteIcon,
  Edit as EditIcon,
} from '@mui/icons-material';

import { TraversalPathBuilder } from './TraversalPathBuilder';
import { FilterBuilder } from './FilterBuilder';
import { QueryExecutor } from './QueryExecutor';
import { QueryTemplates } from './QueryTemplates';
import { QueryResultsPanel } from './QueryResultsPanel';
import { vexfsApi } from '../../services/api';
import type {
  NodeResponse,
  EdgeResponse,
  TraversalQuery,
  TraversalResult,
  TraversalAlgorithm,
  NodeType,
  EdgeType,
  NodeFilters,
  EdgeFilters,
} from '../../types/graph';

export interface QueryBuilderQuery {
  id: string;
  name: string;
  description?: string;
  algorithm: TraversalAlgorithm;
  startNode?: string;
  endNode?: string;
  maxDepth?: number;
  maxResults?: number;
  nodeFilters: NodeFilters;
  edgeFilters: EdgeFilters;
  weightThreshold?: number;
  timeoutMs?: number;
  createdAt: string;
  updatedAt: string;
}

export interface SavedQuery extends QueryBuilderQuery {
  tags: string[];
  category: string;
  isPublic: boolean;
  executionCount: number;
  lastExecuted?: string;
}

export interface QueryBuilderProps {
  nodes: NodeResponse[];
  edges: EdgeResponse[];
  onQueryExecute?: (query: TraversalQuery, results: TraversalResult) => void;
  onResultsHighlight?: (nodeIds: string[], edgeIds: string[]) => void;
  disabled?: boolean;
}

interface TabPanelProps {
  children?: React.ReactNode;
  index: number;
  value: number;
}

function TabPanel(props: TabPanelProps) {
  const { children, value, index, ...other } = props;
  return (
    <div
      role="tabpanel"
      hidden={value !== index}
      id={`query-builder-tabpanel-${index}`}
      aria-labelledby={`query-builder-tab-${index}`}
      {...other}
    >
      {value === index && <Box sx={{ p: 3 }}>{children}</Box>}
    </div>
  );
}

export const QueryBuilder: React.FC<QueryBuilderProps> = ({
  nodes,
  edges,
  onQueryExecute,
  onResultsHighlight,
  disabled = false,
}) => {
  // State management
  const [currentQuery, setCurrentQuery] = useState<QueryBuilderQuery>({
    id: '',
    name: 'Untitled Query',
    algorithm: 'BreadthFirstSearch',
    maxDepth: 3,
    maxResults: 100,
    nodeFilters: {},
    edgeFilters: {},
    timeoutMs: 30000,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  });

  const [savedQueries, setSavedQueries] = useState<SavedQuery[]>([]);
  const [queryHistory, setQueryHistory] = useState<(QueryBuilderQuery & { results?: TraversalResult })[]>([]);
  const [isExecuting, setIsExecuting] = useState(false);
  const [executionResults, setExecutionResults] = useState<TraversalResult | null>(null);
  const [executionError, setExecutionError] = useState<string | null>(null);
  const [previewResults, setPreviewResults] = useState<TraversalResult | null>(null);
  const [isPreviewMode, setIsPreviewMode] = useState(false);
  
  // UI state
  const [activeTab, setActiveTab] = useState(0);
  const [saveDialogOpen, setSaveDialogOpen] = useState(false);
  const [saveQueryName, setSaveQueryName] = useState('');
  const [saveQueryDescription, setSaveQueryDescription] = useState('');
  const [saveQueryTags, setSaveQueryTags] = useState<string[]>([]);
  const [saveQueryCategory, setSaveQueryCategory] = useState('general');

  // Load saved queries on mount
  useEffect(() => {
    loadSavedQueries();
    loadQueryHistory();
  }, []);

  const loadSavedQueries = useCallback(async () => {
    try {
      // In a real implementation, this would load from localStorage or API
      const saved = localStorage.getItem('vexgraph-saved-queries');
      if (saved) {
        setSavedQueries(JSON.parse(saved));
      }
    } catch (error) {
      console.error('Failed to load saved queries:', error);
    }
  }, []);

  const loadQueryHistory = useCallback(async () => {
    try {
      const history = localStorage.getItem('vexgraph-query-history');
      if (history) {
        setQueryHistory(JSON.parse(history));
      }
    } catch (error) {
      console.error('Failed to load query history:', error);
    }
  }, []);

  const saveQueryToHistory = useCallback((query: QueryBuilderQuery, results?: TraversalResult) => {
    const historyEntry = {
      ...query,
      id: `history-${Date.now()}`,
      results,
    };
    
    const newHistory = [historyEntry, ...queryHistory.slice(0, 49)]; // Keep last 50
    setQueryHistory(newHistory);
    localStorage.setItem('vexgraph-query-history', JSON.stringify(newHistory));
  }, [queryHistory]);

  const handleQueryUpdate = useCallback((updates: Partial<QueryBuilderQuery>) => {
    setCurrentQuery(prev => ({
      ...prev,
      ...updates,
      updatedAt: new Date().toISOString(),
    }));
  }, []);

  const handleExecuteQuery = useCallback(async () => {
    if (!currentQuery.startNode) {
      setExecutionError('Please select a starting node');
      return;
    }

    try {
      setIsExecuting(true);
      setExecutionError(null);

      const traversalQuery: TraversalQuery = {
        algorithm: currentQuery.algorithm,
        start_node: currentQuery.startNode,
        end_node: currentQuery.endNode,
        max_depth: currentQuery.maxDepth,
        max_results: currentQuery.maxResults,
        node_filter: currentQuery.nodeFilters.node_type,
        edge_filter: currentQuery.edgeFilters.edge_type,
        weight_threshold: currentQuery.weightThreshold,
        timeout_ms: currentQuery.timeoutMs,
      };

      const results = await vexfsApi.executeTraversal(traversalQuery);
      setExecutionResults(results);
      
      // Save to history
      saveQueryToHistory(currentQuery, results);
      
      // Highlight results in visualization
      if (onResultsHighlight) {
        onResultsHighlight(results.visited_nodes, results.traversed_edges);
      }
      
      // Notify parent
      if (onQueryExecute) {
        onQueryExecute(traversalQuery, results);
      }

    } catch (error) {
      console.error('Query execution failed:', error);
      setExecutionError(error instanceof Error ? error.message : 'Query execution failed');
    } finally {
      setIsExecuting(false);
    }
  }, [currentQuery, onQueryExecute, onResultsHighlight, saveQueryToHistory]);

  const handlePreviewQuery = useCallback(async () => {
    if (!currentQuery.startNode) return;

    try {
      setIsPreviewMode(true);
      
      // Create a limited preview query
      const previewQuery: TraversalQuery = {
        algorithm: currentQuery.algorithm,
        start_node: currentQuery.startNode!,
        end_node: currentQuery.endNode,
        max_depth: Math.min(currentQuery.maxDepth || 3, 2),
        max_results: Math.min(currentQuery.maxResults || 100, 20),
        node_filter: currentQuery.nodeFilters.node_type,
        edge_filter: currentQuery.edgeFilters.edge_type,
        weight_threshold: currentQuery.weightThreshold,
        timeout_ms: 5000,
      };

      const results = await vexfsApi.executeTraversal(previewQuery);
      setPreviewResults(results);
      
      if (onResultsHighlight) {
        onResultsHighlight(results.visited_nodes, results.traversed_edges);
      }

    } catch (error) {
      console.error('Preview failed:', error);
    } finally {
      setIsPreviewMode(false);
    }
  }, [currentQuery, onResultsHighlight]);

  const handleSaveQuery = useCallback(async () => {
    try {
      const savedQuery: SavedQuery = {
        ...currentQuery,
        id: `saved-${Date.now()}`,
        name: saveQueryName || currentQuery.name,
        description: saveQueryDescription,
        tags: saveQueryTags,
        category: saveQueryCategory,
        isPublic: false,
        executionCount: 0,
      };

      const newSavedQueries = [...savedQueries, savedQuery];
      setSavedQueries(newSavedQueries);
      localStorage.setItem('vexgraph-saved-queries', JSON.stringify(newSavedQueries));
      
      setSaveDialogOpen(false);
      setSaveQueryName('');
      setSaveQueryDescription('');
      setSaveQueryTags([]);
      setSaveQueryCategory('general');

    } catch (error) {
      console.error('Failed to save query:', error);
    }
  }, [currentQuery, saveQueryName, saveQueryDescription, saveQueryTags, saveQueryCategory, savedQueries]);

  const handleLoadQuery = useCallback((query: QueryBuilderQuery) => {
    setCurrentQuery({
      ...query,
      id: '',
      updatedAt: new Date().toISOString(),
    });
  }, []);

  const handleClearQuery = useCallback(() => {
    setCurrentQuery({
      id: '',
      name: 'Untitled Query',
      algorithm: 'BreadthFirstSearch',
      maxDepth: 3,
      maxResults: 100,
      nodeFilters: {},
      edgeFilters: {},
      timeoutMs: 30000,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    });
    setExecutionResults(null);
    setExecutionError(null);
    setPreviewResults(null);
  }, []);

  const handleExportQuery = useCallback(() => {
    const exportData = {
      query: currentQuery,
      results: executionResults,
      exportedAt: new Date().toISOString(),
    };
    
    const blob = new Blob([JSON.stringify(exportData, null, 2)], {
      type: 'application/json',
    });
    
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `vexgraph-query-${currentQuery.name.replace(/\s+/g, '-')}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, [currentQuery, executionResults]);

  return (
    <Paper sx={{ p: 3 }}>
      <Box sx={{ mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
          <Typography variant="h6" component="div">
            Graph Traversal Query Builder
          </Typography>
          <Box sx={{ display: 'flex', gap: 1 }}>
            <Tooltip title="Preview Query">
              <IconButton
                onClick={handlePreviewQuery}
                disabled={disabled || !currentQuery.startNode || isPreviewMode}
                size="small"
              >
                {isPreviewMode ? <CircularProgress size={20} /> : <PreviewIcon />}
              </IconButton>
            </Tooltip>
            <Tooltip title="Execute Query">
              <IconButton
                onClick={handleExecuteQuery}
                disabled={disabled || !currentQuery.startNode || isExecuting}
                color="primary"
                size="small"
              >
                {isExecuting ? <CircularProgress size={20} /> : <ExecuteIcon />}
              </IconButton>
            </Tooltip>
            <Tooltip title="Save Query">
              <IconButton
                onClick={() => setSaveDialogOpen(true)}
                disabled={disabled}
                size="small"
              >
                <SaveIcon />
              </IconButton>
            </Tooltip>
            <Tooltip title="Export Query">
              <IconButton
                onClick={handleExportQuery}
                disabled={disabled}
                size="small"
              >
                <ExportIcon />
              </IconButton>
            </Tooltip>
            <Tooltip title="Clear Query">
              <IconButton
                onClick={handleClearQuery}
                disabled={disabled}
                size="small"
              >
                <ClearIcon />
              </IconButton>
            </Tooltip>
          </Box>
        </Box>
        
        <Typography variant="body2" color="text.secondary">
          Build complex graph traversal queries with visual tools and execute them on the VexGraph
        </Typography>
      </Box>

      {/* Error Alert */}
      {executionError && (
        <Alert severity="error" sx={{ mb: 3 }} onClose={() => setExecutionError(null)}>
          <Typography variant="subtitle2">Query Execution Failed</Typography>
          <Typography variant="body2">{executionError}</Typography>
        </Alert>
      )}

      {/* Preview Results Alert */}
      {previewResults && (
        <Alert severity="info" sx={{ mb: 3 }}>
          <Typography variant="subtitle2">Preview Results</Typography>
          <Typography variant="body2">
            Found {previewResults.visited_nodes.length} nodes and {previewResults.traversed_edges.length} edges
            in {previewResults.execution_time_ms}ms (limited preview)
          </Typography>
        </Alert>
      )}

      {/* Query Builder Tabs */}
      <Box sx={{ borderBottom: 1, borderColor: 'divider', mb: 3 }}>
        <Tabs value={activeTab} onChange={(_, newValue) => setActiveTab(newValue)}>
          <Tab label="Build Query" />
          <Tab label="Templates" />
          <Tab label="History" />
          <Tab label="Saved Queries" />
          <Tab label="Results" disabled={!executionResults} />
        </Tabs>
      </Box>

      {/* Build Query Tab */}
      <TabPanel value={activeTab} index={0}>
        <Grid container spacing={3}>
          <Grid item xs={12} md={6}>
            <TraversalPathBuilder
              query={currentQuery}
              nodes={nodes}
              edges={edges}
              onQueryUpdate={handleQueryUpdate}
              disabled={disabled}
            />
          </Grid>
          <Grid item xs={12} md={6}>
            <FilterBuilder
              query={currentQuery}
              nodes={nodes}
              edges={edges}
              onQueryUpdate={handleQueryUpdate}
              disabled={disabled}
            />
          </Grid>
          <Grid item xs={12}>
            <QueryExecutor
              query={currentQuery}
              onExecute={handleExecuteQuery}
              onPreview={handlePreviewQuery}
              isExecuting={isExecuting}
              isPreviewMode={isPreviewMode}
              disabled={disabled}
            />
          </Grid>
        </Grid>
      </TabPanel>

      {/* Templates Tab */}
      <TabPanel value={activeTab} index={1}>
        <QueryTemplates
          nodes={nodes}
          edges={edges}
          onTemplateSelect={handleLoadQuery}
          disabled={disabled}
        />
      </TabPanel>

      {/* History Tab */}
      <TabPanel value={activeTab} index={2}>
        <Typography variant="h6" gutterBottom>
          Query History
        </Typography>
        <List>
          {queryHistory.map((historyQuery, index) => (
            <ListItem key={historyQuery.id} divider>
              <ListItemText
                primary={historyQuery.name}
                secondary={
                  <Box>
                    <Typography variant="body2" color="text.secondary">
                      {historyQuery.algorithm} • {new Date(historyQuery.updatedAt).toLocaleString()}
                    </Typography>
                    {historyQuery.results && (
                      <Typography variant="caption" color="text.secondary">
                        Results: {historyQuery.results.visited_nodes.length} nodes, {historyQuery.results.traversed_edges.length} edges
                      </Typography>
                    )}
                  </Box>
                }
              />
              <ListItemSecondaryAction>
                <IconButton
                  onClick={() => handleLoadQuery(historyQuery)}
                  disabled={disabled}
                  size="small"
                >
                  <EditIcon />
                </IconButton>
              </ListItemSecondaryAction>
            </ListItem>
          ))}
          {queryHistory.length === 0 && (
            <ListItem>
              <ListItemText
                primary="No query history"
                secondary="Execute queries to see them here"
              />
            </ListItem>
          )}
        </List>
      </TabPanel>

      {/* Saved Queries Tab */}
      <TabPanel value={activeTab} index={3}>
        <Typography variant="h6" gutterBottom>
          Saved Queries
        </Typography>
        <List>
          {savedQueries.map((savedQuery) => (
            <ListItem key={savedQuery.id} divider>
              <ListItemText
                primary={savedQuery.name}
                secondary={
                  <Box>
                    <Typography variant="body2" color="text.secondary">
                      {savedQuery.description || 'No description'}
                    </Typography>
                    <Box sx={{ display: 'flex', gap: 0.5, mt: 0.5 }}>
                      {savedQuery.tags.map((tag) => (
                        <Chip key={tag} label={tag} size="small" variant="outlined" />
                      ))}
                    </Box>
                  </Box>
                }
              />
              <ListItemSecondaryAction>
                <IconButton
                  onClick={() => handleLoadQuery(savedQuery)}
                  disabled={disabled}
                  size="small"
                >
                  <EditIcon />
                </IconButton>
              </ListItemSecondaryAction>
            </ListItem>
          ))}
          {savedQueries.length === 0 && (
            <ListItem>
              <ListItemText
                primary="No saved queries"
                secondary="Save queries to reuse them later"
              />
            </ListItem>
          )}
        </List>
      </TabPanel>

      {/* Results Tab */}
      <TabPanel value={activeTab} index={4}>
        {executionResults && (
          <QueryResultsPanel
            results={executionResults}
            query={currentQuery}
            nodes={nodes}
            edges={edges}
            onHighlight={onResultsHighlight}
          />
        )}
      </TabPanel>

      {/* Save Query Dialog */}
      <Dialog open={saveDialogOpen} onClose={() => setSaveDialogOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle>Save Query</DialogTitle>
        <DialogContent>
          <TextField
            autoFocus
            margin="dense"
            label="Query Name"
            fullWidth
            variant="outlined"
            value={saveQueryName}
            onChange={(e) => setSaveQueryName(e.target.value)}
            sx={{ mb: 2 }}
          />
          <TextField
            margin="dense"
            label="Description"
            fullWidth
            multiline
            rows={3}
            variant="outlined"
            value={saveQueryDescription}
            onChange={(e) => setSaveQueryDescription(e.target.value)}
            sx={{ mb: 2 }}
          />
          <FormControl fullWidth sx={{ mb: 2 }}>
            <InputLabel>Category</InputLabel>
            <Select
              value={saveQueryCategory}
              onChange={(e) => setSaveQueryCategory(e.target.value)}
              label="Category"
            >
              <MenuItem value="general">General</MenuItem>
              <MenuItem value="analysis">Analysis</MenuItem>
              <MenuItem value="pathfinding">Pathfinding</MenuItem>
              <MenuItem value="exploration">Exploration</MenuItem>
              <MenuItem value="custom">Custom</MenuItem>
            </Select>
          </FormControl>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setSaveDialogOpen(false)}>Cancel</Button>
          <Button onClick={handleSaveQuery} variant="contained">
            Save Query
          </Button>
        </DialogActions>
      </Dialog>
    </Paper>
  );
};

export default QueryBuilder;