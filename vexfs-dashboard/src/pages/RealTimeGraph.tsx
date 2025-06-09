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
  Stack,
  Switch,
  FormControlLabel,
  Tooltip,
  IconButton,
} from '@mui/material';
import {
  AccountTree as GraphIcon,
  Timeline as AnalyticsIcon,
  Search as SearchIcon,
  Settings as ConfigIcon,
  Refresh as RefreshIcon,
  Sync as SyncIcon,
  Notifications as NotificationsIcon,
  WifiOff as OfflineIcon,
  Wifi as OnlineIcon,
} from '@mui/icons-material';

import {
  RealTimeProvider,
  RealTimeGraphVisualization,
  NodeEdgeManager,
  QueryBuilder,
  SchemaManager,
  RealTimeConnectionStatus,
  UpdateNotifications,
  SyncManager,
  useRealTime,
  useConnectionStatus,
} from '../components/Graph';
import { vexfsApi } from '../services/api';
import type { NodeResponse, EdgeResponse, GraphStatistics } from '../types/graph';

// Real-time Graph Dashboard Component
const RealTimeGraphDashboard: React.FC = () => {
  const theme = useTheme();
  const api = vexfsApi;
  const { state, connect, disconnect } = useRealTime();
  const connectionStatus = useConnectionStatus();
  
  // State management
  const [nodes, setNodes] = useState<NodeResponse[]>([]);
  const [edges, setEdges] = useState<EdgeResponse[]>([]);
  const [selectedNodes, setSelectedNodes] = useState<string[]>([]);
  const [selectedEdges, setSelectedEdges] = useState<string[]>([]);
  const [graphStats, setGraphStats] = useState<GraphStatistics | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isHealthy, setIsHealthy] = useState<boolean | null>(null);
  
  // Real-time settings
  const [realTimeEnabled, setRealTimeEnabled] = useState(true);
  const [optimisticUpdatesEnabled, setOptimisticUpdatesEnabled] = useState(true);
  const [showNotifications, setShowNotifications] = useState(true);
  const [showSyncManager, setShowSyncManager] = useState(true);

  // Load graph data
  const loadGraphData = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);

      // Check VexGraph health first
      const healthy = await api.checkVexGraphHealth();
      setIsHealthy(healthy);

      if (!healthy) {
        setError('VexGraph service is not available. Please check the backend connection.');
        return;
      }

      // Load nodes and edges
      const [nodesResponse, edgesResponse, statsResponse] = await Promise.all([
        api.listNodes(),
        api.listEdges(),
        api.getGraphStats(),
      ]);

      setNodes(nodesResponse);
      setEdges(edgesResponse);
      setGraphStats(statsResponse);
    } catch (err) {
      console.error('Failed to load graph data:', err);
      setError(err instanceof Error ? err.message : 'Failed to load graph data');
    } finally {
      setIsLoading(false);
    }
  }, [api]);

  // Initialize data on mount
  useEffect(() => {
    loadGraphData();
  }, [loadGraphData]);

  // Connect to real-time updates when enabled
  useEffect(() => {
    if (realTimeEnabled && connectionStatus.state === 'disconnected') {
      connect();
    } else if (!realTimeEnabled && connectionStatus.state === 'connected') {
      disconnect();
    }
  }, [realTimeEnabled, connectionStatus.state, connect, disconnect]);

  // Handle node selection
  const handleNodeSelect = useCallback((nodeIds: string[]) => {
    setSelectedNodes(nodeIds);
  }, []);

  // Handle edge selection
  const handleEdgeSelect = useCallback((edgeIds: string[]) => {
    setSelectedEdges(edgeIds);
  }, []);

  // Handle real-time node updates
  const handleNodeCreated = useCallback((node: NodeResponse) => {
    setNodes(prev => [...prev, node]);
  }, []);

  const handleNodeUpdated = useCallback((nodeId: string, node: NodeResponse) => {
    setNodes(prev => prev.map(n => n.id === nodeId ? node : n));
  }, []);

  const handleNodeDeleted = useCallback((nodeId: string) => {
    setNodes(prev => prev.filter(n => n.id !== nodeId));
    setSelectedNodes(prev => prev.filter(id => id !== nodeId));
  }, []);

  // Handle real-time edge updates
  const handleEdgeCreated = useCallback((edge: EdgeResponse) => {
    setEdges(prev => [...prev, edge]);
  }, []);

  const handleEdgeUpdated = useCallback((edgeId: string, edge: EdgeResponse) => {
    setEdges(prev => prev.map(e => e.id === edgeId ? edge : e));
  }, []);

  const handleEdgeDeleted = useCallback((edgeId: string) => {
    setEdges(prev => prev.filter(e => e.id !== edgeId));
    setSelectedEdges(prev => prev.filter(id => id !== edgeId));
  }, []);

  // Render connection status indicator
  const renderConnectionIndicator = () => {
    const isConnected = connectionStatus.state === 'connected';
    return (
      <Tooltip title={`Real-time updates: ${connectionStatus.state}`}>
        <Chip
          icon={isConnected ? <OnlineIcon /> : <OfflineIcon />}
          label={isConnected ? 'Live' : 'Offline'}
          color={isConnected ? 'success' : 'default'}
          size="small"
          variant="outlined"
        />
      </Tooltip>
    );
  };

  if (isLoading) {
    return (
      <Box display="flex" justifyContent="center" alignItems="center" height="100vh">
        <CircularProgress />
        <Typography variant="body1" sx={{ ml: 2 }}>
          Loading VexGraph...
        </Typography>
      </Box>
    );
  }

  if (error) {
    return (
      <Box p={3}>
        <Alert severity="error" sx={{ mb: 2 }}>
          {error}
        </Alert>
        <Button variant="contained" onClick={loadGraphData} startIcon={<RefreshIcon />}>
          Retry
        </Button>
      </Box>
    );
  }

  return (
    <Box sx={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <Paper elevation={1} sx={{ p: 2, borderRadius: 0 }}>
        <Box display="flex" alignItems="center" justifyContent="space-between">
          <Box display="flex" alignItems="center" gap={2}>
            <GraphIcon color="primary" />
            <Typography variant="h5" fontWeight="bold">
              VexGraph Real-time Dashboard
            </Typography>
            {renderConnectionIndicator()}
          </Box>
          
          <Stack direction="row" spacing={2} alignItems="center">
            {/* Real-time Controls */}
            <FormControlLabel
              control={
                <Switch
                  checked={realTimeEnabled}
                  onChange={(e) => setRealTimeEnabled(e.target.checked)}
                  size="small"
                />
              }
              label="Real-time Updates"
            />
            
            <FormControlLabel
              control={
                <Switch
                  checked={optimisticUpdatesEnabled}
                  onChange={(e) => setOptimisticUpdatesEnabled(e.target.checked)}
                  size="small"
                  disabled={!realTimeEnabled}
                />
              }
              label="Optimistic Updates"
            />
            
            <Divider orientation="vertical" flexItem />
            
            {/* Action Buttons */}
            <Tooltip title="Refresh Data">
              <IconButton onClick={loadGraphData} size="small">
                <RefreshIcon />
              </IconButton>
            </Tooltip>
            
            <UpdateNotifications />
          </Stack>
        </Box>
      </Paper>

      {/* Main Content */}
      <Box sx={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        {/* Left Sidebar */}
        <Paper 
          elevation={1} 
          sx={{ 
            width: 350, 
            borderRadius: 0, 
            borderRight: 1, 
            borderColor: 'divider',
            display: 'flex',
            flexDirection: 'column',
            overflow: 'hidden'
          }}
        >
          <Box sx={{ p: 2, borderBottom: 1, borderColor: 'divider' }}>
            <Typography variant="h6" gutterBottom>
              Graph Controls
            </Typography>
          </Box>
          
          <Box sx={{ flex: 1, overflow: 'auto', p: 2 }}>
            <Stack spacing={3}>
              {/* Connection Status */}
              <RealTimeConnectionStatus 
                showDetails={true}
                showMetrics={true}
              />
              
              {/* Graph Statistics */}
              {graphStats && (
                <Card variant="outlined">
                  <CardContent>
                    <Typography variant="subtitle1" gutterBottom>
                      Graph Statistics
                    </Typography>
                    <Grid container spacing={1}>
                      <Grid item xs={6}>
                        <Typography variant="body2">
                          <strong>Nodes:</strong> {graphStats.node_count}
                        </Typography>
                      </Grid>
                      <Grid item xs={6}>
                        <Typography variant="body2">
                          <strong>Edges:</strong> {graphStats.edge_count}
                        </Typography>
                      </Grid>
                      <Grid item xs={6}>
                        <Typography variant="body2">
                          <strong>Density:</strong> {graphStats.density.toFixed(3)}
                        </Typography>
                      </Grid>
                      <Grid item xs={6}>
                        <Typography variant="body2">
                          <strong>Components:</strong> {graphStats.connected_components}
                        </Typography>
                      </Grid>
                    </Grid>
                  </CardContent>
                </Card>
              )}
              
              {/* Node/Edge Manager */}
              <NodeEdgeManager
                selectedNodes={selectedNodes}
                selectedEdges={selectedEdges}
              />
              
              {/* Query Builder */}
              <QueryBuilder
                nodes={nodes}
                edges={edges}
              />
              
              {/* Schema Manager */}
              <SchemaManager />
              
              {/* Sync Manager */}
              {showSyncManager && (
                <SyncManager
                  autoSync={true}
                  syncInterval={30000}
                  showSyncHistory={true}
                />
              )}
            </Stack>
          </Box>
        </Paper>

        {/* Main Graph Area */}
        <Box sx={{ flex: 1, position: 'relative' }}>
          <RealTimeGraphVisualization
            nodes={nodes}
            edges={edges}
            onNodeSelect={handleNodeSelect}
            onEdgeSelect={handleEdgeSelect}
            enableRealTimeUpdates={realTimeEnabled}
            enableOptimisticUpdates={optimisticUpdatesEnabled}
            showConnectionStatus={false} // Already shown in sidebar
            showNotifications={showNotifications}
            showSyncManager={false} // Already shown in sidebar
            isLoading={isLoading}
          />
        </Box>
      </Box>

      {/* Status Bar */}
      <Paper elevation={1} sx={{ p: 1, borderRadius: 0, borderTop: 1, borderColor: 'divider' }}>
        <Box display="flex" alignItems="center" justifyContent="space-between">
          <Stack direction="row" spacing={2} alignItems="center">
            <Typography variant="caption" color="text.secondary">
              Selected: {selectedNodes.length} nodes, {selectedEdges.length} edges
            </Typography>
            
            {state.updateQueue.length > 0 && (
              <Chip
                label={`${state.updateQueue.length} pending updates`}
                size="small"
                color="info"
              />
            )}
            
            {state.pendingConflicts.length > 0 && (
              <Chip
                label={`${state.pendingConflicts.length} conflicts`}
                size="small"
                color="error"
              />
            )}
          </Stack>
          
          <Typography variant="caption" color="text.secondary">
            VexGraph Real-time Dashboard v1.0.0
          </Typography>
        </Box>
      </Paper>
    </Box>
  );
};

// Main Graph Page with Real-time Provider
const RealTimeGraph: React.FC = () => {
  return (
    <RealTimeProvider
      wsUrl="ws://localhost:7680"
      enableOptimisticUpdates={true}
      enableBatching={true}
      batchSize={10}
      batchTimeout={1000}
    >
      <RealTimeGraphDashboard />
    </RealTimeProvider>
  );
};

export default RealTimeGraph;