// Semantic Search Type Definitions for VexGraph
// Extends the base graph types with AI-powered semantic search capabilities

import type { NodeId, EdgeId, NodeResponse, EdgeResponse, NodeType, EdgeType } from './graph';

export type SemanticSearchMode = 'natural_language' | 'vector_similarity' | 'hybrid' | 'clustering';

export type SimilarityThreshold = number; // 0.0 to 1.0

export interface SemanticSearchQuery {
  query: string;
  mode: SemanticSearchMode;
  similarity_threshold?: SimilarityThreshold;
  max_results?: number;
  node_types?: NodeType[];
  edge_types?: EdgeType[];
  date_range?: {
    start?: string;
    end?: string;
  };
  include_explanations?: boolean;
  cluster_results?: boolean;
}

export interface SemanticSearchResult {
  nodes: NodeResponse[];
  edges: EdgeResponse[];
  relevance_scores: Record<NodeId | EdgeId, number>;
  explanations?: Record<NodeId | EdgeId, string>;
  clusters?: SemanticCluster[];
  execution_time_ms: number;
  total_results: number;
  query_embedding?: number[];
}

export interface VectorSimilarityQuery {
  reference_node_id?: NodeId;
  reference_vector?: number[];
  reference_text?: string;
  similarity_threshold: SimilarityThreshold;
  max_results?: number;
  node_types?: NodeType[];
  include_distances?: boolean;
}

export interface VectorSimilarityResult {
  similar_nodes: Array<{
    node: NodeResponse;
    similarity_score: number;
    distance: number;
    explanation?: string;
  }>;
  reference_embedding: number[];
  execution_time_ms: number;
}

export interface HybridSearchQuery {
  text_query: string;
  vector_query?: {
    reference_node_id?: NodeId;
    reference_vector?: number[];
  };
  text_weight: number; // 0.0 to 1.0
  vector_weight: number; // 0.0 to 1.0
  max_results?: number;
  node_types?: NodeType[];
  edge_types?: EdgeType[];
}

export interface HybridSearchResult {
  results: Array<{
    node?: NodeResponse;
    edge?: EdgeResponse;
    text_score: number;
    vector_score: number;
    combined_score: number;
    explanation: string;
  }>;
  execution_time_ms: number;
  total_results: number;
}

export interface SemanticCluster {
  id: string;
  label: string;
  nodes: NodeId[];
  centroid_embedding: number[];
  coherence_score: number;
  representative_terms: string[];
  size: number;
}

export interface SemanticClusteringQuery {
  node_ids?: NodeId[];
  num_clusters?: number;
  min_cluster_size?: number;
  similarity_threshold?: SimilarityThreshold;
  include_terms?: boolean;
}

export interface SemanticClusteringResult {
  clusters: SemanticCluster[];
  outliers: NodeId[];
  silhouette_score: number;
  execution_time_ms: number;
}

export interface SearchSuggestion {
  text: string;
  type: 'query' | 'filter' | 'concept';
  confidence: number;
  category?: string;
}

export interface SearchExplanation {
  node_id?: NodeId;
  edge_id?: EdgeId;
  match_type: 'semantic' | 'keyword' | 'property' | 'vector';
  confidence: number;
  matched_terms: string[];
  reasoning: string;
  embedding_similarity?: number;
}

export interface SearchHistoryEntry {
  id: string;
  query: SemanticSearchQuery;
  results_count: number;
  execution_time_ms: number;
  timestamp: string;
  user_id?: string;
  saved?: boolean;
}

export interface SavedSemanticSearch {
  id: string;
  name: string;
  description?: string;
  query: SemanticSearchQuery;
  tags: string[];
  created_at: string;
  updated_at: string;
  user_id?: string;
  is_public: boolean;
  usage_count: number;
}

export interface SemanticSearchFilters {
  node_types: NodeType[];
  edge_types: EdgeType[];
  date_range: {
    start?: Date;
    end?: Date;
  };
  similarity_threshold: SimilarityThreshold;
  max_results: number;
  include_explanations: boolean;
  cluster_results: boolean;
}

