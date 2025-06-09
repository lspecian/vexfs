import React, { useCallback, useEffect, useMemo, useState } from 'react';
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
} from '@mui/material';
import {
  ZoomIn as ZoomInIcon,
  ZoomOut as ZoomOutIcon,
  CenterFocusStrong as CenterIcon,
  Fullscreen as FullscreenIcon,
  Settings as SettingsIcon,
  Refresh as RefreshIcon,
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
  NodeTypes,
  EdgeTypes,
  OnNodesChange,
  OnEdgesChange,
  NodeMouseHandler,
  EdgeMouseHandler,
  OnSelectionChangeFunc,
} from 'reactflow';
import 'reactflow/dist/style.css';

import type { NodeResponse, EdgeResponse, GraphViewState, GraphLayoutOptions, GraphStyleOptions } from '../../types/graph';
import { vexfsColors } from '../../theme';

// Custom Node Components
const FileNode = ({ data, selected }: { data: any; selected: boolean }) => {
  const theme = useTheme();
  
  return (
    <Box
      sx={{
        padding: 1,
        borderRadius: 1,
        backgroundColor: selected ? theme.palette.primary.light : theme.palette.background.paper,
        border: `2px solid ${selected ? theme.palette.primary.main : theme.palette.divider}`,
        minWidth: 120,
        textAlign: 'center',
        boxShadow: selected ? 2 : 1,
        transition: 'all 0.2s ease-in-out',
        '&:hover': {
          boxShadow: 3,
          transform: 'scale(1.05)',
        },
      }}
    >
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

const DirectoryNode = ({ data, selected }: { data: any; selected: boolean }) => {
  const theme = useTheme();
  
  return (
    <Box
      sx={{
        padding: 1.5,
        borderRadius: 2,
        backgroundColor: selected ? theme.palette.secondary.light : theme.palette.background.paper,
        border: `2px solid ${selected ? theme.palette.secondary.main : theme.palette.divider}`,
        minWidth: 140,
        textAlign: 'center',
        boxShadow: selected ? 3 : 2,
        transition: 'all 0.2s ease-in-out',
        '&:hover': {
          boxShadow: 4,
          transform: 'scale(1.05)',
        },
      }}
    >
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
const nodeTypes: NodeTypes = {
  File: FileNode,
  Directory: DirectoryNode,
  Symlink: FileNode,
  Device: FileNode,
  Custom: FileNode,
};

// Edge types - using default React Flow edges with custom styling
const edgeTypes: EdgeTypes = {};

// Props interface
export interface GraphVisualizationProps {
  nodes: NodeResponse[];
  edges: EdgeResponse[];
  onNodeSelect?: (nodeIds: string[]) => void;
  onEdgeSelect?: (edgeIds: string[]) => void;
  onNodeDoubleClick?: (node: NodeResponse) => void;
  onEdgeDoubleClick?: (edge: EdgeResponse) => void;
  layout?: GraphLayoutOptions;
  style?: GraphStyleOptions;
  isLoading?: boolean;
  error?: string;
  height?: number | string;
  enableMiniMap?: boolean;
  enableControls?: boolean;
  enableBackground?: boolean;
  className?: string;
}

// Main component implementation
const GraphVisualizationInner: React.FC<GraphVisualizationProps> = ({
  nodes: nodeData,
  edges: edgeData,
  onNodeSelect,
  onEdgeSelect,
  onNodeDoubleClick,
  onEdgeDoubleClick,
  layout = { name: 'cose', animate: true, fit: true },
  style = {},
  isLoading = false,
  error,
  height = 600,
  enableMiniMap = true,
  enableControls = true,
  enableBackground = true,
  className,
}) => {
  const theme = useTheme();
  const { fitView, zoomIn, zoomOut, setCenter } = useReactFlow();
  
  // Convert VexGraph data to React Flow format
  const reactFlowNodes: Node[] = useMemo(() => {
    return nodeData.map((node) => ({
      id: node.id,
      type: node.node_type,
      position: { x: Math.random() * 500, y: Math.random() * 500 }, // TODO: Use layout algorithm
      data: {
        ...node,
        label: node.properties?.name || node.properties?.path || `Node ${node.id}`,
      },
      style: {
        backgroundColor: style.nodeColor || theme.palette.background.paper,
        border: `2px solid ${style.nodeColor || theme.palette.divider}`,
        borderRadius: 8,
      },
    }));
  }, [nodeData, style.nodeColor, theme]);

  const reactFlowEdges: Edge[] = useMemo(() => {
    return edgeData.map((edge) => ({
      id: edge.id,
      source: edge.source_id,
      target: edge.target_id,
      type: 'default',
      data: {
        ...edge,
        label: edge.edge_type,
      },
      style: {
        stroke: style.edgeColor || theme.palette.text.secondary,
        strokeWidth: style.edgeWidth || 2,
      },
      animated: false,
      markerEnd: {
        type: MarkerType.ArrowClosed,
        color: style.edgeColor || theme.palette.text.secondary,
      },
      label: edge.edge_type,
    }));
  }, [edgeData, style.edgeColor, style.edgeWidth, theme]);

  const [nodes, setNodes, onNodesChange] = useNodesState(reactFlowNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(reactFlowEdges);
  const [selectedNodes, setSelectedNodes] = useState<string[]>([]);
  const [selectedEdges, setSelectedEdges] = useState<string[]>([]);

  // Update nodes and edges when data changes
  useEffect(() => {
    setNodes(reactFlowNodes);
  }, [reactFlowNodes, setNodes]);

  useEffect(() => {
    setEdges(reactFlowEdges);
  }, [reactFlowEdges, setEdges]);

  // Event handlers
  const handleNodeClick: NodeMouseHandler = useCallback((event, node) => {
    const nodeId = node.id;
    const newSelection = selectedNodes.includes(nodeId)
      ? selectedNodes.filter(id => id !== nodeId)
      : [...selectedNodes, nodeId];
    
    setSelectedNodes(newSelection);
    onNodeSelect?.(newSelection);
  }, [selectedNodes, onNodeSelect]);

  const handleEdgeClick: EdgeMouseHandler = useCallback((event, edge) => {
    const edgeId = edge.id;
    const newSelection = selectedEdges.includes(edgeId)
      ? selectedEdges.filter(id => id !== edgeId)
      : [...selectedEdges, edgeId];
    
    setSelectedEdges(newSelection);
    onEdgeSelect?.(newSelection);
  }, [selectedEdges, onEdgeSelect]);

  const handleNodeDoubleClick: NodeMouseHandler = useCallback((event, node) => {
    const originalNode = nodeData.find(n => n.id === node.id);
    if (originalNode) {
      onNodeDoubleClick?.(originalNode);
    }
  }, [nodeData, onNodeDoubleClick]);

  const handleSelectionChange: OnSelectionChangeFunc = useCallback(({ nodes, edges }) => {
    const nodeIds = nodes.map(n => n.id);
    const edgeIds = edges.map(e => e.id);
    
    setSelectedNodes(nodeIds);
    setSelectedEdges(edgeIds);
    
    onNodeSelect?.(nodeIds);
    onEdgeSelect?.(edgeIds);
  }, [onNodeSelect, onEdgeSelect]);

  // Toolbar actions
  const handleFitView = useCallback(() => {
    fitView({ duration: 800 });
  }, [fitView]);

  const handleZoomIn = useCallback(() => {
    zoomIn({ duration: 200 });
  }, [zoomIn]);

  const handleZoomOut = useCallback(() => {
    zoomOut({ duration: 200 });
  }, [zoomOut]);

  const handleCenter = useCallback(() => {
    setCenter(250, 250, { zoom: 1, duration: 800 });
  }, [setCenter]);

  // Loading state
  if (isLoading) {
    return (
      <Paper sx={{ height, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Box textAlign="center">
          <CircularProgress size={48} />
          <Typography variant="body2" sx={{ mt: 2 }}>
            Loading graph data...
          </Typography>
        </Box>
      </Paper>
    );
  }

  // Error state
  if (error) {
    return (
      <Paper sx={{ height, p: 2 }}>
        <Alert severity="error" sx={{ height: '100%' }}>
          <Typography variant="h6">Graph Visualization Error</Typography>
          <Typography variant="body2">{error}</Typography>
        </Alert>
      </Paper>
    );
  }

  return (
    <Paper
      sx={{
        height,
        position: 'relative',
        overflow: 'hidden',
        backgroundColor: style.backgroundColor || theme.palette.background.default,
      }}
      className={className}
    >
      {/* Toolbar */}
      <Panel position="top-left">
        <Paper sx={{ p: 1 }}>
          <Toolbar variant="dense" sx={{ minHeight: 'auto', gap: 1 }}>
            <ButtonGroup size="small" variant="outlined">
              <Tooltip title="Zoom In">
                <IconButton onClick={handleZoomIn} size="small">
                  <ZoomInIcon fontSize="small" />
                </IconButton>
              </Tooltip>
              <Tooltip title="Zoom Out">
                <IconButton onClick={handleZoomOut} size="small">
                  <ZoomOutIcon fontSize="small" />
                </IconButton>
              </Tooltip>
              <Tooltip title="Fit View">
                <IconButton onClick={handleFitView} size="small">
                  <CenterIcon fontSize="small" />
                </IconButton>
              </Tooltip>
              <Tooltip title="Center">
                <IconButton onClick={handleCenter} size="small">
                  <FullscreenIcon fontSize="small" />
                </IconButton>
              </Tooltip>
            </ButtonGroup>
            
            <Tooltip title="Refresh Layout">
              <IconButton size="small">
                <RefreshIcon fontSize="small" />
              </IconButton>
            </Tooltip>
            
            <Tooltip title="Settings">
              <IconButton size="small">
                <SettingsIcon fontSize="small" />
              </IconButton>
            </Tooltip>
          </Toolbar>
        </Paper>
      </Panel>

      {/* Stats Panel */}
      <Panel position="top-right">
        <Paper sx={{ p: 1 }}>
          <Typography variant="caption" display="block">
            Nodes: {nodes.length}
          </Typography>
          <Typography variant="caption" display="block">
            Edges: {edges.length}
          </Typography>
          {selectedNodes.length > 0 && (
            <Typography variant="caption" display="block" color="primary">
              Selected: {selectedNodes.length} nodes
            </Typography>
          )}
          {selectedEdges.length > 0 && (
            <Typography variant="caption" display="block" color="secondary">
              Selected: {selectedEdges.length} edges
            </Typography>
          )}
        </Paper>
      </Panel>

      {/* React Flow */}
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={handleNodeClick}
        onEdgeClick={handleEdgeClick}
        onNodeDoubleClick={handleNodeDoubleClick}
        onSelectionChange={handleSelectionChange}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        connectionMode={ConnectionMode.Loose}
        selectionMode={SelectionMode.Partial}
        multiSelectionKeyCode="Shift"
        deleteKeyCode="Delete"
        fitView
        attributionPosition="bottom-left"
        proOptions={{ hideAttribution: true }}
      >
        {enableBackground && (
          <Background
            color={theme.palette.divider}
            gap={20}
            size={1}
          />
        )}
        
        {enableControls && <Controls showInteractive={false} />}
        
        {enableMiniMap && (
          <MiniMap
            nodeColor={theme.palette.primary.main}
            nodeStrokeWidth={2}
            zoomable
            pannable
            style={{
              backgroundColor: theme.palette.background.paper,
              border: `1px solid ${theme.palette.divider}`,
            }}
          />
        )}
      </ReactFlow>
    </Paper>
  );
};

// Wrapper component with ReactFlowProvider
export const GraphVisualization: React.FC<GraphVisualizationProps> = (props) => {
  return (
    <ReactFlowProvider>
      <GraphVisualizationInner {...props} />
    </ReactFlowProvider>
  );
};

export default GraphVisualization;