import React, { useState, useCallback } from 'react';
import {
  Box,
  Typography,
  Grid,
  Card,
  CardContent,
  CardActions,
  Button,
  Chip,
  TextField,
  InputAdornment,
  IconButton,
  Tooltip,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Alert,
} from '@mui/material';
import {
  ViewModule as TemplateIcon,
  Search as SearchIcon,
  PlayArrow as UseIcon,
  Visibility as PreviewIcon,
  Info as InfoIcon,
  FilterList as FilterIcon,
} from '@mui/icons-material';

import type {
  NodeResponse,
  EdgeResponse,
  TraversalAlgorithm,
  NodeType,
  EdgeType,
} from '../../types/graph';
import type { QueryBuilderQuery } from './QueryBuilder';

export interface QueryTemplatesProps {
  nodes: NodeResponse[];
  edges: EdgeResponse[];
  onTemplateSelect: (query: QueryBuilderQuery) => void;
  disabled?: boolean;
}

interface QueryTemplate {
  id: string;
  name: string;
  description: string;
  category: 'exploration' | 'pathfinding' | 'analysis' | 'search';
  algorithm: TraversalAlgorithm;
  defaultMaxDepth: number;
  defaultMaxResults: number;
  nodeTypeFilter?: NodeType;
  edgeTypeFilter?: EdgeType;
  requiresEndNode: boolean;
  useCase: string;
  example: string;
  complexity: 'low' | 'medium' | 'high';
}

const QUERY_TEMPLATES: QueryTemplate[] = [
  {
    id: 'explore-neighbors',
    name: 'Explore Neighbors',
    description: 'Find all nodes directly connected to a starting node',
    category: 'exploration',
    algorithm: 'BreadthFirstSearch',
    defaultMaxDepth: 1,
    defaultMaxResults: 50,
    requiresEndNode: false,
    useCase: 'Discover immediate connections and relationships',
    example: 'Find all files in a directory or all nodes referencing a specific file',
    complexity: 'low',
  },
  {
    id: 'local-neighborhood',
    name: 'Local Neighborhood',
    description: 'Explore nodes within 2-3 hops from a starting point',
    category: 'exploration',
    algorithm: 'BreadthFirstSearch',
    defaultMaxDepth: 3,
    defaultMaxResults: 100,
    requiresEndNode: false,
    useCase: 'Understand local graph structure and nearby relationships',
    example: 'Find all files and directories within 3 levels of a root directory',
    complexity: 'low',
  },
  {
    id: 'shortest-path',
    name: 'Shortest Path',
    description: 'Find the shortest path between two specific nodes',
    category: 'pathfinding',
    algorithm: 'Dijkstra',
    defaultMaxDepth: 10,
    defaultMaxResults: 1,
    requiresEndNode: true,
    useCase: 'Find optimal routes or dependency chains between nodes',
    example: 'Find the shortest dependency path between two files',
    complexity: 'medium',
  },
  {
    id: 'deep-exploration',
    name: 'Deep Exploration',
    description: 'Comprehensive exploration of graph structure from a starting point',
    category: 'exploration',
    algorithm: 'DepthFirstSearch',
    defaultMaxDepth: 5,
    defaultMaxResults: 200,
    requiresEndNode: false,
    useCase: 'Thorough analysis of graph branches and deep relationships',
    example: 'Explore all nested subdirectories and their contents',
    complexity: 'high',
  },
  {
    id: 'file-dependencies',
    name: 'File Dependencies',
    description: 'Find all files that depend on or reference a specific file',
    category: 'analysis',
    algorithm: 'BreadthFirstSearch',
    defaultMaxDepth: 4,
    defaultMaxResults: 150,
    nodeTypeFilter: 'File',
    edgeTypeFilter: 'DependsOn',
    requiresEndNode: false,
    useCase: 'Analyze file dependencies and impact analysis',
    example: 'Find all files that would be affected by changing a library file',
    complexity: 'medium',
  },
  {
    id: 'directory-structure',
    name: 'Directory Structure',
    description: 'Explore directory hierarchy and containment relationships',
    category: 'exploration',
    algorithm: 'BreadthFirstSearch',
    defaultMaxDepth: 6,
    defaultMaxResults: 300,
    nodeTypeFilter: 'Directory',
    edgeTypeFilter: 'Contains',
    requiresEndNode: false,
    useCase: 'Understand filesystem organization and directory structure',
    example: 'Map out the complete directory tree structure',
    complexity: 'medium',
  },
  {
    id: 'similar-files',
    name: 'Similar Files',
    description: 'Find files similar to a starting file based on similarity edges',
    category: 'search',
    algorithm: 'BreadthFirstSearch',
    defaultMaxDepth: 2,
    defaultMaxResults: 50,
    nodeTypeFilter: 'File',
    edgeTypeFilter: 'SimilarTo',
    requiresEndNode: false,
    useCase: 'Content-based file discovery and similarity analysis',
    example: 'Find files with similar content or structure to a reference file',
    complexity: 'low',
  },
  {
    id: 'reference-network',
    name: 'Reference Network',
    description: 'Explore reference relationships between nodes',
    category: 'analysis',
    algorithm: 'DepthFirstSearch',
    defaultMaxDepth: 4,
    defaultMaxResults: 100,
    edgeTypeFilter: 'References',
    requiresEndNode: false,
    useCase: 'Analyze cross-references and citation networks',
    example: 'Find all files that reference each other in a project',
    complexity: 'medium',
  },
  {
    id: 'connected-component',
    name: 'Connected Component',
    description: 'Find all nodes connected to a starting node (full component)',
    category: 'analysis',
    algorithm: 'BreadthFirstSearch',
    defaultMaxDepth: 10,
    defaultMaxResults: 500,
    requiresEndNode: false,
    useCase: 'Identify isolated groups and connected components',
    example: 'Find all files in a disconnected project module',
    complexity: 'high',
  },
];

