import React, { useState, useCallback, useMemo } from 'react';
import {
  Box,
  Typography,
  Grid,
  Card,
  CardContent,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Paper,
  Chip,
  Button,
  IconButton,
  Tooltip,
  Tabs,
  Tab,
  Alert,
  Divider,
  List,
  ListItem,
  ListItemText,
  ListItemIcon,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  TextField,
  InputAdornment,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
} from '@mui/material';
import {
  Assessment as ResultsIcon,
  Timeline as PathIcon,
  AccountTree as NodesIcon,
  Share as EdgesIcon,
  Speed as PerformanceIcon,
  Visibility as HighlightIcon,
  Download as ExportIcon,
  Search as SearchIcon,
  ExpandMore as ExpandMoreIcon,
  CheckCircle as SuccessIcon,
  Error as ErrorIcon,
  Info as InfoIcon,
} from '@mui/icons-material';

import type {
  NodeResponse,
  EdgeResponse,
  TraversalResult,
} from '../../types/graph';
import type { QueryBuilderQuery } from './QueryBuilder';

export interface QueryResultsPanelProps {
  results: TraversalResult;
  query: QueryBuilderQuery;
  nodes: NodeResponse[];
  edges: EdgeResponse[];
  onHighlight?: (nodeIds: string[], edgeIds: string[]) => void;
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
      id={`results-tabpanel-${index}`}
      aria-labelledby={`results-tab-${index}`}
      {...other}
    >
      {value === index && <Box sx={{ p: 3 }}>{children}</Box>}
    </div>
  );
}

