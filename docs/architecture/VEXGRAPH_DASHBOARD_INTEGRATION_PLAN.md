# VexGraph Dashboard Integration Plan

## Overview

This document outlines the architectural plan for integrating VexGraph Phase 2 functionality into the existing VexFS dashboard. Rather than building a separate UI, we will extend the current React-based dashboard with Graph capabilities while maintaining consistency with the existing Material-UI design system and architectural patterns.

## Current Dashboard Architecture

### Existing Structure
- **Framework**: React 18 with TypeScript
- **UI Library**: Material-UI (MUI) v5
- **Routing**: React Router v6
- **State Management**: React Context + hooks
- **Build Tool**: Vite
- **Testing**: Playwright for E2E, Jest for unit tests

### Current Navigation Structure
```
├── Dashboard (/)
├── Collections (/collections)
├── Vector Search (/search)
├── Monitoring (/monitoring)
└── Settings (/settings)
```

### Existing API Service
- **Location**: `vexfs-dashboard/src/services/api.ts`
- **Pattern**: Class-based service with axios
- **Features**: Collections, vectors, search, monitoring
- **Error Handling**: Centralized with interceptors
- **Base URL**: `http://localhost:7680`

## VexGraph Backend API

### Available Endpoints
```
Node Operations:
- POST   /api/v1/nodes
- GET    /api/v1/nodes/:id
- PUT    /api/v1/nodes/:id
- DELETE /api/v1/nodes/:id
- GET    /api/v1/nodes
- GET    /api/v1/nodes/by-inode/:inode

Edge Operations:
- POST   /api/v1/edges
- GET    /api/v1/edges/:id
- PUT    /api/v1/edges/:id
- DELETE /api/v1/edges/:id
- GET    /api/v1/edges

Traversal Operations:
- GET    /api/v1/traversal
- GET    /api/v1/traversal/bfs
- GET    /api/v1/traversal/dfs
- GET    /api/v1/traversal/dijkstra
- GET    /api/v1/traversal/topological

Query Operations:
- GET    /api/v1/query/nodes
- GET    /api/v1/query/edges
- GET    /api/v1/query/neighbors

Utility Operations:
- GET    /api/v1/stats
- GET    /api/v1/health
```

## Integration Plan

### Phase 1: Foundation (Subtasks 9, 10)

#### 1.1 Extend API Service
**File**: `vexfs-dashboard/src/services/api.ts`

Add VexGraph methods to the existing `VexFSApiService` class:

```typescript
// Node operations
async createNode(request: CreateNodeRequest): Promise<NodeResponse>
async getNode(id: NodeId): Promise<NodeResponse>
async getNodeByInode(inode: number): Promise<NodeResponse>
async updateNode(id: NodeId, updates: UpdateNodeRequest): Promise<NodeResponse>
async deleteNode(id: NodeId): Promise<boolean>
async listNodes(filters?: NodeFilters): Promise<NodeResponse[]>

// Edge operations
async createEdge(request: CreateEdgeRequest): Promise<EdgeResponse>
async getEdge(id: EdgeId): Promise<EdgeResponse>
async updateEdge(id: EdgeId, updates: UpdateEdgeRequest): Promise<EdgeResponse>
async deleteEdge(id: EdgeId): Promise<boolean>
async listEdges(filters?: EdgeFilters): Promise<EdgeResponse[]>

// Traversal operations
async executeTraversal(query: TraversalQuery): Promise<TraversalResult>
async breadthFirstSearch(params: TraversalParams): Promise<TraversalResult>
async depthFirstSearch(params: TraversalParams): Promise<TraversalResult>
async dijkstraShortestPath(params: TraversalParams): Promise<TraversalResult>

// Query operations
async queryNodes(filters: NodeQueryFilters): Promise<NodeResponse[]>
async queryEdges(filters: EdgeQueryFilters): Promise<EdgeResponse[]>
async getNeighbors(nodeId: NodeId, options?: NeighborOptions): Promise<NodeResponse[]>

// Statistics
async getGraphStats(): Promise<GraphStatistics>
```

#### 1.2 Add Graph Navigation
**Files**: 
- `vexfs-dashboard/src/components/Layout/Sidebar.tsx`
- `vexfs-dashboard/src/App.tsx`

Update navigation items:
```typescript
const navigationItems: NavigationItem[] = [
  // ... existing items
  {
    id: 'graph',
    label: 'Graph',
    path: '/graph',
    icon: 'graph',
  },
  // ... rest of items
];
```

Add Graph icon and route:
```typescript
// In Sidebar.tsx
import { AccountTree as GraphIcon } from '@mui/icons-material';

// In App.tsx
const Graph = React.lazy(() => import('./pages/Graph'));
```

### Phase 2: Core Visualization (Subtask 1)

#### 2.1 Graph Visualization Component
**File**: `vexfs-dashboard/src/components/Graph/GraphVisualization.tsx`

