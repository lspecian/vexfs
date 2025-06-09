// VexGraph Type Definitions
// Matches the backend API types from rust/src/vexgraph/

export type NodeId = string;
export type EdgeId = string;

export type NodeType =
  | 'File'
  | 'Directory'
  | 'Symlink'
  | 'Device'
  | 'Custom';

export type EdgeType =
  | 'Contains'
  | 'References'
  | 'DependsOn'
  | 'SimilarTo'
  | 'Custom';

export type PropertyType =
  | 'String'
  | 'Integer'
  | 'Float'
  | 'Boolean'
  | 'Array'
  | 'Object';

export type TraversalAlgorithm =
  | 'BreadthFirstSearch'
  | 'DepthFirstSearch'
  | 'Dijkstra'
  | 'TopologicalSort';

// API Request Types
export interface CreateNodeRequest {
  inode_number: number;
  node_type: NodeType;
  properties?: Record<string, PropertyType>;
}

export interface CreateEdgeRequest {
  source_id: NodeId;
  target_id: NodeId;
  edge_type: EdgeType;
  weight?: number;
  properties?: Record<string, PropertyType>;
}

export interface UpdateNodeRequest {
  properties: Record<string, PropertyType>;
}

export interface UpdateEdgeRequest {
  weight?: number;
  properties?: Record<string, PropertyType>;
}

// API Response Types
export interface NodeResponse {
  id: NodeId;
  inode_number: number;
  node_type: NodeType;
  properties: Record<string, PropertyType>;
  outgoing_edges: EdgeId[];
  incoming_edges: EdgeId[];
  created_at: string;
  updated_at: string;
}

export interface EdgeResponse {
  id: EdgeId;
  source_id: NodeId;
  target_id: NodeId;
  edge_type: EdgeType;
  weight: number;
  properties: Record<string, PropertyType>;
  created_at: string;
  updated_at: string;
}

// Traversal Types
export interface TraversalQuery {
  algorithm: TraversalAlgorithm;
  start_node: NodeId;
  end_node?: NodeId;
  max_depth?: number;
  max_results?: number;
  node_filter?: NodeType;
  edge_filter?: EdgeType;
  weight_threshold?: number;
  timeout_ms?: number;
}

export interface TraversalResult {
  algorithm: TraversalAlgorithm;
  start_node: NodeId;
  end_node?: NodeId;
  path?: NodeId[];
  visited_nodes: NodeId[];
  traversed_edges: EdgeId[];
  total_weight?: number;
  execution_time_ms: number;
  success: boolean;
  error_message?: string;
}

// Query Types
export interface NodeFilters {
  node_type?: NodeType;
  inode_number?: number;
  properties?: Record<string, any>;
  created_after?: string;
  created_before?: string;
}

export interface EdgeFilters {
  edge_type?: EdgeType;
  source_id?: NodeId;
  target_id?: NodeId;
  weight_min?: number;
  weight_max?: number;
  properties?: Record<string, any>;
}

export interface NeighborOptions {
  direction?: 'incoming' | 'outgoing' | 'both';
  edge_types?: EdgeType[];
  max_depth?: number;
  include_edge_data?: boolean;
}

// Statistics Types
export interface GraphStatistics {
  node_count: number;
  edge_count: number;
  node_types: Record<NodeType, number>;
  edge_types: Record<EdgeType, number>;
  average_degree: number;
  density: number;
  connected_components: number;
  largest_component_size: number;
  clustering_coefficient?: number;
  diameter?: number;
}

// UI-specific Types
export interface GraphLayoutOptions {
  name: 'cose' | 'grid' | 'circle' | 'breadthfirst' | 'concentric' | 'dagre';
  animate?: boolean;
  fit?: boolean;
  padding?: number;
  randomize?: boolean;
}

export interface GraphStyleOptions {
  nodeColor?: string;
  edgeColor?: string;
  selectedNodeColor?: string;
  selectedEdgeColor?: string;
  nodeSize?: number;
  edgeWidth?: number;
  fontSize?: number;
  backgroundColor?: string;
}

export interface GraphViewState {
  nodes: NodeResponse[];
  edges: EdgeResponse[];
  selectedNodes: NodeId[];
  selectedEdges: EdgeId[];
  layout: GraphLayoutOptions;
  style: GraphStyleOptions;
  isLoading: boolean;
  error?: string;
}

// Search Types
export interface GraphSearchQuery {
  query: string;
  search_type: 'semantic' | 'property' | 'traversal';
  node_types?: NodeType[];
  edge_types?: EdgeType[];
  max_results?: number;
}

export interface GraphSearchResult {
  nodes: NodeResponse[];
  edges: EdgeResponse[];
  relevance_scores?: Record<NodeId | EdgeId, number>;
  execution_time_ms: number;
  total_results: number;
}

// Analytics Types
export interface GraphAnalytics {
  degree_distribution: { degree: number; count: number }[];
  centrality_measures: {
    betweenness: Record<NodeId, number>;
    closeness: Record<NodeId, number>;
    eigenvector: Record<NodeId, number>;
    pagerank: Record<NodeId, number>;
  };
  clustering_coefficients: Record<NodeId, number>;
  shortest_paths_stats: {
    average_path_length: number;
    diameter: number;
    radius: number;
  };
  community_detection?: {
    communities: NodeId[][];
    modularity: number;
  };
}

// Schema Types
export interface NodeTypeSchema {
  type: NodeType;
  required_properties: string[];
  optional_properties: string[];
  property_types: Record<string, PropertyType>;
  constraints?: Record<string, any>;
}

export interface EdgeTypeSchema {
  type: EdgeType;
  allowed_source_types: NodeType[];
  allowed_target_types: NodeType[];
  required_properties: string[];
  optional_properties: string[];
  property_types: Record<string, PropertyType>;
  weight_constraints?: { min?: number; max?: number };
}

export interface GraphSchema {
  node_types: NodeTypeSchema[];
  edge_types: EdgeTypeSchema[];
  global_constraints?: Record<string, any>;
  version: string;
  created_at: string;
  updated_at: string;
}

// Error Types
export interface GraphError {
  error: string;
  message: string;
  timestamp: string;
}

// Navigation Types (extending existing)
export interface GraphNavigationItem {
  id: 'graph';
  label: 'Graph';
  path: '/graph';
  icon: 'graph';
}