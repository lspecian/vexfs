import React, { useEffect, useState, useCallback } from 'react';
import {
  Box,
  Typography,
  Paper,
  Grid,
  Card,
  CardContent,
  Chip,
  useTheme,
  Alert,
  Button,
  CircularProgress,
  Divider,
} from '@mui/material';
import {
  AccountTree as GraphIcon,
  Timeline as AnalyticsIcon,
  Search as SearchIcon,
  Settings as ConfigIcon,
  Refresh as RefreshIcon,
} from '@mui/icons-material';

import { GraphVisualization, GraphDemo, NodeEdgeManager, QueryBuilder, SchemaManager } from '../components/Graph';
import { vexfsApi } from '../services/api';
import type { NodeResponse, EdgeResponse, GraphStatistics } from '../types/graph';

const Graph: React.FC = () => {
  const theme = useTheme();
  const api = vexfsApi;
  
  // State management
  const [nodes, setNodes] = useState<NodeResponse[]>([]);
  const [edges, setEdges] = useState<EdgeResponse[]>([]);
  const [selectedNodes, setSelectedNodes] = useState<string[]>([]);
  const [selectedEdges, setSelectedEdges] = useState<string[]>([]);
  const [graphStats, setGraphStats] = useState<GraphStatistics | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isHealthy, setIsHealthy] = useState<boolean | null>(null);

  // Load graph data
  const loadGraphData = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);

      // Check VexGraph health first
      const healthy = await api.checkVexGraphHealth();
      setIsHealthy(healthy);

      if (!healthy) {
        setError('VexGraph service is not available');
        return;
      }

      // Load nodes and edges in parallel
      const [nodesResult, edgesResult, statsResult] = await Promise.all([
        api.listNodes({}, 100, 0),
        api.listEdges({}, 100, 0),
        api.getGraphStats().catch(() => null), // Don't fail if stats unavailable
      ]);

      setNodes(nodesResult.items);
      setEdges(edgesResult.items);
      setGraphStats(statsResult);

    } catch (err) {
      console.error('Failed to load graph data:', err);
      setError(err instanceof Error ? err.message : 'Failed to load graph data');
    } finally {
      setIsLoading(false);
    }
  }, [api]);

  // Load data on component mount
  useEffect(() => {
    loadGraphData();
  }, [loadGraphData]);

  // Event handlers
  const handleNodeSelect = useCallback((nodeIds: string[]) => {
    setSelectedNodes(nodeIds);
  }, []);

  const handleEdgeSelect = useCallback((edgeIds: string[]) => {
    setSelectedEdges(edgeIds);
  }, []);

  const handleNodeDoubleClick = useCallback((node: NodeResponse) => {
    console.log('Node double-clicked:', node);
    // Double-click opens edit dialog via NodeEdgeManager
    setSelectedNodes([node.id]);
  }, []);

  const handleEdgeDoubleClick = useCallback((edge: EdgeResponse) => {
    console.log('Edge double-clicked:', edge);
    // Double-click opens edit dialog via NodeEdgeManager
    setSelectedEdges([edge.id]);
  }, []);

  const handleRefresh = useCallback(() => {
    loadGraphData();
  }, [loadGraphData]);

  return (
    <Box sx={{ p: 3 }} role="main" aria-label="VexGraph Dashboard">
      {/* Page Header */}
      <Box sx={{ mb: 4 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
          <Typography variant="h4" component="h1" data-testid="graph-page-title">
            VexGraph
          </Typography>
          <Box sx={{ display: 'flex', gap: 1 }}>
            <Button
              variant="outlined"
              startIcon={<RefreshIcon />}
              onClick={handleRefresh}
              disabled={isLoading}
            >
              Refresh
            </Button>
          </Box>
        </Box>
        <Typography variant="body1" color="text.secondary">
          Explore and analyze the filesystem graph structure with advanced traversal and semantic search capabilities.
        </Typography>
      </Box>

      {/* Status Banner */}
      <Paper
        sx={{
          p: 3,
          mb: 4,
          backgroundColor: isHealthy
            ? theme.palette.success.light
            : theme.palette.warning.light,
          color: isHealthy
            ? theme.palette.success.contrastText
            : theme.palette.warning.contrastText,
        }}
        role="banner"
        aria-label="Graph interface status"
        data-testid="graph-status-banner"
      >
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
          <GraphIcon aria-hidden="true" />
          <Box>
            <Typography variant="h6" component="div">
              {isHealthy ? 'Graph Interface Active' : 'Graph Interface Unavailable'}
            </Typography>
            <Typography variant="body2">
              {isHealthy
                ? 'VexGraph backend is running and ready for visualization and analysis.'
                : 'VexGraph service is not responding. Please check the backend connection.'
              }
            </Typography>
          </Box>
          <Chip
            label={isHealthy ? 'Online' : 'Offline'}
            color={isHealthy ? 'success' : 'warning'}
            variant="outlined"
            sx={{ ml: 'auto' }}
            aria-label={`Service status: ${isHealthy ? 'Online' : 'Offline'}`}
          />
        </Box>
      </Paper>

      {/* Error Alert */}
      {error && (
        <Alert severity="error" sx={{ mb: 4 }} onClose={() => setError(null)}>
          <Typography variant="h6">Error Loading Graph Data</Typography>
          <Typography variant="body2">{error}</Typography>
        </Alert>
      )}

      {/* Graph Statistics */}
      {graphStats && (
        <Grid container spacing={2} sx={{ mb: 4 }}>
          <Grid item xs={6} sm={3}>
            <Card>
              <CardContent sx={{ textAlign: 'center' }}>
                <Typography variant="h4" color="primary">
                  {graphStats.node_count}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Nodes
                </Typography>
              </CardContent>
            </Card>
          </Grid>
          <Grid item xs={6} sm={3}>
            <Card>
              <CardContent sx={{ textAlign: 'center' }}>
                <Typography variant="h4" color="secondary">
                  {graphStats.edge_count}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Edges
                </Typography>
              </CardContent>
            </Card>
          </Grid>
          <Grid item xs={6} sm={3}>
            <Card>
              <CardContent sx={{ textAlign: 'center' }}>
                <Typography variant="h4" color="info.main">
                  {graphStats.connected_components}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Components
                </Typography>
              </CardContent>
            </Card>
          </Grid>
          <Grid item xs={6} sm={3}>
            <Card>
              <CardContent sx={{ textAlign: 'center' }}>
                <Typography variant="h4" color="success.main">
                  {graphStats.average_degree.toFixed(1)}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Avg Degree
                </Typography>
              </CardContent>
            </Card>
          </Grid>
        </Grid>
      )}

      {/* Graph Management */}
      {isHealthy && (
        <Paper sx={{ p: 3, mb: 4 }}>
          <Box sx={{ mb: 3 }}>
            <Typography variant="h6" component="div" gutterBottom>
              Graph Management
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              Create, edit, and delete nodes and edges in the VexGraph
            </Typography>
            <NodeEdgeManager
              nodes={nodes}
              edges={edges}
              selectedNodes={selectedNodes}
              selectedEdges={selectedEdges}
              onDataChange={loadGraphData}
              disabled={isLoading}
            />
          </Box>
        </Paper>
      )}

      {/* Graph Traversal Query Builder */}
      {isHealthy && (
        <Paper sx={{ mb: 4 }}>
          <Box sx={{ p: 2, borderBottom: `1px solid ${theme.palette.divider}` }}>
            <Typography variant="h6" component="div">
              Graph Traversal Query Builder
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Build and execute complex graph traversal queries with visual tools
            </Typography>
          </Box>
          <QueryBuilder
            nodes={nodes}
            edges={edges}
            onQueryExecute={(query, results) => {
              console.log('Query executed:', query, results);
            }}
            onResultsHighlight={(nodeIds, edgeIds) => {
              console.log('Highlight results:', nodeIds, edgeIds);
              // Update selected nodes/edges to highlight in visualization
              setSelectedNodes(nodeIds);
              setSelectedEdges(edgeIds);
            }}
            disabled={isLoading}
          />
        </Paper>
      )}

      {/* Schema Management */}
      {isHealthy && (
        <Paper sx={{ mb: 4 }}>
          <Box sx={{ p: 2, borderBottom: `1px solid ${theme.palette.divider}` }}>
            <Typography variant="h6" component="div">
              Schema Management
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Define and manage graph data structures, validation rules, and schema evolution
            </Typography>
          </Box>
          <SchemaManager
            onSchemaChange={(schema) => {
              console.log('Schema updated:', schema);
              // Optionally refresh graph data when schema changes
              loadGraphData();
            }}
            onValidationChange={(results) => {
              console.log('Validation results:', results);
            }}
          />
        </Paper>
      )}

      {/* Graph Visualization */}
      <Paper sx={{ mb: 4 }}>
        <Box sx={{ p: 2, borderBottom: `1px solid ${theme.palette.divider}` }}>
          <Typography variant="h6" component="div">
            Graph Visualization
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Interactive visualization of the VexFS graph structure
          </Typography>
        </Box>
        
        {isLoading ? (
          <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: 400 }}>
            <CircularProgress size={48} />
            <Typography variant="body2" sx={{ ml: 2 }}>
              Loading graph data...
            </Typography>
          </Box>
        ) : isHealthy && nodes.length > 0 ? (
          <GraphVisualization
            nodes={nodes}
            edges={edges}
            onNodeSelect={handleNodeSelect}
            onEdgeSelect={handleEdgeSelect}
            onNodeDoubleClick={handleNodeDoubleClick}
            onEdgeDoubleClick={handleEdgeDoubleClick}
            height={600}
            enableMiniMap={true}
            enableControls={true}
            enableBackground={true}
          />
        ) : !isHealthy ? (
          <Box sx={{ p: 4, textAlign: 'center' }}>
            <Typography variant="h6" color="text.secondary" gutterBottom>
              Service Unavailable
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
              VexGraph service is not responding. Please check the backend connection.
            </Typography>
            <Typography variant="body2" color="text.primary" sx={{ mb: 2 }}>
              Try the interactive demo below:
            </Typography>
            <GraphDemo />
          </Box>
        ) : (
          <Box sx={{ p: 4, textAlign: 'center' }}>
            <Typography variant="h6" color="text.secondary" gutterBottom>
              No Graph Data
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
              No nodes or edges found in the graph. Create some nodes to get started.
            </Typography>
            <Typography variant="body2" color="text.primary" sx={{ mb: 2 }}>
              Try the interactive demo below:
            </Typography>
            <GraphDemo />
          </Box>
        )}
      </Paper>

      {/* Selection Info */}
      {(selectedNodes.length > 0 || selectedEdges.length > 0) && (
        <Paper sx={{ p: 3, mb: 4 }}>
          <Typography variant="h6" gutterBottom>
            Selection Details
          </Typography>
          {selectedNodes.length > 0 && (
            <Box sx={{ mb: 2 }}>
              <Typography variant="subtitle2" color="primary">
                Selected Nodes ({selectedNodes.length}):
              </Typography>
              <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1, mt: 1 }}>
                {selectedNodes.map(nodeId => (
                  <Chip key={nodeId} label={nodeId} size="small" color="primary" />
                ))}
              </Box>
            </Box>
          )}
          {selectedEdges.length > 0 && (
            <Box>
              <Typography variant="subtitle2" color="secondary">
                Selected Edges ({selectedEdges.length}):
              </Typography>
              <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1, mt: 1 }}>
                {selectedEdges.map(edgeId => (
                  <Chip key={edgeId} label={edgeId} size="small" color="secondary" />
                ))}
              </Box>
            </Box>
          )}
        </Paper>
      )}

      {/* Feature Cards */}
      <Grid container spacing={3} role="region" aria-label="Graph features">
        <Grid item xs={12} md={6} lg={3}>
          <Card sx={{ height: '100%' }}>
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
                <GraphIcon color="primary" sx={{ mr: 1 }} />
                <Typography variant="h6" component="div">
                  Graph Visualization
                </Typography>
              </Box>
              <Typography variant="body2" color="text.secondary">
                Interactive graph visualization with zoom, pan, selection, and multiple layout algorithms.
              </Typography>
              <Chip
                label="Active"
                size="small"
                color="success"
                sx={{ mt: 2 }}
              />
            </CardContent>
          </Card>
        </Grid>

        <Grid item xs={12} md={6} lg={3}>
          <Card sx={{ height: '100%' }}>
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
                <SearchIcon color="primary" sx={{ mr: 1 }} />
                <Typography variant="h6" component="div">
                  Graph Traversal
                </Typography>
              </Box>
              <Typography variant="body2" color="text.secondary">
                Advanced graph traversal queries with BFS, DFS, shortest path algorithms and comprehensive filtering.
              </Typography>
              <Chip
                label="Active"
                size="small"
                color="success"
                sx={{ mt: 2 }}
              />
            </CardContent>
          </Card>
        </Grid>

        <Grid item xs={12} md={6} lg={3}>
          <Card sx={{ height: '100%' }}>
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
                <AnalyticsIcon color="primary" sx={{ mr: 1 }} />
                <Typography variant="h6" component="div">
                  Graph Analytics
                </Typography>
              </Box>
              <Typography variant="body2" color="text.secondary">
                Advanced graph analytics including centrality measures, community detection, and path analysis.
              </Typography>
              <Chip
                label="Coming Soon"
                size="small"
                variant="outlined"
                sx={{ mt: 2 }}
              />
            </CardContent>
          </Card>
        </Grid>

        <Grid item xs={12} md={6} lg={3}>
          <Card sx={{ height: '100%' }}>
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
                <ConfigIcon color="primary" sx={{ mr: 1 }} />
                <Typography variant="h6" component="div">
                  Schema Management
                </Typography>
              </Box>
              <Typography variant="body2" color="text.secondary">
                Define and manage graph schemas, node types, edge types, and validation rules for data structure enforcement.
              </Typography>
              <Chip
                label="Active"
                size="small"
                color="success"
                sx={{ mt: 2 }}
              />
            </CardContent>
          </Card>
        </Grid>
      </Grid>
    </Box>
  );
};

export default Graph;