export interface SemanticSearchState {
  query: string;
  mode: SemanticSearchMode;
  filters: SemanticSearchFilters;
  results: SemanticSearchResult | null;
  isLoading: boolean;
  error: string | null;
  suggestions: SearchSuggestion[];
  history: SearchHistoryEntry[];
  savedSearches: SavedSemanticSearch[];
}

// UI Component Props
export interface SemanticSearchProps {
  onSearch: (query: SemanticSearchQuery) => Promise<SemanticSearchResult>;
  onResultSelect: (nodeIds: NodeId[], edgeIds: EdgeId[]) => void;
  onSaveSearch?: (search: Omit<SavedSemanticSearch, 'id' | 'created_at' | 'updated_at'>) => Promise<void>;
  initialQuery?: string;
  initialMode?: SemanticSearchMode;
  className?: string;
}

export interface SearchQueryInputProps {
  value: string;
  onChange: (value: string) => void;
  onSearch: () => void;
  mode: SemanticSearchMode;
  onModeChange: (mode: SemanticSearchMode) => void;
  suggestions: SearchSuggestion[];
  isLoading: boolean;
  placeholder?: string;
  className?: string;
}

export interface SearchFiltersProps {
  filters: SemanticSearchFilters;
  onChange: (filters: SemanticSearchFilters) => void;
  availableNodeTypes: NodeType[];
  availableEdgeTypes: EdgeType[];
  className?: string;
}

export interface SearchResultsProps {
  results: SemanticSearchResult | null;
  onNodeSelect: (nodeIds: NodeId[]) => void;
  onEdgeSelect: (edgeIds: EdgeId[]) => void;
  onExploreCluster: (cluster: SemanticCluster) => void;
  showExplanations: boolean;
  className?: string;
}

export interface SimilarityExplorerProps {
  referenceNodeId?: NodeId;
  onReferenceChange: (nodeId: NodeId | undefined) => void;
  onSimilaritySearch: (query: VectorSimilarityQuery) => Promise<VectorSimilarityResult>;
  onResultSelect: (nodeIds: NodeId[]) => void;
  className?: string;
}

export interface SearchHistoryProps {
  history: SearchHistoryEntry[];
  savedSearches: SavedSemanticSearch[];
  onHistorySelect: (entry: SearchHistoryEntry) => void;
  onSavedSearchSelect: (search: SavedSemanticSearch) => void;
  onSaveSearch: (search: Omit<SavedSemanticSearch, 'id' | 'created_at' | 'updated_at'>) => void;
  onDeleteSavedSearch: (searchId: string) => void;
  className?: string;
}

// API Extensions
export interface SemanticApiMethods {
  semanticSearch: (query: SemanticSearchQuery) => Promise<SemanticSearchResult>;
  vectorSimilaritySearch: (query: VectorSimilarityQuery) => Promise<VectorSimilarityResult>;
  hybridSearch: (query: HybridSearchQuery) => Promise<HybridSearchResult>;
  getSemanticEmbeddings: (nodeIds: NodeId[]) => Promise<Record<NodeId, number[]>>;
  semanticClustering: (query: SemanticClusteringQuery) => Promise<SemanticClusteringResult>;
  explainSemanticMatch: (nodeId: NodeId, query: string) => Promise<SearchExplanation>;
  getSearchSuggestions: (partialQuery: string, context?: NodeId[]) => Promise<SearchSuggestion[]>;
  getSearchHistory: (userId?: string) => Promise<SearchHistoryEntry[]>;
  saveSearchToHistory: (entry: Omit<SearchHistoryEntry, 'id' | 'timestamp'>) => Promise<SearchHistoryEntry>;
  getSavedSearches: (userId?: string) => Promise<SavedSemanticSearch[]>;
  saveSearch: (search: Omit<SavedSemanticSearch, 'id' | 'created_at' | 'updated_at'>) => Promise<SavedSemanticSearch>;
  deleteSavedSearch: (searchId: string) => Promise<boolean>;
}