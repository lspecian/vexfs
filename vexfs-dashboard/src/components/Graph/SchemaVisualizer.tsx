import React, { useState, useEffect, useRef } from 'react';
import {
  Box,
  Typography,
  Card,
  CardContent,
  Grid,
  Chip,
  IconButton,
  Tooltip,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Switch,
  FormControlLabel,
  Paper,
  List,
  ListItem,
  ListItemText,
  ListItemIcon,
  Divider,
} from '@mui/material';
import {
  ZoomIn as ZoomInIcon,
  ZoomOut as ZoomOutIcon,
  CenterFocusStrong as CenterIcon,
  Download as DownloadIcon,
  Visibility as ViewIcon,
  VisibilityOff as HideIcon,
  AccountTree as NodeIcon,
  Timeline as EdgeIcon,
  Schema as SchemaIcon,
} from '@mui/icons-material';
import type { NodeTypeDefinition, EdgeTypeDefinition } from '../../types/schema';

interface SchemaVisualizerProps {
  nodeTypes: NodeTypeDefinition[];
  edgeTypes: EdgeTypeDefinition[];
  onNodeTypeSelect?: (nodeTypeId: string) => void;
  onEdgeTypeSelect?: (edgeTypeId: string) => void;
}

interface VisualizationNode {
  id: string;
  label: string;
  type: 'node-type' | 'edge-type';
  color: string;
  properties: number;
  x?: number;
  y?: number;
}

interface VisualizationEdge {
  id: string;
  source: string;
  target: string;
  label: string;
  type: 'allows' | 'inherits';
}

