import React, { useState, useCallback, useEffect, useMemo } from 'react';
import {
  Box,
  Paper,
  Typography,
  Tabs,
  Tab,
  Divider,
  Alert,
  Collapse,
  IconButton,
  Tooltip,
  useTheme,
} from '@mui/material';
import {
  Search as SearchIcon,
  FilterList as FilterIcon,
  History as HistoryIcon,
  Bookmark as BookmarkIcon,
  ExpandMore as ExpandMoreIcon,
  ExpandLess as ExpandLessIcon,
  Psychology as PsychologyIcon,
  Hub as HubIcon,
  AutoAwesome as AutoAwesomeIcon,
} from '@mui/icons-material';

import type {
  SemanticSearchQuery,
  SemanticSearchResult,
  SemanticSearchMode,
  SemanticSearchFilters,
  SearchHistoryEntry,
  SavedSemanticSearch,
  SemanticSearchProps,
} from '../../types/semantic';
import type { NodeType, EdgeType } from '../../types/graph';

import SearchQueryInput from './SearchQueryInput';
import SearchFilters from './SearchFilters';
import SearchResults from './SearchResults';
import SimilarityExplorer from './SimilarityExplorer';
import SearchHistory from './SearchHistory';

interface TabPanelProps {
  children?: React.ReactNode;
  index: number;
  value: number;
}

const TabPanel: React.FC<TabPanelProps> = ({ children, value, index, ...other }) => (
  <div
    role="tabpanel"
    hidden={value !== index}
    id={`semantic-search-tabpanel-${index}`}
    aria-labelledby={`semantic-search-tab-${index}`}
    {...other}
  >
    {value === index && <Box sx={{ pt: 2 }}>{children}</Box>}
  </div>
);