export const QueryResultsPanel: React.FC<QueryResultsPanelProps> = ({
  results,
  query,
  nodes,
  edges,
  onHighlight,
}) => {
  const [activeTab, setActiveTab] = useState(0);
  const [searchTerm, setSearchTerm] = useState('');
  const [sortBy, setSortBy] = useState<'id' | 'type' | 'created'>('id');

  // Get full node and edge data for results
  const resultNodes = useMemo(() => {
    return results.visited_nodes
      .map(nodeId => nodes.find(node => node.id === nodeId))
      .filter((node): node is NodeResponse => !!node);
  }, [results.visited_nodes, nodes]);

  const resultEdges = useMemo(() => {
    return results.traversed_edges
      .map(edgeId => edges.find(edge => edge.id === edgeId))
      .filter((edge): edge is EdgeResponse => !!edge);
  }, [results.traversed_edges, edges]);

  const pathNodes = useMemo(() => {
    if (!results.path) return [];
    return results.path
      .map(nodeId => nodes.find(node => node.id === nodeId))
      .filter((node): node is NodeResponse => !!node);
  }, [results.path, nodes]);

  // Filter and sort functions
  const filteredNodes = useMemo(() => {
    let filtered = resultNodes.filter(node =>
      node.id.toLowerCase().includes(searchTerm.toLowerCase()) ||
      node.node_type.toLowerCase().includes(searchTerm.toLowerCase())
    );

    switch (sortBy) {
      case 'type':
        filtered.sort((a, b) => a.node_type.localeCompare(b.node_type));
        break;
      case 'created':
        filtered.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());
        break;
      default:
        filtered.sort((a, b) => a.id.localeCompare(b.id));
    }

    return filtered;
  }, [resultNodes, searchTerm, sortBy]);

  const filteredEdges = useMemo(() => {
    return resultEdges.filter(edge =>
      edge.id.toLowerCase().includes(searchTerm.toLowerCase()) ||
      edge.edge_type.toLowerCase().includes(searchTerm.toLowerCase()) ||
      edge.source_id.toLowerCase().includes(searchTerm.toLowerCase()) ||
      edge.target_id.toLowerCase().includes(searchTerm.toLowerCase())
    );
  }, [resultEdges, searchTerm]);

  // Statistics
  const statistics = useMemo(() => {
    const nodeTypes = resultNodes.reduce((acc, node) => {
      acc[node.node_type] = (acc[node.node_type] || 0) + 1;
      return acc;
    }, {} as Record<string, number>);

    const edgeTypes = resultEdges.reduce((acc, edge) => {
      acc[edge.edge_type] = (acc[edge.edge_type] || 0) + 1;
      return acc;
    }, {} as Record<string, number>);

    return {
      totalNodes: resultNodes.length,
      totalEdges: resultEdges.length,
      pathLength: results.path?.length || 0,
      nodeTypes,
      edgeTypes,
      executionTime: results.execution_time_ms,
      totalWeight: results.total_weight,
      success: results.success,
    };
  }, [resultNodes, resultEdges, results]);

  const handleHighlightAll = useCallback(() => {
    if (onHighlight) {
      onHighlight(results.visited_nodes, results.traversed_edges);
    }
  }, [onHighlight, results]);

  const handleHighlightPath = useCallback(() => {
    if (onHighlight && results.path) {
      onHighlight(results.path, results.traversed_edges);
    }
  }, [onHighlight, results]);

  const handleHighlightNodes = useCallback((nodeIds: string[]) => {
    if (onHighlight) {
      onHighlight(nodeIds, []);
    }
  }, [onHighlight]);

  const handleHighlightEdges = useCallback((edgeIds: string[]) => {
    if (onHighlight) {
      onHighlight([], edgeIds);
    }
  }, [onHighlight]);

  const exportResults = useCallback(() => {
    const exportData = {
      query: {
        name: query.name,
        algorithm: query.algorithm,
        startNode: query.startNode,
        endNode: query.endNode,
        maxDepth: query.maxDepth,
        maxResults: query.maxResults,
      },
      results: {
        ...results,
        nodes: resultNodes,
        edges: resultEdges,
      },
      statistics,
      exportedAt: new Date().toISOString(),
    };

    const blob = new Blob([JSON.stringify(exportData, null, 2)], {
      type: 'application/json',
    });

    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `vexgraph-results-${query.name.replace(/\s+/g, '-')}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, [query, results, resultNodes, resultEdges, statistics]);

  return (
    <Box>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center' }}>
          <ResultsIcon color="primary" sx={{ mr: 1 }} />
          <Typography variant="h6" component="div">
            Query Results
          </Typography>
          {results.success ? (
            <SuccessIcon color="success" sx={{ ml: 1 }} />
          ) : (
            <ErrorIcon color="error" sx={{ ml: 1 }} />
          )}
        </Box>
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button
            startIcon={<HighlightIcon />}
            onClick={handleHighlightAll}
            variant="outlined"
            size="small"
          >
            Highlight All
          </Button>
          {results.path && (
            <Button
              startIcon={<PathIcon />}
              onClick={handleHighlightPath}
              variant="outlined"
              size="small"
            >
              Highlight Path
            </Button>
          )}
          <Button
            startIcon={<ExportIcon />}
            onClick={exportResults}
            variant="outlined"
            size="small"
          >
            Export
          </Button>
        </Box>
      </Box>

      {/* Results Status */}
      {!results.success && results.error_message && (
        <Alert severity="error" sx={{ mb: 3 }}>
          <Typography variant="subtitle2">Query Failed</Typography>
          <Typography variant="body2">{results.error_message}</Typography>
        </Alert>
      )}

      {/* Quick Statistics */}
      <Grid container spacing={2} sx={{ mb: 3 }}>
        <Grid item xs={6} sm={3}>
          <Card>
            <CardContent sx={{ textAlign: 'center', py: 2 }}>
              <Typography variant="h4" color="primary">
                {statistics.totalNodes}
              </Typography>
              <Typography variant="body2" color="text.secondary">
                Nodes Found
              </Typography>
            </CardContent>
          </Card>
        </Grid>
        <Grid item xs={6} sm={3}>
          <Card>
            <CardContent sx={{ textAlign: 'center', py: 2 }}>
              <Typography variant="h4" color="secondary">
                {statistics.totalEdges}
              </Typography>
              <Typography variant="body2" color="text.secondary">
                Edges Traversed
              </Typography>
            </CardContent>
          </Card>
        </Grid>
        <Grid item xs={6} sm={3}>
          <Card>
            <CardContent sx={{ textAlign: 'center', py: 2 }}>
              <Typography variant="h4" color="info.main">
                {statistics.pathLength}
              </Typography>
              <Typography variant="body2" color="text.secondary">
                Path Length
              </Typography>
            </CardContent>
          </Card>
        </Grid>
        <Grid item xs={6} sm={3}>
          <Card>
            <CardContent sx={{ textAlign: 'center', py: 2 }}>
              <Typography variant="h4" color="success.main">
                {statistics.executionTime}ms
              </Typography>
              <Typography variant="body2" color="text.secondary">
                Execution Time
              </Typography>
            </CardContent>
          </Card>
        </Grid>
      </Grid>

      {/* Results Tabs */}
      <Box sx={{ borderBottom: 1, borderColor: 'divider', mb: 3 }}>
        <Tabs value={activeTab} onChange={(_, newValue) => setActiveTab(newValue)}>
          <Tab label={`Nodes (${statistics.totalNodes})`} />
          <Tab label={`Edges (${statistics.totalEdges})`} />
          {results.path && <Tab label={`Path (${statistics.pathLength})`} />}
          <Tab label="Statistics" />
        </Tabs>
      </Box>

      {/* Search and Sort Controls */}
      {(activeTab === 0 || activeTab === 1) && (
        <Box sx={{ display: 'flex', gap: 2, mb: 2 }}>
          <TextField
            placeholder="Search results..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            size="small"
            sx={{ flexGrow: 1 }}
            InputProps={{
              startAdornment: (
                <InputAdornment position="start">
                  <SearchIcon />
                </InputAdornment>
              ),
            }}
          />
          <FormControl size="small" sx={{ minWidth: 120 }}>
            <InputLabel>Sort By</InputLabel>
            <Select
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as any)}
              label="Sort By"
            >
              <MenuItem value="id">ID</MenuItem>
              <MenuItem value="type">Type</MenuItem>
              <MenuItem value="created">Created</MenuItem>
            </Select>
          </FormControl>
        </Box>
      )}

      {/* Nodes Tab */}
      <TabPanel value={activeTab} index={0}>
        <TableContainer component={Paper}>
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell>Node ID</TableCell>
                <TableCell>Type</TableCell>
                <TableCell>Inode</TableCell>
                <TableCell>Properties</TableCell>
                <TableCell>Actions</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {filteredNodes.map((node) => (
                <TableRow key={node.id} hover>
                  <TableCell>
                    <Typography variant="body2" fontFamily="monospace">
                      {node.id}
                    </Typography>
                  </TableCell>
                  <TableCell>
                    <Chip label={node.node_type} size="small" color="primary" />
                  </TableCell>
                  <TableCell>{node.inode_number}</TableCell>
                  <TableCell>
                    <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
                      {Object.entries(node.properties || {}).slice(0, 3).map(([key, value]) => (
                        <Chip
                          key={key}
                          label={`${key}: ${String(value).slice(0, 20)}`}
                          size="small"
                          variant="outlined"
                        />
                      ))}
                      {Object.keys(node.properties || {}).length > 3 && (
                        <Chip label="..." size="small" variant="outlined" />
                      )}
                    </Box>
                  </TableCell>
                  <TableCell>
                    <Tooltip title="Highlight Node">
                      <IconButton
                        size="small"
                        onClick={() => handleHighlightNodes([node.id])}
                      >
                        <HighlightIcon />
                      </IconButton>
                    </Tooltip>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      </TabPanel>

      {/* Edges Tab */}
      <TabPanel value={activeTab} index={1}>
        <TableContainer component={Paper}>
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell>Edge ID</TableCell>
                <TableCell>Type</TableCell>
                <TableCell>Source</TableCell>
                <TableCell>Target</TableCell>
                <TableCell>Weight</TableCell>
                <TableCell>Actions</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {filteredEdges.map((edge) => (
                <TableRow key={edge.id} hover>
                  <TableCell>
                    <Typography variant="body2" fontFamily="monospace">
                      {edge.id}
                    </Typography>
                  </TableCell>
                  <TableCell>
                    <Chip label={edge.edge_type} size="small" color="secondary" />
                  </TableCell>
                  <TableCell>
                    <Typography variant="body2" fontFamily="monospace">
                      {edge.source_id}
                    </Typography>
                  </TableCell>
                  <TableCell>
                    <Typography variant="body2" fontFamily="monospace">
                      {edge.target_id}
                    </Typography>
                  </TableCell>
                  <TableCell>{edge.weight.toFixed(2)}</TableCell>
                  <TableCell>
                    <Tooltip title="Highlight Edge">
                      <IconButton
                        size="small"
                        onClick={() => handleHighlightEdges([edge.id])}
                      >
                        <HighlightIcon />
                      </IconButton>
                    </Tooltip>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      </TabPanel>

      {/* Path Tab */}
      {results.path && (
        <TabPanel value={activeTab} index={2}>
          <Alert severity="info" sx={{ mb: 2 }}>
            <Typography variant="body2">
              Path from <strong>{query.startNode}</strong> to <strong>{query.endNode}</strong>
              {statistics.totalWeight && (
                <> with total weight <strong>{statistics.totalWeight.toFixed(2)}</strong></>
              )}
            </Typography>
          </Alert>
          
          <List>
            {pathNodes.map((node, index) => (
              <ListItem key={node.id} divider={index < pathNodes.length - 1}>
                <ListItemIcon>
                  <Typography variant="body2" color="primary" fontWeight="bold">
                    {index + 1}
                  </Typography>
                </ListItemIcon>
                <ListItemText
                  primary={
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      <Typography variant="body1" fontFamily="monospace">
                        {node.id}
                      </Typography>
                      <Chip label={node.node_type} size="small" color="primary" />
                    </Box>
                  }
                  secondary={`Inode: ${node.inode_number}`}
                />
                <IconButton
                  size="small"
                  onClick={() => handleHighlightNodes([node.id])}
                >
                  <HighlightIcon />
                </IconButton>
              </ListItem>
            ))}
          </List>
        </TabPanel>
      )}

      {/* Statistics Tab */}
      <TabPanel value={activeTab} index={results.path ? 3 : 2}>
        <Grid container spacing={3}>
          <Grid item xs={12} md={6}>
            <Accordion defaultExpanded>
              <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                <Typography variant="h6">Node Type Distribution</Typography>
              </AccordionSummary>
              <AccordionDetails>
                <List dense>
                  {Object.entries(statistics.nodeTypes).map(([type, count]) => (
                    <ListItem key={type}>
                      <ListItemIcon>
                        <NodesIcon color="primary" />
                      </ListItemIcon>
                      <ListItemText
                        primary={type}
                        secondary={`${count} nodes (${((count / statistics.totalNodes) * 100).toFixed(1)}%)`}
                      />
                    </ListItem>
                  ))}
                </List>
              </AccordionDetails>
            </Accordion>
          </Grid>

          <Grid item xs={12} md={6}>
            <Accordion defaultExpanded>
              <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                <Typography variant="h6">Edge Type Distribution</Typography>
              </AccordionSummary>
              <AccordionDetails>
                <List dense>
                  {Object.entries(statistics.edgeTypes).map(([type, count]) => (
                    <ListItem key={type}>
                      <ListItemIcon>
                        <EdgesIcon color="secondary" />
                      </ListItemIcon>
                      <ListItemText
                        primary={type}
                        secondary={`${count} edges (${((count / statistics.totalEdges) * 100).toFixed(1)}%)`}
                      />
                    </ListItem>
                  ))}
                </List>
              </AccordionDetails>
            </Accordion>
          </Grid>

          <Grid item xs={12}>
            <Accordion>
              <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                <Typography variant="h6">Performance Metrics</Typography>
              </AccordionSummary>
              <AccordionDetails>
                <Grid container spacing={2}>
                  <Grid item xs={6} md={3}>
                    <Box sx={{ textAlign: 'center' }}>
                      <Typography variant="h5" color="success.main">
                        {statistics.executionTime}ms
                      </Typography>
                      <Typography variant="body2" color="text.secondary">
                        Execution Time
                      </Typography>
                    </Box>
                  </Grid>
                  <Grid item xs={6} md={3}>
                    <Box sx={{ textAlign: 'center' }}>
                      <Typography variant="h5" color="info.main">
                        {(statistics.totalNodes / (statistics.executionTime / 1000)).toFixed(0)}
                      </Typography>
                      <Typography variant="body2" color="text.secondary">
                        Nodes/Second
                      </Typography>
                    </Box>
                  </Grid>
                  <Grid item xs={6} md={3}>
                    <Box sx={{ textAlign: 'center' }}>
                      <Typography variant="h5" color="warning.main">
                        {statistics.pathLength}
                      </Typography>
                      <Typography variant="body2" color="text.secondary">
                        Path Hops
                      </Typography>
                    </Box>
                  </Grid>
                  <Grid item xs={6} md={3}>
                    <Box sx={{ textAlign: 'center' }}>
                      <Typography variant="h5" color="error.main">
                        {statistics.totalWeight?.toFixed(2) || 'N/A'}
                      </Typography>
                      <Typography variant="body2" color="text.secondary">
                        Total Weight
                      </Typography>
                    </Box>
                  </Grid>
                </Grid>
              </AccordionDetails>
            </Accordion>
          </Grid>
        </Grid>
      </TabPanel>
    </Box>
  );
};

export default QueryResultsPanel;