const SchemaVisualizer: React.FC<SchemaVisualizerProps> = ({
  nodeTypes,
  edgeTypes,
  onNodeTypeSelect,
  onEdgeTypeSelect,
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [viewMode, setViewMode] = useState<'diagram' | 'matrix' | 'list'>('diagram');
  const [showProperties, setShowProperties] = useState(true);
  const [showInheritance, setShowInheritance] = useState(true);
  const [selectedItem, setSelectedItem] = useState<string | null>(null);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });

  const visualizationData = React.useMemo(() => {
    const nodes: VisualizationNode[] = [
      ...nodeTypes.map(nt => ({
        id: nt.id,
        label: nt.displayName,
        type: 'node-type' as const,
        color: nt.color || '#1976d2',
        properties: nt.properties.length,
      })),
      ...edgeTypes.map(et => ({
        id: et.id,
        label: et.displayName,
        type: 'edge-type' as const,
        color: '#9c27b0',
        properties: et.properties.length,
      })),
    ];

    const edges: VisualizationEdge[] = [];

    // Add inheritance relationships
    nodeTypes.forEach(nt => {
      if (nt.inheritFrom) {
        nt.inheritFrom.forEach(parentId => {
          edges.push({
            id: `inherit-${nt.id}-${parentId}`,
            source: parentId,
            target: nt.id,
            label: 'inherits',
            type: 'inherits',
          });
        });
      }
    });

    // Add edge type relationships
    edgeTypes.forEach(et => {
      et.allowedSourceTypes.forEach(sourceType => {
        et.allowedTargetTypes.forEach(targetType => {
          edges.push({
            id: `allows-${et.id}-${sourceType}-${targetType}`,
            source: sourceType,
            target: targetType,
            label: et.displayName,
            type: 'allows',
          });
        });
      });
    });

    return { nodes, edges };
  }, [nodeTypes, edgeTypes]);

  const drawDiagram = () => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Clear canvas
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Set up transformation
    ctx.save();
    ctx.translate(pan.x, pan.y);
    ctx.scale(zoom, zoom);

    // Simple force-directed layout
    const { nodes, edges } = visualizationData;
    const centerX = canvas.width / 2;
    const centerY = canvas.height / 2;
    const radius = Math.min(centerX, centerY) * 0.6;

    // Position nodes in a circle
    nodes.forEach((node, index) => {
      const angle = (index / nodes.length) * 2 * Math.PI;
      node.x = centerX + Math.cos(angle) * radius;
      node.y = centerY + Math.sin(angle) * radius;
    });

    // Draw edges
    edges.forEach(edge => {
      const sourceNode = nodes.find(n => n.id === edge.source);
      const targetNode = nodes.find(n => n.id === edge.target);
      
      if (sourceNode && targetNode && sourceNode.x && sourceNode.y && targetNode.x && targetNode.y) {
        ctx.beginPath();
        ctx.moveTo(sourceNode.x, sourceNode.y);
        ctx.lineTo(targetNode.x, targetNode.y);
        
        if (edge.type === 'inherits') {
          ctx.strokeStyle = '#ff9800';
          ctx.setLineDash([5, 5]);
        } else {
          ctx.strokeStyle = '#666';
          ctx.setLineDash([]);
        }
        
        ctx.lineWidth = 2;
        ctx.stroke();

        // Draw arrow
        const angle = Math.atan2(targetNode.y - sourceNode.y, targetNode.x - sourceNode.x);
        const arrowLength = 10;
        ctx.beginPath();
        ctx.moveTo(targetNode.x, targetNode.y);
        ctx.lineTo(
          targetNode.x - arrowLength * Math.cos(angle - Math.PI / 6),
          targetNode.y - arrowLength * Math.sin(angle - Math.PI / 6)
        );
        ctx.moveTo(targetNode.x, targetNode.y);
        ctx.lineTo(
          targetNode.x - arrowLength * Math.cos(angle + Math.PI / 6),
          targetNode.y - arrowLength * Math.sin(angle + Math.PI / 6)
        );
        ctx.stroke();
      }
    });

    // Draw nodes
    nodes.forEach(node => {
      if (node.x && node.y) {
        // Draw node circle
        ctx.beginPath();
        ctx.arc(node.x, node.y, 30, 0, 2 * Math.PI);
        ctx.fillStyle = node.color;
        ctx.fill();
        
        if (selectedItem === node.id) {
          ctx.strokeStyle = '#000';
          ctx.lineWidth = 3;
          ctx.stroke();
        }

        // Draw node label
        ctx.fillStyle = '#fff';
        ctx.font = '12px Arial';
        ctx.textAlign = 'center';
        ctx.fillText(node.label, node.x, node.y + 4);

        // Draw property count if enabled
        if (showProperties && node.properties > 0) {
          ctx.fillStyle = '#333';
          ctx.font = '10px Arial';
          ctx.fillText(`${node.properties} props`, node.x, node.y + 45);
        }
      }
    });

    ctx.restore();
  };

  useEffect(() => {
    drawDiagram();
  }, [visualizationData, zoom, pan, selectedItem, showProperties, showInheritance]);

  const handleCanvasClick = (event: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const x = (event.clientX - rect.left - pan.x) / zoom;
    const y = (event.clientY - rect.top - pan.y) / zoom;

    // Find clicked node
    const clickedNode = visualizationData.nodes.find(node => {
      if (!node.x || !node.y) return false;
      const distance = Math.sqrt((x - node.x) ** 2 + (y - node.y) ** 2);
      return distance <= 30;
    });

    if (clickedNode) {
      setSelectedItem(clickedNode.id);
      if (clickedNode.type === 'node-type') {
        onNodeTypeSelect?.(clickedNode.id);
      } else {
        onEdgeTypeSelect?.(clickedNode.id);
      }
    } else {
      setSelectedItem(null);
    }
  };

  const handleZoomIn = () => setZoom(prev => Math.min(prev * 1.2, 3));
  const handleZoomOut = () => setZoom(prev => Math.max(prev / 1.2, 0.3));
  const handleCenter = () => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  };

  const renderRelationshipMatrix = () => {
    const matrix: Record<string, Record<string, string[]>> = {};
    
    edgeTypes.forEach(et => {
      et.allowedSourceTypes.forEach(source => {
        if (!matrix[source]) matrix[source] = {};
        et.allowedTargetTypes.forEach(target => {
          if (!matrix[source][target]) matrix[source][target] = [];
          matrix[source][target].push(et.displayName);
        });
      });
    });

    return (
      <Box>
        <Typography variant="h6" gutterBottom>
          Relationship Matrix
        </Typography>
        <Paper sx={{ overflow: 'auto', maxHeight: 400 }}>
          <table style={{ width: '100%', borderCollapse: 'collapse' }}>
            <thead>
              <tr>
                <th style={{ padding: 8, border: '1px solid #ddd', background: '#f5f5f5' }}>
                  Source → Target
                </th>
                {nodeTypes.map(nt => (
                  <th key={nt.id} style={{ padding: 8, border: '1px solid #ddd', background: '#f5f5f5' }}>
                    {nt.displayName}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {nodeTypes.map(sourceType => (
                <tr key={sourceType.id}>
                  <td style={{ padding: 8, border: '1px solid #ddd', fontWeight: 'bold' }}>
                    {sourceType.displayName}
                  </td>
                  {nodeTypes.map(targetType => (
                    <td key={targetType.id} style={{ padding: 8, border: '1px solid #ddd' }}>
                      {matrix[sourceType.name]?.[targetType.name]?.map(edgeName => (
                        <Chip key={edgeName} label={edgeName} size="small" sx={{ m: 0.25 }} />
                      ))}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </Paper>
      </Box>
    );
  };

  const renderSchemaList = () => (
    <Grid container spacing={2}>
      <Grid item xs={12} md={6}>
        <Card>
          <CardContent>
            <Typography variant="h6" gutterBottom sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
              <NodeIcon />
              Node Types ({nodeTypes.length})
            </Typography>
            <List>
              {nodeTypes.map(nt => (
                <ListItem
                  key={nt.id}
                  button
                  onClick={() => {
                    setSelectedItem(nt.id);
                    onNodeTypeSelect?.(nt.id);
                  }}
                  selected={selectedItem === nt.id}
                >
                  <ListItemIcon>
                    <Box
                      sx={{
                        width: 20,
                        height: 20,
                        borderRadius: '50%',
                        backgroundColor: nt.color || '#1976d2',
                      }}
                    />
                  </ListItemIcon>
                  <ListItemText
                    primary={nt.displayName}
                    secondary={`${nt.properties.length} properties`}
                  />
                </ListItem>
              ))}
            </List>
          </CardContent>
        </Card>
      </Grid>
      <Grid item xs={12} md={6}>
        <Card>
          <CardContent>
            <Typography variant="h6" gutterBottom sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
              <EdgeIcon />
              Edge Types ({edgeTypes.length})
            </Typography>
            <List>
              {edgeTypes.map(et => (
                <ListItem
                  key={et.id}
                  button
                  onClick={() => {
                    setSelectedItem(et.id);
                    onEdgeTypeSelect?.(et.id);
                  }}
                  selected={selectedItem === et.id}
                >
                  <ListItemIcon>
                    <EdgeIcon />
                  </ListItemIcon>
                  <ListItemText
                    primary={et.displayName}
                    secondary={`${et.directionality} • ${et.cardinality}`}
                  />
                </ListItem>
              ))}
            </List>
          </CardContent>
        </Card>
      </Grid>
    </Grid>
  );

  return (
    <Box>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
        <Typography variant="h6" sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <SchemaIcon />
          Schema Visualization
        </Typography>
        <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
          <FormControl size="small" sx={{ minWidth: 120 }}>
            <InputLabel>View Mode</InputLabel>
            <Select
              value={viewMode}
              label="View Mode"
              onChange={(e) => setViewMode(e.target.value as any)}
            >
              <MenuItem value="diagram">Diagram</MenuItem>
              <MenuItem value="matrix">Matrix</MenuItem>
              <MenuItem value="list">List</MenuItem>
            </Select>
          </FormControl>
          {viewMode === 'diagram' && (
            <>
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
              <Tooltip title="Center">
                <IconButton onClick={handleCenter}>
                  <CenterIcon />
                </IconButton>
              </Tooltip>
            </>
          )}
        </Box>
      </Box>

      {viewMode === 'diagram' && (
        <Box>
          <Box sx={{ display: 'flex', gap: 2, mb: 2 }}>
            <FormControlLabel
              control={
                <Switch
                  checked={showProperties}
                  onChange={(e) => setShowProperties(e.target.checked)}
                />
              }
              label="Show Properties"
            />
            <FormControlLabel
              control={
                <Switch
                  checked={showInheritance}
                  onChange={(e) => setShowInheritance(e.target.checked)}
                />
              }
              label="Show Inheritance"
            />
          </Box>
          <Paper sx={{ p: 2, textAlign: 'center' }}>
            <canvas
              ref={canvasRef}
              width={800}
              height={600}
              onClick={handleCanvasClick}
              style={{
                border: '1px solid #ddd',
                cursor: 'pointer',
                maxWidth: '100%',
              }}
            />
          </Paper>
        </Box>
      )}

      {viewMode === 'matrix' && renderRelationshipMatrix()}
      {viewMode === 'list' && renderSchemaList()}
    </Box>
  );
};

export default SchemaVisualizer;