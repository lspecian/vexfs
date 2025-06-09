import React, { useCallback, useEffect, useMemo, useState, useRef } from 'react';
import {
  Box,
  Paper,
  Typography,
  CircularProgress,
  Alert,
  useTheme,
  Toolbar,
  IconButton,
  Tooltip,
  ButtonGroup,
  Button,
  Chip,
  Stack,
  Badge,
} from '@mui/material';
import {
  ZoomIn as ZoomInIcon,
  ZoomOut as ZoomOutIcon,
  CenterFocusStrong as CenterIcon,
  Fullscreen as FullscreenIcon,
  Settings as SettingsIcon,
  Refresh as RefreshIcon,
  Sync as SyncIcon,
  Notifications as NotificationsIcon,
  Timeline as TimelineIcon,
} from '@mui/icons-material';
import ReactFlow, {
  Controls,
  Background,
  useNodesState,
  useEdgesState,
  useReactFlow,
  ReactFlowProvider,
  ConnectionMode,
  Panel,
  MiniMap,
  SelectionMode,
  MarkerType,
} from 'reactflow';
import type {
  Node,
  Edge,
  NodeChange,
  EdgeChange,
} from 'reactflow';
import 'reactflow/dist/style.css';

import { useRealTime } from './RealTimeProvider';
import GraphUpdateHandler from './GraphUpdateHandler';
import ConnectionStatus from './ConnectionStatus';
import UpdateNotifications from './UpdateNotifications';
import SyncManager from './SyncManager';
import ConflictResolver from './ConflictResolver';

import type { 
  NodeResponse, 
  EdgeResponse, 
  GraphViewState, 
  GraphLayoutOptions, 
  GraphStyleOptions,
  NodeId,
  EdgeId,
} from '../../types/graph';
import type { ConflictDetectedEvent } from '../../types/realtime';

// Custom Node Components with real-time indicators
const RealTimeFileNode = ({ data, selected }: { data: any; selected: boolean }) => {
  const theme = useTheme();
  const isRecentlyUpdated = data.lastUpdated && 
    Date.now() - new Date(data.lastUpdated).getTime() < 5000; // 5 seconds
  
  return (
    <Box
      sx={{
        padding: 1,
        borderRadius: 1,
        backgroundColor: selected ? theme.palette.primary.light : theme.palette.background.paper,
        border: `2px solid ${
          isRecentlyUpdated ? theme.palette.success.main :
          selected ? theme.palette.primary.main : theme.palette.divider
        }`,
        minWidth: 120,
        textAlign: 'center',
        boxShadow: selected ? 2 : 1,
        transition: 'all 0.2s ease-in-out',
        position: 'relative',
        '&:hover': {
          boxShadow: 3,
          transform: 'scale(1.05)',
        },
        ...(isRecentlyUpdated && {
          animation: 'pulse 2s infinite',
          '@keyframes pulse': {
            '0%': { boxShadow: `0 0 0 0 ${theme.palette.success.main}40` },
            '70%': { boxShadow: `0 0 0 10px ${theme.palette.success.main}00` },
            '100%': { boxShadow: `0 0 0 0 ${theme.palette.success.main}00` },
          },
        }),
      }}
    >
      {isRecentlyUpdated && (
        <Box
          sx={{
            position: 'absolute',
            top: -4,
            right: -4,
            width: 8,
            height: 8,
            borderRadius: '50%',
            backgroundColor: theme.palette.success.main,
          }}
        />
      )}
      
      <Typography variant="caption" color={selected ? 'primary.contrastText' : 'text.primary'}>
        {data.node_type}
      </Typography>
      <Typography variant="body2" fontWeight="bold" color={selected ? 'primary.contrastText' : 'text.primary'}>
        {data.label || `Node ${data.id}`}
      </Typography>
      {data.properties && Object.keys(data.properties).length > 0 && (
        <Chip
          size="small"
          label={`${Object.keys(data.properties).length} props`}
          sx={{ mt: 0.5, fontSize: '0.6rem', height: 16 }}
        />
      )}
    </Box>
  );
};

