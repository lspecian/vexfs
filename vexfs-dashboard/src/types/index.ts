// VexFS API Types
export interface VexFSCollection {
  id: string;
  name: string;
  description?: string;
  vectorSize: number;
  distance: 'cosine' | 'euclidean' | 'dot';
  createdAt: string;
  updatedAt: string;
  pointsCount: number;
}

export interface VexFSPoint {
  id: string | number;
  vector: number[];
  payload?: Record<string, unknown>;
}

export interface VexFSSearchResult {
  id: string | number;
  score: number;
  payload?: Record<string, unknown>;
  vector?: number[];
}

export interface VexFSSearchRequest {
  vector: number[];
  limit?: number;
  offset?: number;
  filter?: Record<string, unknown>;
  withPayload?: boolean;
  withVector?: boolean;
}

// Dashboard UI Types
export interface DashboardStats {
  totalCollections: number;
  totalPoints: number;
  totalStorage: string;
  serverStatus: 'online' | 'offline' | 'error';
}

export interface NavigationItem {
  id: string;
  label: string;
  path: string;
  icon: string;
}

// API Response Types
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  message?: string;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  pageSize: number;
  hasNext: boolean;
  hasPrev: boolean;
}

// Vector-specific Types
export interface VectorData extends VexFSPoint {
  createdAt?: string;
  updatedAt?: string;
}

export interface VectorSearchQuery {
  vector?: number[];
  vectorId?: string | number;
  k?: number;
  threshold?: number;
  filter?: Record<string, unknown>;
}

export interface VectorVisualizationConfig {
  type: 'bar' | 'line' | 'heatmap';
  maxDimensions?: number;
  showLabels?: boolean;
  colorScheme?: string;
}

export interface VectorListFilters extends Record<string, unknown> {
  search?: string;
  metadataFilter?: Record<string, unknown>;
  sortBy?: 'id' | 'createdAt' | 'similarity';
  sortOrder?: 'asc' | 'desc';
}

// Theme Types
export interface ThemeConfig {
  mode: 'light' | 'dark';
  primaryColor: string;
  secondaryColor: string;
}

// Re-export search types
export * from './search';

// Re-export monitoring types
export * from './monitoring';