const SemanticSearchInterface: React.FC<SemanticSearchProps> = ({
  onSearch,
  onResultSelect,
  onSaveSearch,
  initialQuery = '',
  initialMode = 'natural_language',
  className,
}) => {
  const theme = useTheme();
  
  // State management
  const [query, setQuery] = useState(initialQuery);
  const [mode, setMode] = useState<SemanticSearchMode>(initialMode);
  const [results, setResults] = useState<SemanticSearchResult | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState(0);
  
  // UI state
  const [filtersExpanded, setFiltersExpanded] = useState(false);
  const [historyExpanded, setHistoryExpanded] = useState(false);
  
  // Mock data - in real implementation, these would come from API
  const [searchHistory, setSearchHistory] = useState<SearchHistoryEntry[]>([]);
  const [savedSearches, setSavedSearches] = useState<SavedSemanticSearch[]>([]);
  const [suggestions, setSuggestions] = useState<string[]>([]);
  
  // Default filters
  const [filters, setFilters] = useState<SemanticSearchFilters>({
    node_types: [],
    edge_types: [],
    date_range: {},
    similarity_threshold: 0.7,
    max_results: 50,
    include_explanations: true,
    cluster_results: false,
  });

  // Available types - in real implementation, these would come from schema
  const availableNodeTypes: NodeType[] = ['File', 'Directory', 'Symlink', 'Device', 'Custom'];
  const availableEdgeTypes: EdgeType[] = ['Contains', 'References', 'DependsOn', 'SimilarTo', 'Custom'];

  // Search execution
  const executeSearch = useCallback(async () => {
    if (!query.trim()) return;

    setIsLoading(true);
    setError(null);

    try {
      const searchQuery: SemanticSearchQuery = {
        query: query.trim(),
        mode,
        similarity_threshold: filters.similarity_threshold,
        max_results: filters.max_results,
        node_types: filters.node_types.length > 0 ? filters.node_types : undefined,
        edge_types: filters.edge_types.length > 0 ? filters.edge_types : undefined,
        date_range: filters.date_range.start || filters.date_range.end ? {
          start: filters.date_range.start?.toISOString(),
          end: filters.date_range.end?.toISOString(),
        } : undefined,
        include_explanations: filters.include_explanations,
        cluster_results: filters.cluster_results,
      };

      const searchResults = await onSearch(searchQuery);
      setResults(searchResults);

      // Add to search history
      const historyEntry: SearchHistoryEntry = {
        id: `search-${Date.now()}`,
        query: searchQuery,
        results_count: searchResults.total_results,
        execution_time_ms: searchResults.execution_time_ms,
        timestamp: new Date().toISOString(),
      };
      setSearchHistory(prev => [historyEntry, ...prev.slice(0, 19)]); // Keep last 20

    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed');
      setResults(null);
    } finally {
      setIsLoading(false);
    }
  }, [query, mode, filters, onSearch]);

  // Handle result selection
  const handleResultSelect = useCallback((nodeIds: string[], edgeIds: string[]) => {
    onResultSelect(nodeIds, edgeIds);
  }, [onResultSelect]);

  // Handle search history selection
  const handleHistorySelect = useCallback((entry: SearchHistoryEntry) => {
    setQuery(entry.query.query);
    setMode(entry.query.mode);
    if (entry.query.similarity_threshold !== undefined) {
      setFilters(prev => ({
        ...prev,
        similarity_threshold: entry.query.similarity_threshold!,
        max_results: entry.query.max_results || 50,
        node_types: entry.query.node_types || [],
        edge_types: entry.query.edge_types || [],
        include_explanations: entry.query.include_explanations || true,
        cluster_results: entry.query.cluster_results || false,
      }));
    }
  }, []);

  // Handle saved search selection
  const handleSavedSearchSelect = useCallback((search: SavedSemanticSearch) => {
    setQuery(search.query.query);
    setMode(search.query.mode);
    if (search.query.similarity_threshold !== undefined) {
      setFilters(prev => ({
        ...prev,
        similarity_threshold: search.query.similarity_threshold!,
        max_results: search.query.max_results || 50,
        node_types: search.query.node_types || [],
        edge_types: search.query.edge_types || [],
        include_explanations: search.query.include_explanations || true,
        cluster_results: search.query.cluster_results || false,
      }));
    }
  }, []);

  // Handle save search
  const handleSaveSearch = useCallback(async (search: Omit<SavedSemanticSearch, 'id' | 'created_at' | 'updated_at'>) => {
    if (onSaveSearch) {
      await onSaveSearch(search);
    }
    // Add to local state for demo
    const newSearch: SavedSemanticSearch = {
      ...search,
      id: `saved-${Date.now()}`,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
    setSavedSearches(prev => [newSearch, ...prev]);
  }, [onSaveSearch]);

  // Tab configuration
  const tabs = [
    { label: 'Search', icon: <SearchIcon />, value: 0 },
    { label: 'Similarity', icon: <HubIcon />, value: 1 },
    { label: 'History', icon: <HistoryIcon />, value: 2 },
  ];

  // Mode icons
  const getModeIcon = (searchMode: SemanticSearchMode) => {
    switch (searchMode) {
      case 'natural_language': return <PsychologyIcon />;
      case 'vector_similarity': return <HubIcon />;
      case 'hybrid': return <AutoAwesomeIcon />;
      case 'clustering': return <FilterIcon />;
      default: return <SearchIcon />;
    }
  };

  return (
    <Paper
      sx={{
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
      }}
      className={className}
    >
      {/* Header */}
      <Box sx={{ p: 2, borderBottom: 1, borderColor: 'divider' }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
          {getModeIcon(mode)}
          <Typography variant="h6" component="h2">
            Semantic Search
          </Typography>
          <Box sx={{ flexGrow: 1 }} />
          <Tooltip title="Toggle Filters">
            <IconButton
              size="small"
              onClick={() => setFiltersExpanded(!filtersExpanded)}
              color={filtersExpanded ? 'primary' : 'default'}
            >
              <FilterIcon />
            </IconButton>
          </Tooltip>
          <Tooltip title="Toggle History">
            <IconButton
              size="small"
              onClick={() => setHistoryExpanded(!historyExpanded)}
              color={historyExpanded ? 'primary' : 'default'}
            >
              <HistoryIcon />
            </IconButton>
          </Tooltip>
        </Box>

        {/* Search Input */}
        <SearchQueryInput
          value={query}
          onChange={setQuery}
          onSearch={executeSearch}
          mode={mode}
          onModeChange={setMode}
          suggestions={suggestions.map(s => ({ text: s, type: 'query' as const, confidence: 0.8 }))}
          isLoading={isLoading}
          placeholder="Search using natural language, e.g., 'Find files related to machine learning'"
        />

        {/* Filters Panel */}
        <Collapse in={filtersExpanded}>
          <Box sx={{ mt: 2 }}>
            <SearchFilters
              filters={filters}
              onChange={setFilters}
              availableNodeTypes={availableNodeTypes}
              availableEdgeTypes={availableEdgeTypes}
            />
          </Box>
        </Collapse>

        {/* History Panel */}
        <Collapse in={historyExpanded}>
          <Box sx={{ mt: 2 }}>
            <SearchHistory
              history={searchHistory}
              savedSearches={savedSearches}
              onHistorySelect={handleHistorySelect}
              onSavedSearchSelect={handleSavedSearchSelect}
              onSaveSearch={handleSaveSearch}
              onDeleteSavedSearch={(id: string) => setSavedSearches(prev => prev.filter(s => s.id !== id))}
            />
          </Box>
        </Collapse>
      </Box>

      {/* Error Display */}
      {error && (
        <Box sx={{ p: 2 }}>
          <Alert severity="error" onClose={() => setError(null)}>
            {error}
          </Alert>
        </Box>
      )}

      {/* Tabs */}
      <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
        <Tabs
          value={activeTab}
          onChange={(_, newValue) => setActiveTab(newValue)}
          variant="fullWidth"
        >
          {tabs.map((tab) => (
            <Tab
              key={tab.value}
              label={tab.label}
              icon={tab.icon}
              iconPosition="start"
              sx={{ minHeight: 48 }}
            />
          ))}
        </Tabs>
      </Box>

      {/* Tab Content */}
      <Box sx={{ flexGrow: 1, overflow: 'hidden' }}>
        <TabPanel value={activeTab} index={0}>
          <SearchResults
            results={results}
            onNodeSelect={(nodeIds: string[]) => handleResultSelect(nodeIds, [])}
            onEdgeSelect={(edgeIds: string[]) => handleResultSelect([], edgeIds)}
            onExploreCluster={(cluster: any) => {
              // Handle cluster exploration
              handleResultSelect(cluster.nodes, []);
            }}
            showExplanations={filters.include_explanations}
          />
        </TabPanel>

        <TabPanel value={activeTab} index={1}>
          <SimilarityExplorer
            onSimilaritySearch={async (query: any) => {
              // Convert to semantic search and execute
              const semanticQuery: SemanticSearchQuery = {
                query: query.reference_text || `Similar to node ${query.reference_node_id}`,
                mode: 'vector_similarity',
                similarity_threshold: query.similarity_threshold,
                max_results: query.max_results,
                node_types: query.node_types,
              };
              
              const result = await onSearch(semanticQuery);
              
              // Convert to similarity result format
              return {
                similar_nodes: result.nodes.map(node => ({
                  node,
                  similarity_score: result.relevance_scores[node.id] || 0,
                  distance: 1 - (result.relevance_scores[node.id] || 0),
                  explanation: result.explanations?.[node.id],
                })),
                reference_embedding: result.query_embedding || [],
                execution_time_ms: result.execution_time_ms,
              };
            }}
            onResultSelect={(nodeIds: string[]) => handleResultSelect(nodeIds, [])}
          />
        </TabPanel>

        <TabPanel value={activeTab} index={2}>
          <SearchHistory
            history={searchHistory}
            savedSearches={savedSearches}
            onHistorySelect={handleHistorySelect}
            onSavedSearchSelect={handleSavedSearchSelect}
            onSaveSearch={handleSaveSearch}
            onDeleteSavedSearch={(id: string) => setSavedSearches(prev => prev.filter(s => s.id !== id))}
          />
        </TabPanel>
      </Box>
    </Paper>
  );
};

export default SemanticSearchInterface;