const CATEGORY_COLORS = {
  exploration: 'primary',
  pathfinding: 'secondary',
  analysis: 'success',
  search: 'info',
} as const;

const COMPLEXITY_COLORS = {
  low: 'success',
  medium: 'warning',
  high: 'error',
} as const;

export const QueryTemplates: React.FC<QueryTemplatesProps> = ({
  nodes,
  edges,
  onTemplateSelect,
  disabled = false,
}) => {
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [previewTemplate, setPreviewTemplate] = useState<QueryTemplate | null>(null);

  const categories = ['all', 'exploration', 'pathfinding', 'analysis', 'search'];

  const filteredTemplates = QUERY_TEMPLATES.filter(template => {
    const matchesSearch = template.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
                         template.description.toLowerCase().includes(searchTerm.toLowerCase()) ||
                         template.useCase.toLowerCase().includes(searchTerm.toLowerCase());
    
    const matchesCategory = selectedCategory === 'all' || template.category === selectedCategory;
    
    return matchesSearch && matchesCategory;
  });

  const handleUseTemplate = useCallback((template: QueryTemplate) => {
    const query: QueryBuilderQuery = {
      id: '',
      name: template.name,
      description: template.description,
      algorithm: template.algorithm,
      maxDepth: template.defaultMaxDepth,
      maxResults: template.defaultMaxResults,
      nodeFilters: {
        ...(template.nodeTypeFilter && { node_type: template.nodeTypeFilter }),
      },
      edgeFilters: {
        ...(template.edgeTypeFilter && { edge_type: template.edgeTypeFilter }),
      },
      timeoutMs: 30000,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };

    onTemplateSelect(query);
  }, [onTemplateSelect]);

  const handlePreviewTemplate = useCallback((template: QueryTemplate) => {
    setPreviewTemplate(template);
  }, []);

  return (
    <Box>
      <Box sx={{ mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
          <TemplateIcon color="primary" sx={{ mr: 1 }} />
          <Typography variant="h6" component="div">
            Query Templates
          </Typography>
        </Box>
        <Typography variant="body2" color="text.secondary" gutterBottom>
          Pre-built query templates for common graph traversal patterns
        </Typography>

        {/* Search and Filter Controls */}
        <Box sx={{ display: 'flex', gap: 2, mb: 2 }}>
          <TextField
            placeholder="Search templates..."
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
          <Box sx={{ display: 'flex', gap: 1 }}>
            {categories.map((category) => (
              <Chip
                key={category}
                label={category.charAt(0).toUpperCase() + category.slice(1)}
                onClick={() => setSelectedCategory(category)}
                color={selectedCategory === category ? 'primary' : 'default'}
                variant={selectedCategory === category ? 'filled' : 'outlined'}
                size="small"
              />
            ))}
          </Box>
        </Box>
      </Box>

      {/* Templates Grid */}
      <Grid container spacing={2}>
        {filteredTemplates.map((template) => (
          <Grid item xs={12} md={6} lg={4} key={template.id}>
            <Card sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
              <CardContent sx={{ flexGrow: 1 }}>
                <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 1 }}>
                  <Typography variant="h6" component="div" noWrap>
                    {template.name}
                  </Typography>
                  <Tooltip title="View Details">
                    <IconButton
                      size="small"
                      onClick={() => handlePreviewTemplate(template)}
                    >
                      <InfoIcon />
                    </IconButton>
                  </Tooltip>
                </Box>

                <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                  {template.description}
                </Typography>

                <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5, mb: 2 }}>
                  <Chip
                    label={template.category}
                    size="small"
                    color={CATEGORY_COLORS[template.category]}
                    variant="outlined"
                  />
                  <Chip
                    label={template.algorithm}
                    size="small"
                    variant="outlined"
                  />
                  <Chip
                    label={`${template.complexity} complexity`}
                    size="small"
                    color={COMPLEXITY_COLORS[template.complexity]}
                    variant="outlined"
                  />
                </Box>

                <Typography variant="caption" color="text.secondary" display="block" sx={{ mb: 1 }}>
                  <strong>Use Case:</strong> {template.useCase}
                </Typography>

                <Box sx={{ display: 'flex', gap: 1, mb: 1 }}>
                  <Chip label={`Depth: ${template.defaultMaxDepth}`} size="small" />
                  <Chip label={`Results: ${template.defaultMaxResults}`} size="small" />
                </Box>

                {(template.nodeTypeFilter || template.edgeTypeFilter) && (
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
                    <FilterIcon fontSize="small" color="action" />
                    <Typography variant="caption" color="text.secondary">
                      {template.nodeTypeFilter && `Node: ${template.nodeTypeFilter}`}
                      {template.nodeTypeFilter && template.edgeTypeFilter && ', '}
                      {template.edgeTypeFilter && `Edge: ${template.edgeTypeFilter}`}
                    </Typography>
                  </Box>
                )}
              </CardContent>

              <CardActions>
                <Button
                  size="small"
                  startIcon={<PreviewIcon />}
                  onClick={() => handlePreviewTemplate(template)}
                >
                  Preview
                </Button>
                <Button
                  size="small"
                  variant="contained"
                  startIcon={<UseIcon />}
                  onClick={() => handleUseTemplate(template)}
                  disabled={disabled}
                >
                  Use Template
                </Button>
              </CardActions>
            </Card>
          </Grid>
        ))}
      </Grid>

      {filteredTemplates.length === 0 && (
        <Box sx={{ textAlign: 'center', py: 4 }}>
          <Typography variant="h6" color="text.secondary" gutterBottom>
            No templates found
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Try adjusting your search terms or category filter
          </Typography>
        </Box>
      )}

      {/* Template Preview Dialog */}
      <Dialog
        open={!!previewTemplate}
        onClose={() => setPreviewTemplate(null)}
        maxWidth="md"
        fullWidth
      >
        {previewTemplate && (
          <>
            <DialogTitle>
              <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <Typography variant="h6">{previewTemplate.name}</Typography>
                <Box sx={{ display: 'flex', gap: 0.5 }}>
                  <Chip
                    label={previewTemplate.category}
                    size="small"
                    color={CATEGORY_COLORS[previewTemplate.category]}
                  />
                  <Chip
                    label={`${previewTemplate.complexity} complexity`}
                    size="small"
                    color={COMPLEXITY_COLORS[previewTemplate.complexity]}
                  />
                </Box>
              </Box>
            </DialogTitle>
            <DialogContent>
              <Typography variant="body1" gutterBottom>
                {previewTemplate.description}
              </Typography>

              <Alert severity="info" sx={{ my: 2 }}>
                <Typography variant="body2">
                  <strong>Use Case:</strong> {previewTemplate.useCase}
                </Typography>
              </Alert>

              <Typography variant="subtitle2" gutterBottom sx={{ mt: 2 }}>
                Configuration Details:
              </Typography>
              <Grid container spacing={2}>
                <Grid item xs={6}>
                  <Typography variant="body2">
                    <strong>Algorithm:</strong> {previewTemplate.algorithm}
                  </Typography>
                </Grid>
                <Grid item xs={6}>
                  <Typography variant="body2">
                    <strong>Max Depth:</strong> {previewTemplate.defaultMaxDepth}
                  </Typography>
                </Grid>
                <Grid item xs={6}>
                  <Typography variant="body2">
                    <strong>Max Results:</strong> {previewTemplate.defaultMaxResults}
                  </Typography>
                </Grid>
                <Grid item xs={6}>
                  <Typography variant="body2">
                    <strong>Requires End Node:</strong> {previewTemplate.requiresEndNode ? 'Yes' : 'No'}
                  </Typography>
                </Grid>
                {previewTemplate.nodeTypeFilter && (
                  <Grid item xs={6}>
                    <Typography variant="body2">
                      <strong>Node Filter:</strong> {previewTemplate.nodeTypeFilter}
                    </Typography>
                  </Grid>
                )}
                {previewTemplate.edgeTypeFilter && (
                  <Grid item xs={6}>
                    <Typography variant="body2">
                      <strong>Edge Filter:</strong> {previewTemplate.edgeTypeFilter}
                    </Typography>
                  </Grid>
                )}
              </Grid>

              <Typography variant="subtitle2" gutterBottom sx={{ mt: 2 }}>
                Example:
              </Typography>
              <Typography variant="body2" color="text.secondary">
                {previewTemplate.example}
              </Typography>
            </DialogContent>
            <DialogActions>
              <Button onClick={() => setPreviewTemplate(null)}>
                Close
              </Button>
              <Button
                variant="contained"
                startIcon={<UseIcon />}
                onClick={() => {
                  handleUseTemplate(previewTemplate);
                  setPreviewTemplate(null);
                }}
                disabled={disabled}
              >
                Use This Template
              </Button>
            </DialogActions>
          </>
        )}
      </Dialog>
    </Box>
  );
};

export default QueryTemplates;