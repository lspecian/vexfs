import React, { useState, useCallback, useMemo } from 'react';
import {
  Box,
  Paper,
  Typography,
  List,
  ListItem,
  ListItemText,
  ListItemIcon,
  ListItemButton,
  Chip,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  LinearProgress,
  Alert,
  Tabs,
  Tab,
  Badge,
  IconButton,
  Tooltip,
  Divider,
  Card,
  CardContent,
  useTheme,
} from '@mui/material';
import {
  ExpandMore as ExpandMoreIcon,
  Folder as FolderIcon,
  InsertDriveFile as FileIcon,
  Link as LinkIcon,
  DeviceHub as DeviceIcon,
  Extension as ExtensionIcon,
  Psychology as PsychologyIcon,
  TrendingUp as TrendingUpIcon,
  Group as GroupIcon,
  Visibility as VisibilityIcon,
  Info as InfoIcon,
} from '@mui/icons-material';

import type {
  SemanticSearchResult,
  SemanticCluster,
  SearchResultsProps,
} from '../../types/semantic';
import type { NodeResponse, EdgeResponse, NodeType } from '../../types/graph';

interface TabPanelProps {
  children?: React.ReactNode;
  index: number;
  value: number;
}

const TabPanel: React.FC<TabPanelProps> = ({ children, value, index, ...other }) => (
  <div
    role="tabpanel"
    hidden={value !== index}
    id={`search-results-tabpanel-${index}`}
    aria-labelledby={`search-results-tab-${index}`}
    {...other}
  >
    {value === index && <Box sx={{ p: 2 }}>{children}</Box>}
  </div>
);