const RealTimeDirectoryNode = ({ data, selected }: { data: any; selected: boolean }) => {
  const theme = useTheme();
  const isRecentlyUpdated = data.lastUpdated && 
    Date.now() - new Date(data.lastUpdated).getTime() < 5000;
  
  return (
    <Box
      sx={{
        padding: 1.5,
        borderRadius: 2,
        backgroundColor: selected ? theme.palette.secondary.light : theme.palette.background.paper,
        border: `2px solid ${
          isRecentlyUpdated ? theme.palette.success.main :
          selected ? theme.palette.secondary.main : theme.palette.divider
        }`,
        minWidth: 140,
        textAlign: 'center',
        boxShadow: selected ? 3 : 2,
        transition: 'all 0.2s ease-in-out',
        position: 'relative',
        '&:hover': {
          boxShadow: 4,
          transform: 'scale(1.05)',
        },
        ...(isRecentlyUpdated && {
          animation: 'pulse 2s infinite',
        }),
      }}
    >
      {isRecentlyUpdated && (
        <Box
          sx={{
            position: 'absolute',
            top: -4,
            right: -4,
            width: 8,
            height: 8,
            borderRadius: '50%',
            backgroundColor: theme.palette.success.main,
          }}
        />
      )}
      
      <Typography variant="caption" color={selected ? 'secondary.contrastText' : 'text.secondary'}>
        {data.node_type}
      </Typography>
      <Typography variant="body2" fontWeight="bold" color={selected ? 'secondary.contrastText' : 'text.primary'}>
        {data.label || `Directory ${data.id}`}
      </Typography>
      {data.children_count && (
        <Chip
          size="small"
          label={`${data.children_count} items`}
          sx={{ mt: 0.5, fontSize: '0.6rem', height: 16 }}
        />
      )}
    </Box>
  );
};

// Node type mapping
const nodeTypes = {
  File: RealTimeFileNode,
  Directory: RealTimeDirectoryNode,
  Symlink: RealTimeFileNode,
  Device: RealTimeFileNode,
  Custom: RealTimeFileNode,
};

// Props interface
export interface RealTimeGraphVisualizationProps {
  nodes: NodeResponse[];
  edges: EdgeResponse[];
  onNodeSelect?: (nodeIds: string[]) => void;
  onEdgeSelect?: (edgeIds: string[]) => void;
  onNodeDoubleClick?: (node: NodeResponse) => void;
  onEdgeDoubleClick?: (edge: EdgeResponse) => void;
  layout?: GraphLayoutOptions;
  style?: GraphStyleOptions;
  isLoading?: boolean;
  enableRealTimeUpdates?: boolean;
  enableOptimisticUpdates?: boolean;
  showConnectionStatus?: boolean;
  showNotifications?: boolean;
  showSyncManager?: boolean;
}

