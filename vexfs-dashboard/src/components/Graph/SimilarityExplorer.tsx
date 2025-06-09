import React, { useState, useCallback, useEffect } from 'react';
import {
  Box,
  Paper,
  Typography,
  TextField,
  Button,
  Slider,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Chip,
  List,
  ListItem,
  ListItemText,
  ListItemIcon,
  ListItemButton,
  Card,
  CardContent,
  LinearProgress,
  Alert,
  Divider,
  IconButton,
  Tooltip,
  useTheme,
} from '@mui/material';
import {
  Hub as HubIcon,
  Search as SearchIcon,
  Clear as ClearIcon,
  TrendingUp as TrendingUpIcon,
  Folder as FolderIcon,
  InsertDriveFile as FileIcon,
  Link as LinkIcon,
  DeviceHub as DeviceIcon,
  Extension as ExtensionIcon,
  Refresh as RefreshIcon,
} from '@mui/icons-material';

import type {
  VectorSimilarityQuery,
  VectorSimilarityResult,
  SimilarityExplorerProps,
} from '../../types/semantic';
import type { NodeType } from '../../types/graph';

const SimilarityExplorer: React.FC<SimilarityExplorerProps> = ({
  referenceNodeId,
  onReferenceChange,
  onSimilaritySearch,
  onResultSelect,
  className,
}) => {
  const theme = useTheme();
  
  // State
  const [referenceText, setReferenceText] = useState('');
  const [similarityThreshold, setSimilarityThreshold] = useState(0.7);
  const [maxResults, setMaxResults] = useState(20);
  const [selectedNodeTypes, setSelectedNodeTypes] = useState<NodeType[]>([]);
  const [results, setResults] = useState<VectorSimilarityResult | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedNodes, setSelectedNodes] = useState<string[]>([]);

  // Available node types
  const availableNodeTypes: NodeType[] = ['File', 'Directory', 'Symlink', 'Device', 'Custom'];

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

  // Get similarity color based on score
  const getSimilarityColor = (score: number) => {
    if (score >= 0.8) return theme.palette.success.main;
    if (score >= 0.6) return theme.palette.warning.main;
    return theme.palette.error.main;
  };

  // Execute similarity search
  const executeSearch = useCallback(async () => {
    if (!referenceNodeId && !referenceText.trim()) {
      setError('Please provide a reference node or text');
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const query: VectorSimilarityQuery = {
        reference_node_id: referenceNodeId,
        reference_text: referenceText.trim() || undefined,
        similarity_threshold: similarityThreshold,
        max_results: maxResults,
        node_types: selectedNodeTypes.length > 0 ? selectedNodeTypes : undefined,
        include_distances: true,
      };

      const searchResults = await onSimilaritySearch(query);
      setResults(searchResults);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Similarity search failed');
      setResults(null);
    } finally {
      setIsLoading(false);
    }
  }, [referenceNodeId, referenceText, similarityThreshold, maxResults, selectedNodeTypes, onSimilaritySearch]);

  // Handle node selection
  const handleNodeClick = useCallback((nodeId: string) => {
    const newSelection = selectedNodes.includes(nodeId)
      ? selectedNodes.filter(id => id !== nodeId)
      : [...selectedNodes, nodeId];
    
    setSelectedNodes(newSelection);
    onResultSelect(newSelection);
  }, [selectedNodes, onResultSelect]);

  // Clear reference
  const clearReference = useCallback(() => {
    onReferenceChange(undefined);
    setReferenceText('');
    setResults(null);
    setSelectedNodes([]);
  }, [onReferenceChange]);

  // Render similarity result item
  const renderSimilarityItem = (item: VectorSimilarityResult['similar_nodes'][0]) => {
    const { node, similarity_score, distance, explanation } = item;
    const isSelected = selectedNodes.includes(node.id);

    return (
      <ListItem key={node.id} disablePadding>
        <ListItemButton
          selected={isSelected}
          onClick={() => handleNodeClick(node.id)}
          sx={{
            borderLeft: 4,
            borderLeftColor: getSimilarityColor(similarity_score),
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
                  label={`${Math.round(similarity_score * 100)}%`}
                  size="small"
                  sx={{
                    backgroundColor: getSimilarityColor(similarity_score),
                    color: 'white',
                  }}
                />
              </Box>
            }
            secondary={
              <Box>
                <Typography variant="caption" color="text.secondary">
                  Distance: {distance.toFixed(4)} • ID: {node.id}
                </Typography>
                {explanation && (
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

  return (
    <Paper sx={{ height: '100%', display: 'flex', flexDirection: 'column' }} className={className}>
      {/* Header */}
      <Box sx={{ p: 2, borderBottom: 1, borderColor: 'divider' }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <HubIcon color="primary" />
          <Typography variant="h6">
            Vector Similarity Explorer
          </Typography>
          <Box sx={{ flexGrow: 1 }} />
          {results && (
            <Typography variant="caption" color="text.secondary">
              {results.execution_time_ms}ms
            </Typography>
          )}
        </Box>

        {/* Reference Selection */}
        <Box sx={{ mb: 2 }}>
          <Typography variant="subtitle2" gutterBottom>
            Reference
          </Typography>
          {referenceNodeId ? (
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
              <Chip
                label={`Node: ${referenceNodeId}`}
                color="primary"
                onDelete={clearReference}
              />
            </Box>
          ) : (
            <TextField
              fullWidth
              size="small"
              label="Reference text"
              value={referenceText}
              onChange={(e) => setReferenceText(e.target.value)}
              placeholder="Enter text to find similar nodes..."
              InputProps={{
                endAdornment: referenceText && (
                  <IconButton size="small" onClick={() => setReferenceText('')}>
                    <ClearIcon />
                  </IconButton>
                ),
              }}
            />
          )}
        </Box>

        {/* Similarity Threshold */}
        <Box sx={{ mb: 2 }}>
          <Typography variant="body2" gutterBottom>
            Similarity Threshold: {similarityThreshold.toFixed(2)}
          </Typography>
          <Slider
            value={similarityThreshold}
            onChange={(_, value) => setSimilarityThreshold(Array.isArray(value) ? value[0] : value)}
            min={0}
            max={1}
            step={0.01}
            marks={[
              { value: 0, label: '0%' },
              { value: 0.5, label: '50%' },
              { value: 1, label: '100%' },
            ]}
            valueLabelDisplay="auto"
            valueLabelFormat={(value) => `${Math.round(value * 100)}%`}
          />
        </Box>

        {/* Filters */}
        <Box sx={{ display: 'flex', gap: 2, mb: 2 }}>
          <TextField
            size="small"
            label="Max Results"
            type="number"
            value={maxResults}
            onChange={(e) => setMaxResults(parseInt(e.target.value, 10) || 20)}
            inputProps={{ min: 1, max: 100 }}
            sx={{ width: 120 }}
          />
          <FormControl size="small" sx={{ minWidth: 150 }}>
            <InputLabel>Node Types</InputLabel>
            <Select
              multiple
              value={selectedNodeTypes}
              onChange={(e) => setSelectedNodeTypes(e.target.value as NodeType[])}
              renderValue={(selected) => (
                <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
                  {selected.map((value) => (
                    <Chip key={value} label={value} size="small" />
                  ))}
                </Box>
              )}
            >
              {availableNodeTypes.map((type) => (
                <MenuItem key={type} value={type}>
                  {type}
                </MenuItem>
              ))}
            </Select>
          </FormControl>
        </Box>

        {/* Search Button */}
        <Button
          fullWidth
          variant="contained"
          startIcon={isLoading ? <RefreshIcon /> : <SearchIcon />}
          onClick={executeSearch}
          disabled={isLoading || (!referenceNodeId && !referenceText.trim())}
        >
          {isLoading ? 'Searching...' : 'Find Similar Nodes'}
        </Button>
      </Box>

      {/* Error Display */}
      {error && (
        <Box sx={{ p: 2 }}>
          <Alert severity="error" onClose={() => setError(null)}>
            {error}
          </Alert>
        </Box>
      )}

      {/* Results */}
      <Box sx={{ flexGrow: 1, overflow: 'auto' }}>
        {!results ? (
          <Box sx={{ p: 3, textAlign: 'center' }}>
            <HubIcon sx={{ fontSize: 48, color: 'text.secondary', mb: 2 }} />
            <Typography variant="h6" color="text.secondary">
              No similarity search performed
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Select a reference node or enter text to find similar nodes
            </Typography>
          </Box>
        ) : results.similar_nodes.length === 0 ? (
          <Box sx={{ p: 3, textAlign: 'center' }}>
            <Alert severity="info">
              No similar nodes found above the threshold
            </Alert>
            <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
              Try lowering the similarity threshold or using different reference content
            </Typography>
          </Box>
        ) : (
          <Box sx={{ p: 2 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
              <TrendingUpIcon color="primary" />
              <Typography variant="subtitle1">
                Similar Nodes ({results.similar_nodes.length})
              </Typography>
              {selectedNodes.length > 0 && (
                <Chip
                  label={`${selectedNodes.length} selected`}
                  size="small"
                  color="primary"
                />
              )}
            </Box>
            
            <List dense>
              {results.similar_nodes.map(renderSimilarityItem)}
            </List>
          </Box>
        )}
      </Box>
    </Paper>
  );
};

export default SimilarityExplorer;