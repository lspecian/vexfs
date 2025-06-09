import React, { useState, useCallback } from 'react';
import {
  Box,
  Paper,
  Typography,
  Button,
  ButtonGroup,
  Chip,
  Alert,
} from '@mui/material';
import {
  PlayArrow as PlayIcon,
  Stop as StopIcon,
  Shuffle as ShuffleIcon,
} from '@mui/icons-material';

import { GraphVisualization } from './GraphVisualization';
import type { NodeResponse, EdgeResponse, PropertyType } from '../../types/graph';

// Mock data generator
const generateMockData = (nodeCount: number = 10, edgeCount: number = 15) => {
  const nodeTypes = ['File', 'Directory', 'Symlink', 'Device'] as const;
  const edgeTypes = ['Contains', 'References', 'DependsOn', 'SimilarTo'] as const;

  // Generate nodes
  const nodes: NodeResponse[] = Array.from({ length: nodeCount }, (_, i) => ({
    id: `node-${i + 1}`,
    inode_number: 1000 + i,
    node_type: nodeTypes[Math.floor(Math.random() * nodeTypes.length)],
    properties: {
      name: 'String' as PropertyType,
      path: 'String' as PropertyType,
      size: 'Integer' as PropertyType,
    },
    outgoing_edges: [],
    incoming_edges: [],
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  }));

  // Generate edges
  const edges: EdgeResponse[] = Array.from({ length: edgeCount }, (_, i) => {
    const sourceIndex = Math.floor(Math.random() * nodeCount);
    let targetIndex = Math.floor(Math.random() * nodeCount);
    
    // Ensure source and target are different
    while (targetIndex === sourceIndex) {
      targetIndex = Math.floor(Math.random() * nodeCount);
    }

    return {
      id: `edge-${i + 1}`,
      source_id: nodes[sourceIndex].id,
      target_id: nodes[targetIndex].id,
      edge_type: edgeTypes[Math.floor(Math.random() * edgeTypes.length)],
      weight: Math.random() * 10,
      properties: {
        relationship: 'String' as PropertyType,
      },
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
  });

  // Update node edge references
  edges.forEach(edge => {
    const sourceNode = nodes.find(n => n.id === edge.source_id);
    const targetNode = nodes.find(n => n.id === edge.target_id);
    
    if (sourceNode) {
      sourceNode.outgoing_edges.push(edge.id);
    }
    if (targetNode) {
      targetNode.incoming_edges.push(edge.id);
    }
  });

  return { nodes, edges };
};

export const GraphDemo: React.FC = () => {
  const [mockData, setMockData] = useState(() => generateMockData());
  const [selectedNodes, setSelectedNodes] = useState<string[]>([]);
  const [selectedEdges, setSelectedEdges] = useState<string[]>([]);
  const [isAnimating, setIsAnimating] = useState(false);

  const handleGenerateNew = useCallback(() => {
    const nodeCount = 8 + Math.floor(Math.random() * 12); // 8-20 nodes
    const edgeCount = Math.floor(nodeCount * 1.5); // ~1.5 edges per node
    setMockData(generateMockData(nodeCount, edgeCount));
    setSelectedNodes([]);
    setSelectedEdges([]);
  }, []);

  const handleNodeSelect = useCallback((nodeIds: string[]) => {
    setSelectedNodes(nodeIds);
  }, []);

  const handleEdgeSelect = useCallback((edgeIds: string[]) => {
    setSelectedEdges(edgeIds);
  }, []);

  const handleNodeDoubleClick = useCallback((node: NodeResponse) => {
    console.log('Demo: Node double-clicked:', node);
  }, []);

  const handleEdgeDoubleClick = useCallback((edge: EdgeResponse) => {
    console.log('Demo: Edge double-clicked:', edge);
  }, []);

  const handleToggleAnimation = useCallback(() => {
    setIsAnimating(prev => !prev);
  }, []);

  // Helper function to get display name for nodes
  const getNodeDisplayName = (node: NodeResponse) => {
    return `${node.node_type} ${node.id}`;
  };

  return (
    <Box>
      {/* Demo Controls */}
      <Paper sx={{ p: 2, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
          <Typography variant="h6">
            Graph Visualization Demo
          </Typography>
          <ButtonGroup variant="outlined" size="small">
            <Button
              startIcon={<ShuffleIcon />}
              onClick={handleGenerateNew}
            >
              Generate New
            </Button>
            <Button
              startIcon={isAnimating ? <StopIcon /> : <PlayIcon />}
              onClick={handleToggleAnimation}
            >
              {isAnimating ? 'Stop' : 'Animate'}
            </Button>
          </ButtonGroup>
        </Box>

        <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
          <Chip
            label={`${mockData.nodes.length} Nodes`}
            color="primary"
            variant="outlined"
          />
          <Chip
            label={`${mockData.edges.length} Edges`}
            color="secondary"
            variant="outlined"
          />
          {selectedNodes.length > 0 && (
            <Chip
              label={`${selectedNodes.length} Selected Nodes`}
              color="primary"
            />
          )}
          {selectedEdges.length > 0 && (
            <Chip
              label={`${selectedEdges.length} Selected Edges`}
              color="secondary"
            />
          )}
        </Box>
      </Paper>

      {/* Demo Info */}
      <Alert severity="info" sx={{ mb: 3 }}>
        <Typography variant="body2">
          <strong>Demo Features:</strong> This demonstration shows the core graph visualization capabilities.
          Try clicking nodes/edges to select them, double-clicking for details, and using the zoom/pan controls.
          The "Generate New" button creates a fresh random graph structure.
        </Typography>
      </Alert>

      {/* Graph Visualization */}
      <GraphVisualization
        nodes={mockData.nodes}
        edges={mockData.edges}
        onNodeSelect={handleNodeSelect}
        onEdgeSelect={handleEdgeSelect}
        onNodeDoubleClick={handleNodeDoubleClick}
        onEdgeDoubleClick={handleEdgeDoubleClick}
        height={600}
        enableMiniMap={true}
        enableControls={true}
        enableBackground={true}
        layout={{
          name: 'cose',
          animate: isAnimating,
          fit: true,
          padding: 50,
        }}
        style={{
          nodeColor: '#1976d2',
          edgeColor: '#666',
          selectedNodeColor: '#9c27b0',
          selectedEdgeColor: '#9c27b0',
        }}
      />

      {/* Selection Details */}
      {(selectedNodes.length > 0 || selectedEdges.length > 0) && (
        <Paper sx={{ p: 2, mt: 3 }}>
          <Typography variant="h6" gutterBottom>
            Selection Details
          </Typography>
          
          {selectedNodes.length > 0 && (
            <Box sx={{ mb: 2 }}>
              <Typography variant="subtitle2" color="primary" gutterBottom>
                Selected Nodes:
              </Typography>
              {selectedNodes.map(nodeId => {
                const node = mockData.nodes.find(n => n.id === nodeId);
                return node ? (
                  <Box key={nodeId} sx={{ ml: 2, mb: 1 }}>
                    <Typography variant="body2">
                      <strong>{getNodeDisplayName(node)}</strong>
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
                      Inode: {node.inode_number} | Properties: {Object.keys(node.properties).length}
                    </Typography>
                  </Box>
                ) : null;
              })}
            </Box>
          )}

          {selectedEdges.length > 0 && (
            <Box>
              <Typography variant="subtitle2" color="secondary" gutterBottom>
                Selected Edges:
              </Typography>
              {selectedEdges.map(edgeId => {
                const edge = mockData.edges.find(e => e.id === edgeId);
                const sourceNode = mockData.nodes.find(n => n.id === edge?.source_id);
                const targetNode = mockData.nodes.find(n => n.id === edge?.target_id);
                
                return edge ? (
                  <Box key={edgeId} sx={{ ml: 2, mb: 1 }}>
                    <Typography variant="body2">
                      <strong>{edge.edge_type}</strong> (Weight: {edge.weight.toFixed(2)})
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
                      {sourceNode ? getNodeDisplayName(sourceNode) : edge.source_id} → {targetNode ? getNodeDisplayName(targetNode) : edge.target_id}
                    </Typography>
                  </Box>
                ) : null;
              })}
            </Box>
          )}
        </Paper>
      )}
    </Box>
  );
};

export default GraphDemo;