import React, { useState, useCallback } from 'react';
import {
  Box,
  Typography,
  Paper,
  Grid,
  Card,
  CardContent,
  Button,
  Stack,
  Alert,
  Chip,
  Divider,
} from '@mui/material';
import {
  PlayArrow as PlayIcon,
  Stop as StopIcon,
  Add as AddIcon,
  Edit as EditIcon,
  Delete as DeleteIcon,
  Sync as SyncIcon,
} from '@mui/icons-material';

import {
  RealTimeProvider,
  RealTimeGraphVisualization,
  RealTimeConnectionStatus,
  UpdateNotifications,
  SyncManager,
  useRealTime,
} from '../components/Graph';
import type { NodeResponse, EdgeResponse } from '../types/graph';

// Demo data
const demoNodes: NodeResponse[] = [
  {
    id: 'node1',
    inode_number: 1,
    node_type: 'File',
    properties: { name: 'String', size: 'Integer' },
    outgoing_edges: ['edge1'],
    incoming_edges: [],
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: 'node2',
    inode_number: 2,
    node_type: 'Directory',
    properties: { name: 'String', children_count: 'Integer' },
    outgoing_edges: [],
    incoming_edges: ['edge1'],
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: 'node3',
    inode_number: 3,
    node_type: 'File',
    properties: { name: 'String', size: 'Integer' },
    outgoing_edges: ['edge2'],
    incoming_edges: [],
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
];

const demoEdges: EdgeResponse[] = [
  {
    id: 'edge1',
    source_id: 'node1',
    target_id: 'node2',
    edge_type: 'Contains',
    weight: 1.0,
    properties: { relationship: 'String' },
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
  {
    id: 'edge2',
    source_id: 'node3',
    target_id: 'node2',
    edge_type: 'Contains',
    weight: 1.0,
    properties: { relationship: 'String' },
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  },
];

// Demo Controls Component
const DemoControls: React.FC = () => {
  const { broadcastEvent, connectionStatus } = useRealTime();
  const [isRunning, setIsRunning] = useState(false);
  const [eventCount, setEventCount] = useState(0);

  const simulateNodeCreated = useCallback(() => {
    const newNode: NodeResponse = {
      id: `node_${Date.now()}`,
      inode_number: Math.floor(Math.random() * 1000),
      node_type: Math.random() > 0.5 ? 'File' : 'Directory',
      properties: {
        name: 'String',
        size: 'Integer',
      },
      outgoing_edges: [],
      incoming_edges: [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };

    broadcastEvent({
      type: 'graph.node.created',
      data: { node: newNode },
    });

    setEventCount(prev => prev + 1);
  }, [broadcastEvent]);

  const simulateNodeUpdated = useCallback(() => {
    const nodeId = demoNodes[Math.floor(Math.random() * demoNodes.length)].id;
    const updatedNode = {
      ...demoNodes.find(n => n.id === nodeId)!,
      properties: {
        ...demoNodes.find(n => n.id === nodeId)!.properties,
        lastModified: 'String',
        version: 'Integer',
      },
      updated_at: new Date().toISOString(),
    };

    broadcastEvent({
      type: 'graph.node.updated',
      data: {
        nodeId,
        node: updatedNode,
        changes: { lastModified: 'String' },
      },
    });

    setEventCount(prev => prev + 1);
  }, [broadcastEvent]);

  const simulateEdgeCreated = useCallback(() => {
    const sourceId = demoNodes[Math.floor(Math.random() * demoNodes.length)].id;
    const targetId = demoNodes[Math.floor(Math.random() * demoNodes.length)].id;
    
    if (sourceId === targetId) return;

    const newEdge: EdgeResponse = {
      id: `edge_${Date.now()}`,
      source_id: sourceId,
      target_id: targetId,
      edge_type: 'References',
      weight: Math.random(),
      properties: { strength: 'Float' },
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };

    broadcastEvent({
      type: 'graph.edge.created',
      data: { edge: newEdge },
    });

    setEventCount(prev => prev + 1);
  }, [broadcastEvent]);

  const simulateConflict = useCallback(() => {
    const nodeId = demoNodes[Math.floor(Math.random() * demoNodes.length)].id;

    broadcastEvent({
      type: 'graph.conflict.detected',
      data: {
        entityType: 'node',
        entityId: nodeId,
        conflictType: 'concurrent_modification',
        localVersion: 1,
        remoteVersion: 2,
        conflictingChanges: {
          name: 'String',
          size: 'Integer',
        },
      },
    });

    setEventCount(prev => prev + 1);
  }, [broadcastEvent]);

  const startDemo = useCallback(() => {
    setIsRunning(true);
    
    const interval = setInterval(() => {
      const eventType = Math.random();
      
      if (eventType < 0.4) {
        simulateNodeCreated();
      } else if (eventType < 0.7) {
        simulateNodeUpdated();
      } else if (eventType < 0.9) {
        simulateEdgeCreated();
      } else {
        simulateConflict();
      }
    }, 2000);

    // Store interval ID for cleanup
    (window as any).demoInterval = interval;
  }, [simulateNodeCreated, simulateNodeUpdated, simulateEdgeCreated, simulateConflict]);

  const stopDemo = useCallback(() => {
    setIsRunning(false);
    if ((window as any).demoInterval) {
      clearInterval((window as any).demoInterval);
      (window as any).demoInterval = null;
    }
  }, []);

  const isConnected = connectionStatus.state === 'connected';

  return (
    <Card>
      <CardContent>
        <Typography variant="h6" gutterBottom>
          Real-time Demo Controls
        </Typography>
        
        <Stack spacing={2}>
          <Alert severity="info">
            This demo simulates real-time graph events. Use the controls below to generate events
            and observe how the graph updates in real-time.
          </Alert>

          <Box display="flex" alignItems="center" gap={1}>
            <Typography variant="body2">
              Connection Status:
            </Typography>
            <Chip
              label={connectionStatus.state}
              color={isConnected ? 'success' : 'error'}
              size="small"
            />
          </Box>

          <Box display="flex" alignItems="center" gap={1}>
            <Typography variant="body2">
              Events Generated:
            </Typography>
            <Chip label={eventCount} color="primary" size="small" />
          </Box>

          <Divider />

          <Stack direction="row" spacing={1}>
            <Button
              variant="contained"
              startIcon={isRunning ? <StopIcon /> : <PlayIcon />}
              onClick={isRunning ? stopDemo : startDemo}
              disabled={!isConnected}
              color={isRunning ? 'error' : 'primary'}
            >
              {isRunning ? 'Stop Demo' : 'Start Demo'}
            </Button>
          </Stack>

          <Typography variant="caption" color="text.secondary">
            Auto-demo generates random events every 2 seconds
          </Typography>

          <Divider />

          <Typography variant="subtitle2">
            Manual Event Generation
          </Typography>

          <Stack direction="row" spacing={1} flexWrap="wrap">
            <Button
              size="small"
              startIcon={<AddIcon />}
              onClick={simulateNodeCreated}
              disabled={!isConnected}
            >
              Add Node
            </Button>
            
            <Button
              size="small"
              startIcon={<EditIcon />}
              onClick={simulateNodeUpdated}
              disabled={!isConnected}
            >
              Update Node
            </Button>
            
            <Button
              size="small"
              startIcon={<AddIcon />}
              onClick={simulateEdgeCreated}
              disabled={!isConnected}
            >
              Add Edge
            </Button>
            
            <Button
              size="small"
              startIcon={<SyncIcon />}
              onClick={simulateConflict}
              disabled={!isConnected}
              color="warning"
            >
              Simulate Conflict
            </Button>
          </Stack>
        </Stack>
      </CardContent>
    </Card>
  );
};

// Main Demo Component
const RealTimeDemoContent: React.FC = () => {
  const [nodes] = useState<NodeResponse[]>(demoNodes);
  const [edges] = useState<EdgeResponse[]>(demoEdges);

  return (
    <Box sx={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <Paper elevation={1} sx={{ p: 2, borderRadius: 0 }}>
        <Typography variant="h4" fontWeight="bold" gutterBottom>
          VexGraph Real-time Updates Demo
        </Typography>
        <Typography variant="body1" color="text.secondary">
          Experience real-time graph updates, conflict resolution, and collaborative editing features.
        </Typography>
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
            overflow: 'auto',
            p: 2,
          }}
        >
          <Stack spacing={3}>
            {/* Connection Status */}
            <RealTimeConnectionStatus 
              showDetails={true}
              showMetrics={true}
            />
            
            {/* Demo Controls */}
            <DemoControls />
            
            {/* Sync Manager */}
            <SyncManager
              autoSync={false}
              showSyncHistory={true}
            />
          </Stack>
        </Paper>

        {/* Main Graph Area */}
        <Box sx={{ flex: 1, position: 'relative' }}>
          <RealTimeGraphVisualization
            nodes={nodes}
            edges={edges}
            enableRealTimeUpdates={true}
            enableOptimisticUpdates={true}
            showConnectionStatus={false}
            showNotifications={true}
            showSyncManager={false}
          />
        </Box>
      </Box>

      {/* Footer */}
      <Paper elevation={1} sx={{ p: 1, borderRadius: 0, borderTop: 1, borderColor: 'divider' }}>
        <Typography variant="caption" color="text.secondary" textAlign="center">
          VexGraph Real-time Demo - WebSocket-based collaborative graph editing
        </Typography>
      </Paper>
    </Box>
  );
};

// Main Demo Page with Provider
const RealTimeDemo: React.FC = () => {
  return (
    <RealTimeProvider
      wsUrl="ws://localhost:7680"
      enableOptimisticUpdates={true}
      enableBatching={true}
      batchSize={5}
      batchTimeout={500}
    >
      <RealTimeDemoContent />
      <UpdateNotifications />
    </RealTimeProvider>
  );
};

export default RealTimeDemo;