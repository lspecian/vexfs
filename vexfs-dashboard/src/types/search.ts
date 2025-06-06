// Advanced Search Types for VexFS Dashboard

export interface AdvancedSearchQuery {
  type: 'vector' | 'metadata' | 'hybrid';
  collectionId: string;
  vectorQuery?: VectorSimilarityQuery;
  metadataQuery?: MetadataFilterQuery;
  hybridQuery?: HybridSearchQuery;
  pagination?: SearchPagination;
  sorting?: SearchSorting;
}

export interface VectorSimilarityQuery {
  vector?: number[];
  vectorId?: string | number;
  vectorFile?: File;
  textQuery?: string; // For text-to-vector conversion
  k: number;
  threshold?: number;
  distanceMetric?: 'cosine' | 'euclidean' | 'dot_product';
  includeVectorIds?: (string | number)[];
  excludeVectorIds?: (string | number)[];
  metadataPreFilter?: Record<string, unknown>;
}

export interface MetadataFilterQuery {
  conditions: FilterCondition[];
  logicalOperator?: 'AND' | 'OR';
}

export interface FilterCondition {
  id: string;
  field: string;
  operator: FilterOperator;
  value: unknown;
  logicalOperator?: 'AND' | 'OR' | 'NOT';
  group?: string;
}

export type FilterOperator =
  | 'equals'
  | 'not_equals'
  | 'contains'
  | 'not_contains'
  | 'starts_with'
  | 'ends_with'
  | 'greater_than'
  | 'less_than'
  | 'greater_equal'
  | 'less_equal'
  | 'in'
  | 'not_in'
  | 'exists'
  | 'not_exists'
  | 'regex';

export interface HybridSearchQuery {
  vectorQuery: VectorSimilarityQuery;
  metadataQuery: MetadataFilterQuery;
  vectorWeight: number; // 0-1, weight for vector similarity
  metadataWeight: number; // 0-1, weight for metadata relevance
  fusionMethod?: 'rrf' | 'weighted_sum' | 'max_score';
}

export interface SearchPagination {
  page: number;
  pageSize: number;
  offset?: number;
}

export interface SearchSorting {
  field: 'similarity' | 'id' | 'metadata';
  order: 'asc' | 'desc';
  metadataField?: string;
}

export interface AdvancedSearchResult {
  id: string | number;
  score: number;
  vector?: number[];
  payload?: Record<string, unknown>;
  metadata?: {
    similarity?: number;
    metadataScore?: number;
    combinedScore?: number;
    rank?: number;
  };
}

export interface SearchResultsResponse {
  results: AdvancedSearchResult[];
  total: number;
  page: number;
  pageSize: number;
  hasNext: boolean;
  hasPrev: boolean;
  executionTime: number;
  searchId: string;
}

export interface SavedSearch {
  id: string;
  name: string;
  description?: string;
  query: AdvancedSearchQuery;
  createdAt: string;
  updatedAt: string;
  userId?: string;
  isPublic: boolean;
  tags: string[];
  category?: string;
}

export interface SearchHistory {
  id: string;
  query: AdvancedSearchQuery;
  results: SearchResultsResponse;
  executedAt: string;
  executionTime: number;
  userId?: string;
}

export interface SearchAnalytics {
  totalSearches: number;
  averageExecutionTime: number;
  popularSearchTypes: {
    type: string;
    count: number;
    percentage: number;
  }[];
  topCollections: {
    collectionId: string;
    searchCount: number;
  }[];
  performanceMetrics: {
    fastQueries: number; // < 100ms
    mediumQueries: number; // 100ms - 1s
    slowQueries: number; // > 1s
  };
  searchPatterns: {
    hourlyDistribution: number[];
    dailyDistribution: number[];
  };
}

export interface QueryTemplate {
  id: string;
  name: string;
  description: string;
  category: string;
  template: Partial<AdvancedSearchQuery>;
  parameters: QueryParameter[];
}

export interface QueryParameter {
  name: string;
  type: 'string' | 'number' | 'boolean' | 'array' | 'object';
  required: boolean;
  defaultValue?: unknown;
  description: string;
  validation?: {
    min?: number;
    max?: number;
    pattern?: string;
    options?: unknown[];
  };
}

export interface SearchExportOptions {
  format: 'json' | 'csv' | 'pdf' | 'excel';
  includeVectors: boolean;
  includeMetadata: boolean;
  includeScores: boolean;
  maxResults?: number;
  fields?: string[];
}

export interface CollectionSchema {
  fields: SchemaField[];
  vectorDimensions: number;
  distanceMetric: string;
}

export interface SchemaField {
  name: string;
  type: 'string' | 'number' | 'boolean' | 'array' | 'object' | 'date';
  required: boolean;
  indexed: boolean;
  description?: string;
  examples?: unknown[];
}

// UI State Types
export interface SearchUIState {
  activeSearchType: 'vector' | 'metadata' | 'hybrid' | 'saved';
  isSearching: boolean;
  searchResults: SearchResultsResponse | null;
  selectedCollection: string | null;
  searchHistory: SearchHistory[];
  savedSearches: SavedSearch[];
  error: string | null;
  previewMode: boolean;
}

export interface SearchFormState {
  vectorForm: VectorSimilarityQuery;
  metadataForm: MetadataFilterQuery;
  hybridForm: HybridSearchQuery;
  pagination: SearchPagination;
  sorting: SearchSorting;
}