**Library Selection**: Cytoscape.js with React wrapper
- **Pros**: Mature, performant, extensive layout algorithms
- **Integration**: `cytoscape` + `react-cytoscapejs`
- **Styling**: Custom CSS to match Material-UI theme

**Component Structure**:
```typescript
interface GraphVisualizationProps {
  nodes: NodeResponse[];
  edges: EdgeResponse[];
  selectedNodes?: NodeId[];
  selectedEdges?: EdgeId[];
  onNodeSelect?: (nodeId: NodeId) => void;
  onEdgeSelect?: (edgeId: EdgeId) => void;
  onNodeDoubleClick?: (nodeId: NodeId) => void;
  layout?: LayoutOptions;
  style?: GraphStyleOptions;
}

const GraphVisualization: React.FC<GraphVisualizationProps> = ({
  nodes,
  edges,
  selectedNodes = [],
  selectedEdges = [],
  onNodeSelect,
  onEdgeSelect,
  onNodeDoubleClick,
  layout = { name: 'cose' },
  style = defaultGraphStyle,
}) => {
  // Implementation with Cytoscape.js
};
```

**Features**:
- Zoom, pan, fit controls
- Node/edge selection with visual feedback
- Layout algorithms (force-directed, hierarchical, circular)
- Theme integration (light/dark mode support)
- Performance optimization for large graphs
- Export capabilities (PNG, SVG, JSON)

### Phase 3: CRUD Operations (Subtask 2)

#### 3.1 Node Management
**Files**:
- `vexfs-dashboard/src/components/Graph/NodeManagement/CreateNodeDialog.tsx`
- `vexfs-dashboard/src/components/Graph/NodeManagement/EditNodeDialog.tsx`
- `vexfs-dashboard/src/components/Graph/NodeManagement/NodePropertiesPanel.tsx`

**Features**:
- Material-UI dialogs following existing patterns
- Form validation with react-hook-form
- Property editor with dynamic types
- Inode number integration
- Node type selection with icons

#### 3.2 Edge Management
**Files**:
- `vexfs-dashboard/src/components/Graph/EdgeManagement/CreateEdgeDialog.tsx`
- `vexfs-dashboard/src/components/Graph/EdgeManagement/EditEdgeDialog.tsx`
- `vexfs-dashboard/src/components/Graph/EdgeManagement/EdgePropertiesPanel.tsx`

**Features**:
- Visual edge creation (drag between nodes)
- Weight and property editing
- Edge type categorization
- Bidirectional edge support

### Phase 4: Query Builder (Subtask 3)

#### 4.1 Traversal Query Builder
**File**: `vexfs-dashboard/src/components/Graph/QueryBuilder/TraversalQueryBuilder.tsx`

**Interface Design**:
```typescript
interface TraversalQueryBuilderProps {
  onQueryExecute: (query: TraversalQuery) => void;
  availableNodes: NodeResponse[];
  isLoading?: boolean;
}
```

**Features**:
- Algorithm selection (BFS, DFS, Dijkstra, Topological)
- Start/end node selection with autocomplete
- Filter configuration (node types, edge types, weights)
- Query validation and preview
- Results visualization on graph

### Phase 5: Search Integration (Subtask 4)

#### 5.1 Semantic Search Panel
**File**: `vexfs-dashboard/src/components/Graph/Search/GraphSemanticSearch.tsx`

**Integration Points**:
- Reuse existing search UI patterns from `src/components/Search/`
- Vector similarity search for nodes
- Natural language query processing
- Result highlighting on graph visualization
- Search history and saved searches

### Phase 6: Analytics Dashboard (Subtask 5)

#### 6.1 Graph Analytics Widgets
**Files**:
- `vexfs-dashboard/src/components/Graph/Analytics/GraphMetrics.tsx`
- `vexfs-dashboard/src/components/Graph/Analytics/NodeDegreeChart.tsx`
- `vexfs-dashboard/src/components/Graph/Analytics/CentralityAnalysis.tsx`

**Metrics**:
- Node count, edge count, density
- Degree distribution histograms
- Centrality measures (betweenness, closeness, eigenvector)
- Connected components analysis
- Clustering coefficient

### Phase 7: Real-time Updates (Subtask 6)

#### 7.1 Live Graph Updates
**Implementation**:
- WebSocket connection for real-time events
- Incremental graph updates (add/remove/modify)
- Conflict resolution for concurrent edits
- Performance optimization for large graphs

### Phase 8: Schema Management (Subtask 7)

#### 8.1 Graph Schema Editor
**File**: `vexfs-dashboard/src/components/Graph/Schema/GraphSchemaEditor.tsx`

**Features**:
- Node type definitions with properties
- Edge type definitions with constraints
- Schema validation and enforcement
- Import/export schema configurations

## File Structure