const RealTimeGraphVisualizationInner: React.FC<RealTimeGraphVisualizationProps> = ({
  nodes: initialNodes,
  edges: initialEdges,
  onNodeSelect,
  onEdgeSelect,
  onNodeDoubleClick,
  onEdgeDoubleClick,
  layout = { name: 'cose', animate: true, fit: true, padding: 50 },
  style = { nodeColor: '#1976d2', edgeColor: '#666', nodeSize: 30 },
  isLoading = false,
  enableRealTimeUpdates = true,
  enableOptimisticUpdates = true,
  showConnectionStatus = true,
  showNotifications = true,
  showSyncManager = true,
}) => {
  const theme = useTheme();
  const { fitView, zoomIn, zoomOut, setCenter } = useReactFlow();
  const { state } = useRealTime();
  
  // Local state for nodes and edges with real-time updates
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [selectedNodes, setSelectedNodes] = useState<string[]>([]);
  const [selectedEdges, setSelectedEdges] = useState<string[]>([]);
  const [viewState, setViewState] = useState<GraphViewState>({
    nodes: [],
    edges: [],
    selectedNodes: [],
    selectedEdges: [],
    layout: layout || { name: 'cose' },
    style: style || {},
    isLoading: false,
  });
  
  // Conflict resolution
  const [conflictDialogOpen, setConflictDialogOpen] = useState(false);
  const [currentConflict, setCurrentConflict] = useState<ConflictDetectedEvent | null>(null);
  
  // Real-time update tracking
  const nodeUpdateTimestamps = useRef<Map<NodeId, Date>>(new Map());
  const edgeUpdateTimestamps = useRef<Map<EdgeId, Date>>(new Map());

  // Convert backend nodes/edges to React Flow format
  const convertNodesToFlow = useCallback((backendNodes: NodeResponse[]): Node[] => {
    return backendNodes.map((node, index) => ({
      id: node.id,
      type: node.node_type,
      position: { 
        x: (index % 5) * 200, 
        y: Math.floor(index / 5) * 150 
      },
      data: {
        ...node,
        label: `${node.node_type} ${node.id}`,
        lastUpdated: nodeUpdateTimestamps.current.get(node.id),
      },
      draggable: true,
    }));
  }, []);

  const convertEdgesToFlow = useCallback((backendEdges: EdgeResponse[]): Edge[] => {
    return backendEdges.map((edge) => ({
      id: edge.id,
      source: edge.source_id,
      target: edge.target_id,
      type: 'default',
      label: edge.edge_type,
      markerEnd: {
        type: MarkerType.ArrowClosed,
        width: 20,
        height: 20,
      },
      data: {
        ...edge,
        lastUpdated: edgeUpdateTimestamps.current.get(edge.id),
      },
      style: {
        stroke: theme.palette.primary.main,
        strokeWidth: 2,
      },
    }));
  }, [theme]);

  // Initialize nodes and edges
  useEffect(() => {
    const flowNodes = convertNodesToFlow(initialNodes);
    const flowEdges = convertEdgesToFlow(initialEdges);
    setNodes(flowNodes);
    setEdges(flowEdges);
  }, [initialNodes, initialEdges, convertNodesToFlow, convertEdgesToFlow]);

  // Real-time event handlers
  const handleNodeCreated = useCallback((node: NodeResponse) => {
    nodeUpdateTimestamps.current.set(node.id, new Date());
    const flowNode = convertNodesToFlow([node])[0];
    setNodes(prev => [...prev, flowNode]);
  }, [convertNodesToFlow]);

  const handleNodeUpdated = useCallback((nodeId: NodeId, node: NodeResponse, changes: Record<string, any>) => {
    nodeUpdateTimestamps.current.set(nodeId, new Date());
    setNodes(prev => prev.map(n => 
      n.id === nodeId 
        ? { 
            ...n, 
            data: { 
              ...n.data, 
              ...node, 
              lastUpdated: new Date(),
            } 
          }
        : n
    ));
  }, []);

  const handleNodeDeleted = useCallback((nodeId: NodeId, affectedEdges: EdgeId[]) => {
    setNodes(prev => prev.filter(n => n.id !== nodeId));
    setEdges(prev => prev.filter(e => !affectedEdges.includes(e.id)));
    nodeUpdateTimestamps.current.delete(nodeId);
    affectedEdges.forEach(edgeId => edgeUpdateTimestamps.current.delete(edgeId));
  }, []);

  const handleEdgeCreated = useCallback((edge: EdgeResponse) => {
    edgeUpdateTimestamps.current.set(edge.id, new Date());
    const flowEdge = convertEdgesToFlow([edge])[0];
    setEdges(prev => [...prev, flowEdge]);
  }, [convertEdgesToFlow]);

  const handleEdgeUpdated = useCallback((edgeId: EdgeId, edge: EdgeResponse, changes: Record<string, any>) => {
    edgeUpdateTimestamps.current.set(edgeId, new Date());
    setEdges(prev => prev.map(e => 
      e.id === edgeId 
        ? { 
            ...e, 
            data: { 
              ...e.data, 
              ...edge, 
              lastUpdated: new Date(),
            } 
          }
        : e
    ));
  }, []);

  const handleEdgeDeleted = useCallback((edgeId: EdgeId, sourceId: NodeId, targetId: NodeId) => {
    setEdges(prev => prev.filter(e => e.id !== edgeId));
    edgeUpdateTimestamps.current.delete(edgeId);
  }, []);

  // Handle conflicts
  useEffect(() => {
    if (state.pendingConflicts.length > 0 && !currentConflict) {
      setCurrentConflict(state.pendingConflicts[0]);
      setConflictDialogOpen(true);
    }
  }, [state.pendingConflicts, currentConflict]);

  const handleConflictResolution = useCallback((resolution: any) => {
    setConflictDialogOpen(false);
    setCurrentConflict(null);
    // Apply resolution to the graph
    console.log('Conflict resolved:', resolution);
  }, []);

  // Selection handlers
  const handleSelectionChange = useCallback<any>((params: any) => {
    const nodeIds = params.nodes || [];
    const edgeIds = params.edges || [];
    
    setSelectedNodes(nodeIds);
    setSelectedEdges(edgeIds);
    
    if (onNodeSelect) {
      onNodeSelect(nodeIds);
    }
    if (onEdgeSelect) {
      onEdgeSelect(edgeIds);
    }
  }, [onNodeSelect, onEdgeSelect]);

  // Layout controls
  const handleFitView = useCallback(() => {
    fitView({ padding: 0.2 });
  }, [fitView]);

  const handleZoomIn = useCallback(() => {
    zoomIn();
  }, [zoomIn]);

  const handleZoomOut = useCallback(() => {
    zoomOut();
  }, [zoomOut]);

  const handleCenter = useCallback(() => {
    setCenter(0, 0, { zoom: 1 });
  }, [setCenter]);

  if (isLoading) {
    return (
      <Box display="flex" justifyContent="center" alignItems="center" height={400}>
        <CircularProgress />
        <Typography variant="body2" sx={{ ml: 2 }}>
          Loading graph...
        </Typography>
      </Box>
    );
  }

  return (
    <Paper elevation={2} sx={{ height: '100%', position: 'relative' }}>
      {/* Real-time Update Handler */}
      {enableRealTimeUpdates && (
        <GraphUpdateHandler
          onNodeCreated={handleNodeCreated}
          onNodeUpdated={handleNodeUpdated}
          onNodeDeleted={handleNodeDeleted}
          onEdgeCreated={handleEdgeCreated}
          onEdgeUpdated={handleEdgeUpdated}
          onEdgeDeleted={handleEdgeDeleted}
          enableOptimisticUpdates={enableOptimisticUpdates}
        />
      )}

      {/* Top Toolbar */}
      <Toolbar variant="dense" sx={{ borderBottom: 1, borderColor: 'divider' }}>
        <Typography variant="h6" sx={{ flexGrow: 1 }}>
          VexGraph Real-time Visualization
        </Typography>
        
        <Stack direction="row" spacing={1} alignItems="center">
          {/* Connection Status */}
          {showConnectionStatus && (
            <ConnectionStatus compact />
          )}
          
          {/* Notifications */}
          {showNotifications && (
            <UpdateNotifications />
          )}
          
          {/* Layout Controls */}
          <ButtonGroup size="small">
            <Tooltip title="Zoom In">
              <IconButton onClick={handleZoomIn}>
                <ZoomInIcon />
              </IconButton>
            </Tooltip>
            <Tooltip title="Zoom Out">
              <IconButton onClick={handleZoomOut}>
                <ZoomOutIcon />
              </IconButton>
            </Tooltip>
            <Tooltip title="Fit View">
              <IconButton onClick={handleFitView}>
                <CenterIcon />
              </IconButton>
            </Tooltip>
            <Tooltip title="Center">
              <IconButton onClick={handleCenter}>
                <FullscreenIcon />
              </IconButton>
            </Tooltip>
          </ButtonGroup>
        </Stack>
      </Toolbar>

      {/* Main Graph Area */}
      <Box sx={{ height: 'calc(100% - 48px)', position: 'relative' }}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onSelectionChange={handleSelectionChange}
          nodeTypes={nodeTypes}
          connectionMode={ConnectionMode.Loose}
          selectionMode={SelectionMode.Partial}
          fitView
          attributionPosition="bottom-left"
        >
          <Background />
          <Controls />
          <MiniMap 
            nodeStrokeWidth={3}
            nodeColor={(node) => {
              const isRecent = node.data?.lastUpdated && 
                Date.now() - new Date(node.data.lastUpdated).getTime() < 5000;
              return isRecent ? theme.palette.success.main : theme.palette.primary.main;
            }}
          />
          
          {/* Real-time Status Panel */}
          <Panel position="top-right">
            <Stack spacing={1}>
              {state.connectionStatus.state === 'connected' && (
                <Chip
                  icon={<SyncIcon />}
                  label="Live Updates"
                  color="success"
                  size="small"
                />
              )}
              
              {state.updateQueue.length > 0 && (
                <Chip
                  icon={<TimelineIcon />}
                  label={`${state.updateQueue.length} pending`}
                  color="info"
                  size="small"
                />
              )}
              
              {state.pendingConflicts.length > 0 && (
                <Chip
                  label={`${state.pendingConflicts.length} conflicts`}
                  color="error"
                  size="small"
                />
              )}
            </Stack>
          </Panel>
          
          {/* Sync Manager Panel */}
          {showSyncManager && (
            <Panel position="bottom-right">
              <Box sx={{ width: 300 }}>
                <SyncManager />
              </Box>
            </Panel>
          )}
        </ReactFlow>
      </Box>

      {/* Conflict Resolution Dialog */}
      <ConflictResolver
        open={conflictDialogOpen}
        conflict={currentConflict}
        onResolve={handleConflictResolution}
        onCancel={() => {
          setConflictDialogOpen(false);
          setCurrentConflict(null);
        }}
      />
    </Paper>
  );
};

// Wrapper component with ReactFlowProvider
const RealTimeGraphVisualization: React.FC<RealTimeGraphVisualizationProps> = (props) => {
  return (
    <ReactFlowProvider>
      <RealTimeGraphVisualizationInner {...props} />
    </ReactFlowProvider>
  );
};

export default RealTimeGraphVisualization;