const SearchResults: React.FC<SearchResultsProps> = ({
  results,
  onNodeSelect,
  onEdgeSelect,
  onExploreCluster,
  showExplanations = true,
  className,
}) => {
  const theme = useTheme();
  const [activeTab, setActiveTab] = useState(0);
  const [selectedNodes, setSelectedNodes] = useState<string[]>([]);
  const [selectedEdges, setSelectedEdges] = useState<string[]>([]);

  // Get node icon based on type
  const getNodeIcon = (nodeType: NodeType) => {
    switch (nodeType) {
      case 'File': return <FileIcon />;
      case 'Directory': return <FolderIcon />;
      case 'Symlink': return <LinkIcon />;
      case 'Device': return <DeviceIcon />;
      case 'Custom': return <ExtensionIcon />;
      default: return <FileIcon />;
    }
  };

  // Get relevance color based on score
  const getRelevanceColor = (score: number) => {
    if (score >= 0.8) return theme.palette.success.main;
    if (score >= 0.6) return theme.palette.warning.main;
    return theme.palette.error.main;
  };

  // Handle node selection
  const handleNodeClick = useCallback((nodeId: string) => {
    const newSelection = selectedNodes.includes(nodeId)
      ? selectedNodes.filter(id => id !== nodeId)
      : [...selectedNodes, nodeId];
    
    setSelectedNodes(newSelection);
    onNodeSelect(newSelection);
  }, [selectedNodes, onNodeSelect]);

  // Handle edge selection
  const handleEdgeClick = useCallback((edgeId: string) => {
    const newSelection = selectedEdges.includes(edgeId)
      ? selectedEdges.filter(id => id !== edgeId)
      : [...selectedEdges, edgeId];
    
    setSelectedEdges(newSelection);
    onEdgeSelect(newSelection);
  }, [selectedEdges, onEdgeSelect]);

  // Render node result item
  const renderNodeItem = (node: NodeResponse) => {
    const relevanceScore = results?.relevance_scores[node.id] || 0;
    const explanation = results?.explanations?.[node.id];
    const isSelected = selectedNodes.includes(node.id);

    return (
      <ListItem key={node.id} disablePadding>
        <ListItemButton
          selected={isSelected}
          onClick={() => handleNodeClick(node.id)}
          sx={{
            borderLeft: 4,
            borderLeftColor: getRelevanceColor(relevanceScore),
            mb: 1,
            borderRadius: 1,
          }}
        >
          <ListItemIcon>
            {getNodeIcon(node.node_type)}
          </ListItemIcon>
          <ListItemText
            primary={
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <Typography variant="body2" fontWeight="medium">
                  {node.properties?.name || node.properties?.path || `Node ${node.id}`}
                </Typography>
                <Chip
                  label={node.node_type}
                  size="small"
                  variant="outlined"
                />
                <Chip
                  label={`${Math.round(relevanceScore * 100)}%`}
                  size="small"
                  sx={{
                    backgroundColor: getRelevanceColor(relevanceScore),
                    color: 'white',
                  }}
                />
              </Box>
            }
            secondary={
              <Box>
                <Typography variant="caption" color="text.secondary">
                  ID: {node.id} • Created: {new Date(node.created_at).toLocaleDateString()}
                </Typography>
                {showExplanations && explanation && (
                  <Typography variant="body2" sx={{ mt: 0.5, fontStyle: 'italic' }}>
                    {explanation}
                  </Typography>
                )}
                {Object.keys(node.properties).length > 0 && (
                  <Box sx={{ mt: 0.5, display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
                    {Object.entries(node.properties).slice(0, 3).map(([key, value]) => (
                      <Chip
                        key={key}
                        label={`${key}: ${String(value).slice(0, 20)}${String(value).length > 20 ? '...' : ''}`}
                        size="small"
                        variant="outlined"
                        sx={{ fontSize: '0.7rem', height: 20 }}
                      />
                    ))}
                  </Box>
                )}
              </Box>
            }
          />
        </ListItemButton>
      </ListItem>
    );
  };

  // Render edge result item
  const renderEdgeItem = (edge: EdgeResponse) => {
    const relevanceScore = results?.relevance_scores[edge.id] || 0;
    const explanation = results?.explanations?.[edge.id];
    const isSelected = selectedEdges.includes(edge.id);

    return (
      <ListItem key={edge.id} disablePadding>
        <ListItemButton
          selected={isSelected}
          onClick={() => handleEdgeClick(edge.id)}
          sx={{
            borderLeft: 4,
            borderLeftColor: getRelevanceColor(relevanceScore),
            mb: 1,
            borderRadius: 1,
          }}
        >
          <ListItemIcon>
            <LinkIcon />
          </ListItemIcon>
          <ListItemText
            primary={
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <Typography variant="body2" fontWeight="medium">
                  {edge.edge_type}
                </Typography>
                <Typography variant="caption" color="text.secondary">
                  {edge.source_id} → {edge.target_id}
                </Typography>
                <Chip
                  label={`${Math.round(relevanceScore * 100)}%`}
                  size="small"
                  sx={{
                    backgroundColor: getRelevanceColor(relevanceScore),
                    color: 'white',
                  }}
                />
              </Box>
            }
            secondary={
              <Box>
                <Typography variant="caption" color="text.secondary">
                  Weight: {edge.weight} • ID: {edge.id}
                </Typography>
                {showExplanations && explanation && (
                  <Typography variant="body2" sx={{ mt: 0.5, fontStyle: 'italic' }}>
                    {explanation}
                  </Typography>
                )}
              </Box>
            }
          />
        </ListItemButton>
      </ListItem>
    );
  };

  // Render cluster item
  const renderClusterItem = (cluster: SemanticCluster) => {
    return (
      <Card key={cluster.id} sx={{ mb: 2 }}>
        <CardContent>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
            <GroupIcon color="primary" />
            <Typography variant="h6">{cluster.label}</Typography>
            <Chip label={`${cluster.size} nodes`} size="small" />
            <Chip
              label={`${Math.round(cluster.coherence_score * 100)}% coherence`}
              size="small"
              color="secondary"
            />
            <Box sx={{ flexGrow: 1 }} />
            <Tooltip title="Explore cluster">
              <IconButton
                size="small"
                onClick={() => onExploreCluster(cluster)}
                color="primary"
              >
                <VisibilityIcon />
              </IconButton>
            </Tooltip>
          </Box>
          
          {cluster.representative_terms.length > 0 && (
            <Box sx={{ mb: 1 }}>
              <Typography variant="caption" color="text.secondary">
                Key terms:
              </Typography>
              <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5, mt: 0.5 }}>
                {cluster.representative_terms.map((term, index) => (
                  <Chip
                    key={index}
                    label={term}
                    size="small"
                    variant="outlined"
                    sx={{ fontSize: '0.75rem' }}
                  />
                ))}
              </Box>
            </Box>
          )}
          
          <LinearProgress
            variant="determinate"
            value={cluster.coherence_score * 100}
            sx={{ height: 6, borderRadius: 3 }}
          />
        </CardContent>
      </Card>
    );
  };

  // Tab configuration
  const tabs = useMemo(() => {
    const nodeCount = results?.nodes.length || 0;
    const edgeCount = results?.edges.length || 0;
    const clusterCount = results?.clusters?.length || 0;

    return [
      { label: 'Nodes', count: nodeCount, value: 0 },
      { label: 'Edges', count: edgeCount, value: 1 },
      { label: 'Clusters', count: clusterCount, value: 2 },
    ];
  }, [results]);

  // No results state
  if (!results) {
    return (
      <Paper sx={{ p: 3, textAlign: 'center' }} className={className}>
        <PsychologyIcon sx={{ fontSize: 48, color: 'text.secondary', mb: 2 }} />
        <Typography variant="h6" color="text.secondary">
          No search performed yet
        </Typography>
        <Typography variant="body2" color="text.secondary">
          Enter a search query to find semantically related nodes and edges
        </Typography>
      </Paper>
    );
  }

  // Empty results state
  if (results.total_results === 0) {
    return (
      <Paper sx={{ p: 3, textAlign: 'center' }} className={className}>
        <Alert severity="info" sx={{ mb: 2 }}>
          No results found for your search query
        </Alert>
        <Typography variant="body2" color="text.secondary">
          Try adjusting your search terms or filters
        </Typography>
      </Paper>
    );
  }

  return (
    <Paper sx={{ height: '100%', display: 'flex', flexDirection: 'column' }} className={className}>
      {/* Results Header */}
      <Box sx={{ p: 2, borderBottom: 1, borderColor: 'divider' }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
          <TrendingUpIcon color="primary" />
          <Typography variant="h6">
            Search Results
          </Typography>
          <Chip
            label={`${results.total_results} results`}
            size="small"
            color="primary"
          />
          <Box sx={{ flexGrow: 1 }} />
          <Typography variant="caption" color="text.secondary">
            {results.execution_time_ms}ms
          </Typography>
        </Box>
        
        {selectedNodes.length > 0 || selectedEdges.length > 0 ? (
          <Typography variant="body2" color="primary">
            Selected: {selectedNodes.length} nodes, {selectedEdges.length} edges
          </Typography>
        ) : null}
      </Box>

      {/* Tabs */}
      <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
        <Tabs value={activeTab} onChange={(_, newValue) => setActiveTab(newValue)}>
          {tabs.map((tab) => (
            <Tab
              key={tab.value}
              label={
                <Badge badgeContent={tab.count} color="primary" max={999}>
                  {tab.label}
                </Badge>
              }
            />
          ))}
        </Tabs>
      </Box>

      {/* Tab Content */}
      <Box sx={{ flexGrow: 1, overflow: 'auto' }}>
        <TabPanel value={activeTab} index={0}>
          {results.nodes.length > 0 ? (
            <List dense>
              {results.nodes.map(renderNodeItem)}
            </List>
          ) : (
            <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
              No nodes found
            </Typography>
          )}
        </TabPanel>

        <TabPanel value={activeTab} index={1}>
          {results.edges.length > 0 ? (
            <List dense>
              {results.edges.map(renderEdgeItem)}
            </List>
          ) : (
            <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
              No edges found
            </Typography>
          )}
        </TabPanel>

        <TabPanel value={activeTab} index={2}>
          {results.clusters && results.clusters.length > 0 ? (
            <Box>
              {results.clusters.map(renderClusterItem)}
            </Box>
          ) : (
            <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
              No clusters found
            </Typography>
          )}
        </TabPanel>
      </Box>
    </Paper>
  );
};

export default SearchResults;