```
vexfs-dashboard/src/
├── components/
│   ├── Graph/
│   │   ├── GraphVisualization.tsx
│   │   ├── Analytics/
│   │   │   ├── GraphMetrics.tsx
│   │   │   ├── NodeDegreeChart.tsx
│   │   │   └── CentralityAnalysis.tsx
│   │   ├── EdgeManagement/
│   │   │   ├── CreateEdgeDialog.tsx
│   │   │   ├── EditEdgeDialog.tsx
│   │   │   └── EdgePropertiesPanel.tsx
│   │   ├── NodeManagement/
│   │   │   ├── CreateNodeDialog.tsx
│   │   │   ├── EditNodeDialog.tsx
│   │   │   └── NodePropertiesPanel.tsx
│   │   ├── QueryBuilder/
│   │   │   └── TraversalQueryBuilder.tsx
│   │   ├── Schema/
│   │   │   └── GraphSchemaEditor.tsx
│   │   └── Search/
│   │       └── GraphSemanticSearch.tsx
│   └── ... (existing components)
├── pages/
│   ├── Graph.tsx
│   └── ... (existing pages)
├── services/
│   ├── api.ts (extended)
│   └── ... (existing services)
├── types/
│   ├── graph.ts (new)
│   └── ... (existing types)
└── hooks/
    ├── useGraph.ts (new)
    └── ... (existing hooks)
```

## Type Definitions

### New Types File
**File**: `vexfs-dashboard/src/types/graph.ts`

```typescript
// Core types matching backend
export type NodeId = string;
export type EdgeId = string;

export enum NodeType {
  File = 'File',
  Directory = 'Directory',
  Symlink = 'Symlink',
  Device = 'Device',
  Custom = 'Custom',
}

export enum EdgeType {
  Contains = 'Contains',
  References = 'References',
  DependsOn = 'DependsOn',
  SimilarTo = 'SimilarTo',
  Custom = 'Custom',
}

export enum PropertyType {
  String = 'String',
  Integer = 'Integer',
  Float = 'Float',
  Boolean = 'Boolean',
  Array = 'Array',
  Object = 'Object',
}

// API request/response types
export interface CreateNodeRequest {
  inode_number: number;
  node_type: NodeType;
  properties?: Record<string, PropertyType>;
}

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

// ... additional types
```

## Integration with Existing Systems

### 1. Error Handling
- Reuse existing `ErrorBoundary` and `ErrorMessage` components
- Extend error types for graph-specific errors
- Integrate with existing notification system (notistack)

### 2. Loading States
- Use existing loading patterns and `CircularProgress` components
- Implement skeleton loading for graph visualization
- Progressive loading for large graphs

### 3. Theming
- Extend Material-UI theme for graph-specific colors
- Support light/dark mode switching
- Consistent spacing and typography

### 4. Monitoring Integration
- Add graph metrics to existing monitoring dashboard
- Health checks for VexGraph API endpoints
- Performance monitoring for graph operations

### 5. Authentication
- Reuse existing `AuthProvider` and authentication patterns
- Role-based access control for graph operations
- Session management consistency

## Performance Considerations

### 1. Graph Visualization
- Virtualization for large graphs (>1000 nodes)
- Level-of-detail rendering
- Efficient layout algorithms
- Canvas-based rendering for performance

### 2. API Optimization
- Pagination for node/edge listings
- Caching strategies for frequently accessed data
- Batch operations for bulk updates
- WebSocket for real-time updates

### 3. Memory Management
- Efficient data structures for graph representation
- Garbage collection for removed elements
- Memory profiling and optimization

## Testing Strategy

### 1. Unit Tests
- Component testing with React Testing Library
- API service method testing with mock responses
- Hook testing for graph state management

### 2. Integration Tests
- Page-level testing with Playwright
- API integration testing
- Cross-component interaction testing

### 3. E2E Tests
- Complete user workflows
- Graph visualization interactions
- Performance testing with large datasets

## Deployment Considerations

### 1. Build Optimization
- Code splitting for graph components
- Lazy loading of visualization libraries
- Bundle size optimization

### 2. Browser Compatibility
- Modern browser support (ES2020+)
- WebGL fallbacks for visualization
- Progressive enhancement

### 3. Accessibility
- Keyboard navigation for graph elements
- Screen reader support
- High contrast mode support

## Migration Path

### Phase 1: Foundation (Week 1)
- Extend API service
- Add navigation
- Basic page structure

### Phase 2: Core Features (Week 2-3)
- Graph visualization
- Basic CRUD operations
- Query builder

### Phase 3: Advanced Features (Week 4-5)
- Analytics dashboard
- Real-time updates
- Schema management

### Phase 4: Polish & Testing (Week 6)
- Integration testing
- Performance optimization
- Documentation

## Success Metrics

1. **Functionality**: All VexGraph Phase 2 features accessible via UI
2. **Performance**: Graph visualization handles 1000+ nodes smoothly
3. **Usability**: Consistent UX with existing dashboard
4. **Maintainability**: Clean, documented, testable code
5. **Integration**: Seamless operation with existing features

This integration plan ensures that VexGraph functionality becomes a natural extension of the existing VexFS dashboard while maintaining architectural consistency and user experience quality.