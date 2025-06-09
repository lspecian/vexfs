# VexGraph Dashboard Developer Guide

This guide provides comprehensive information for developers working with the VexGraph Dashboard codebase.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Project Structure](#project-structure)
3. [Component Library](#component-library)
4. [API Integration](#api-integration)
5. [State Management](#state-management)
6. [Testing Strategy](#testing-strategy)
7. [Performance Optimization](#performance-optimization)
8. [Development Workflow](#development-workflow)
9. [Deployment](#deployment)
10. [Contributing](#contributing)

## Architecture Overview

### Technology Stack

- **Frontend Framework**: React 18 with TypeScript
- **UI Library**: Material-UI (MUI) v5
- **Graph Visualization**: React Flow v11
- **State Management**: React Context + Custom Hooks
- **Real-time Communication**: Socket.IO
- **Testing**: Playwright for E2E, Jest for unit tests
- **Build Tool**: Vite
- **Package Manager**: npm

### System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    VexGraph Dashboard                       │
├─────────────────────────────────────────────────────────────┤
│  Presentation Layer                                         │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐          │
│  │   Pages     │ │ Components  │ │   Layouts   │          │
│  └─────────────┘ └─────────────┘ └─────────────┘          │
├─────────────────────────────────────────────────────────────┤
│  Business Logic Layer                                       │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐          │
│  │   Hooks     │ │  Services   │ │   Utils     │          │
│  └─────────────┘ └─────────────┘ └─────────────┘          │
├─────────────────────────────────────────────────────────────┤
│  Data Layer                                                 │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐          │
│  │  API Client │ │ WebSocket   │ │   Cache     │          │
│  └─────────────┘ └─────────────┘ └─────────────┘          │
├─────────────────────────────────────────────────────────────┤
│  External Services                                          │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐          │
│  │ VexGraph API│ │ WebSocket   │ │ File System │          │
│  └─────────────┘ └─────────────┘ └─────────────┘          │
└─────────────────────────────────────────────────────────────┘
```

## Project Structure

```
vexfs-dashboard/
├── public/                     # Static assets
├── src/
│   ├── components/            # Reusable UI components
│   │   ├── Auth/             # Authentication components
│   │   ├── Common/           # Shared components
│   │   ├── Graph/            # Graph-specific components
│   │   ├── Layout/           # Layout components
│   │   ├── Monitoring/       # Monitoring components
│   │   ├── Search/           # Search components
│   │   └── Vectors/          # Vector components
│   ├── hooks/                # Custom React hooks
│   ├── pages/                # Page components
│   ├── services/             # API and external services
│   ├── types/                # TypeScript type definitions
│   ├── utils/                # Utility functions
│   ├── theme/                # MUI theme configuration
│   └── config/               # Configuration files
├── tests/                    # Test files
│   ├── integration/          # Integration tests
│   ├── e2e/                  # End-to-end tests
│   ├── performance/          # Performance tests
│   └── setup/                # Test setup and mocks
├── docs/                     # Documentation
└── playwright.config.ts      # Playwright configuration
```

## Component Library

### Core Components

#### Graph Components

**GraphVisualization** (`src/components/Graph/GraphVisualization.tsx`)
- Primary graph rendering component using React Flow
- Handles node/edge rendering, selection, and interactions
- Supports multiple layout algorithms
- Optimized for large graphs (1000+ nodes)

```typescript
interface GraphVisualizationProps {
  nodes: NodeResponse[];
  edges: EdgeResponse[];
  onNodeSelect?: (nodeIds: string[]) => void;
  onEdgeSelect?: (edgeIds: string[]) => void;
  onNodeDoubleClick?: (node: NodeResponse) => void;
  onEdgeDoubleClick?: (edge: EdgeResponse) => void;
  height?: number;
  enableMiniMap?: boolean;
  enableControls?: boolean;
  enableBackground?: boolean;
}
```

**NodeEdgeManager** (`src/components/Graph/NodeEdgeManager.tsx`)
- CRUD operations for nodes and edges
- Form validation and error handling
- Batch operations support
- Schema-aware property editing

**QueryBuilder** (`src/components/Graph/QueryBuilder.tsx`)
- Visual query construction interface
- Support for multiple traversal algorithms
- Filter and parameter configuration
- Query template management

**SchemaManager** (`src/components/Graph/SchemaManager.tsx`)
- Schema definition and editing
- Validation rule configuration
- Schema evolution and migration
- Import/export functionality

#### Real-time Components

**RealTimeProvider** (`src/components/Graph/RealTimeProvider.tsx`)
- WebSocket connection management
- Real-time update distribution
- Conflict resolution handling
- Connection status monitoring

**WebSocketManager** (`src/components/Graph/WebSocketManager.tsx`)
- Low-level WebSocket operations
- Message queuing and retry logic
- Connection health monitoring
- Event routing

#### Analytics Components

**GraphAnalyticsDashboard** (`src/components/Graph/GraphAnalyticsDashboard.tsx`)
- Comprehensive analytics visualization
- Centrality measures display
- Community detection results
- Performance metrics

**PerformanceCharts** (`src/components/Graph/PerformanceCharts.tsx`)
- Real-time performance monitoring
- Query execution metrics
- Memory usage tracking
- Network performance

### Component Patterns

#### Higher-Order Components (HOCs)

**withErrorBoundary**
```typescript
export function withErrorBoundary<P extends object>(
  Component: React.ComponentType<P>
): React.ComponentType<P> {
  return function WrappedComponent(props: P) {
    return (
      <ErrorBoundary>
        <Component {...props} />
      </ErrorBoundary>
    );
  };
}
```

**withPerformanceMonitoring**
```typescript
export function withPerformanceMonitoring<P extends object>(
  Component: React.ComponentType<P>,
  componentName: string
): React.ComponentType<P> {
  return function MonitoredComponent(props: P) {
    const { startMeasurement, endMeasurement } = usePerformanceMonitor();
    
    useEffect(() => {
      const measurementId = startMeasurement(componentName);
      return () => endMeasurement(measurementId);
    }, []);
    
    return <Component {...props} />;
  };
}
```

#### Custom Hooks

**useVexFS** (`src/hooks/useVexFS.ts`)
```typescript
export function useVexFS() {
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  const api = useMemo(() => vexfsApi, []);
  
  const checkConnection = useCallback(async () => {
    try {
      const healthy = await api.checkVexGraphHealth();
      setIsConnected(healthy);
      setError(null);
    } catch (err) {
      setIsConnected(false);
      setError(err instanceof Error ? err.message : 'Connection failed');
    }
  }, [api]);
  
  return { api, isConnected, error, checkConnection };
}
```

**useRealTime** (`src/hooks/useRealTime.ts`)
```typescript
export function useRealTime() {
  const [socket, setSocket] = useState<Socket | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  
  const connect = useCallback(() => {
    const newSocket = io('/vexgraph', {
      transports: ['websocket'],
      upgrade: true,
    });
    
    newSocket.on('connect', () => setIsConnected(true));
    newSocket.on('disconnect', () => setIsConnected(false));
    
    setSocket(newSocket);
    return newSocket;
  }, []);
  
  return { socket, isConnected, connect };
}
```

## API Integration

### Service Layer Architecture

**Base API Client** (`src/services/api.ts`)
```typescript
class VexFSApiClient {
  private baseURL: string;
  private timeout: number;
  
  constructor(config: ApiConfig) {
    this.baseURL = config.baseURL;
    this.timeout = config.timeout || 30000;
  }
  
  private async request<T>(
    endpoint: string,
    options: RequestOptions = {}
  ): Promise<T> {
    const url = `${this.baseURL}${endpoint}`;
    const config: RequestInit = {
      method: options.method || 'GET',
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
      body: options.body ? JSON.stringify(options.body) : undefined,
      signal: AbortSignal.timeout(this.timeout),
    };
    
    const response = await fetch(url, config);
    
    if (!response.ok) {
      throw new ApiError(response.status, await response.text());
    }
    
    return response.json();
  }
}
```

### API Methods

**Node Operations**
```typescript
// Create node
async createNode(nodeData: CreateNodeRequest): Promise<NodeResponse> {
  return this.request('/api/v1/vexgraph/nodes', {
    method: 'POST',
    body: nodeData,
  });
}

// Batch create nodes
async batchCreateNodes(nodes: CreateNodeRequest[]): Promise<NodeResponse[]> {
  return this.request('/api/v1/vexgraph/nodes/batch', {
    method: 'POST',
    body: { nodes },
  });
}

// List nodes with pagination
async listNodes(
  filters: NodeFilters = {},
  limit = 100,
  offset = 0
): Promise<PaginatedResponse<NodeResponse>> {
  const params = new URLSearchParams({
    limit: limit.toString(),
    offset: offset.toString(),
    ...filters,
  });
  
  return this.request(`/api/v1/vexgraph/nodes?${params}`);
}
```

**Query Operations**
```typescript
// Execute traversal
async executeTraversal(query: TraversalQuery): Promise<TraversalResult> {
  return this.request('/api/v1/vexgraph/traversal', {
    method: 'POST',
    body: query,
  });
}

// Semantic search
async semanticSearch(
  query: string,
  maxResults = 10
): Promise<GraphSearchResult> {
  return this.request('/api/v1/vexgraph/search', {
    method: 'POST',
    body: {
      query,
      search_type: 'semantic',
      max_results: maxResults,
    },
  });
}
```

### Error Handling

**API Error Types**
```typescript
export class ApiError extends Error {
  constructor(
    public status: number,
    public message: string,
    public details?: any
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

export class NetworkError extends Error {
  constructor(message: string, public originalError: Error) {
    super(message);
    this.name = 'NetworkError';
  }
}

export class ValidationError extends Error {
  constructor(
    message: string,
    public field: string,
    public value: any
  ) {
    super(message);
    this.name = 'ValidationError';
  }
}
```

**Error Handling Hook**
```typescript
export function useErrorHandler() {
  const { enqueueSnackbar } = useSnackbar();
  
  const handleError = useCallback((error: Error) => {
    if (error instanceof ApiError) {
      enqueueSnackbar(`API Error: ${error.message}`, { variant: 'error' });
    } else if (error instanceof NetworkError) {
      enqueueSnackbar('Network connection failed', { variant: 'error' });
    } else if (error instanceof ValidationError) {
      enqueueSnackbar(`Validation Error: ${error.message}`, { variant: 'error' });
    } else {
      enqueueSnackbar('An unexpected error occurred', { variant: 'error' });
    }
    
    // Log to console in development
    if (import.meta.env.DEV) {
      console.error('Error details:', error);
    }
  }, [enqueueSnackbar]);
  
  return { handleError };
}
```

## State Management

### Context Providers

**Graph Context**
```typescript
interface GraphContextValue {
  nodes: NodeResponse[];
  edges: EdgeResponse[];
  selectedNodes: string[];
  selectedEdges: string[];
  isLoading: boolean;
  error: string | null;
  
  // Actions
  setNodes: (nodes: NodeResponse[]) => void;
  setEdges: (edges: EdgeResponse[]) => void;
  selectNodes: (nodeIds: string[]) => void;
  selectEdges: (edgeIds: string[]) => void;
  clearSelection: () => void;
  refreshData: () => Promise<void>;
}

export const GraphContext = createContext<GraphContextValue | null>(null);

export function GraphProvider({ children }: { children: React.ReactNode }) {
  const [state, setState] = useState<GraphState>(initialState);
  const { api } = useVexFS();
  
  const refreshData = useCallback(async () => {
    setState(prev => ({ ...prev, isLoading: true, error: null }));
    
    try {
      const [nodesResult, edgesResult] = await Promise.all([
        api.listNodes({}, 1000, 0),
        api.listEdges({}, 1000, 0),
      ]);
      
      setState(prev => ({
        ...prev,
        nodes: nodesResult.items,
        edges: edgesResult.items,
        isLoading: false,
      }));
    } catch (error) {
      setState(prev => ({
        ...prev,
        error: error instanceof Error ? error.message : 'Failed to load data',
        isLoading: false,
      }));
    }
  }, [api]);
  
  const value: GraphContextValue = {
    ...state,
    refreshData,
    // ... other actions
  };
  
  return (
    <GraphContext.Provider value={value}>
      {children}
    </GraphContext.Provider>
  );
}
```

### State Optimization

**Memoization Strategies**
```typescript
// Memoize expensive computations
const processedNodes = useMemo(() => {
  return nodes.map(node => ({
    ...node,
    position: calculateNodePosition(node, layout),
    style: getNodeStyle(node, theme),
  }));
}, [nodes, layout, theme]);

// Memoize event handlers
const handleNodeClick = useCallback((nodeId: string) => {
  setSelectedNodes(prev => 
    prev.includes(nodeId) 
      ? prev.filter(id => id !== nodeId)
      : [...prev, nodeId]
  );
}, []);

// Memoize component props
const graphProps = useMemo(() => ({
  nodes: processedNodes,
  edges: processedEdges,
  onNodeClick: handleNodeClick,
  onEdgeClick: handleEdgeClick,
}), [processedNodes, processedEdges, handleNodeClick, handleEdgeClick]);
```

## Testing Strategy

### Test Structure

```
tests/
├── integration/              # Component integration tests
│   └── component-integration.spec.ts
├── e2e/                     # End-to-end workflow tests
│   └── complete-workflows.spec.ts
├── performance/             # Performance and load tests
│   └── performance-tests.spec.ts
└── setup/                   # Test configuration and mocks
    └── mock-server.ts
```

### Testing Patterns

**Component Testing**
```typescript
// Example component test
test('should render graph visualization', async ({ page }) => {
  await page.goto('/ui/graph');
  
  // Wait for component to load
  await page.waitForSelector('[data-testid="graph-visualization"]');
  
  // Verify nodes are rendered
  const nodes = page.locator('.react-flow__node');
  const nodeCount = await nodes.count();
  expect(nodeCount).toBeGreaterThan(0);
  
  // Test interaction
  await nodes.first().click();
  await expect(page.locator('.react-flow__node.selected')).toBeVisible();
});
```

**Integration Testing**
```typescript
// Example integration test
test('should sync data between components', async ({ page }) => {
  await page.goto('/ui/graph');
  
  // Create node in one component
  await page.click('text=Create Node');
  await page.fill('input[name="name"]', 'test-node');
  await page.click('button[type="submit"]');
  
  // Verify it appears in visualization
  await expect(page.locator('text=test-node')).toBeVisible();
  
  // Verify it appears in analytics
  await page.click('text=Analytics');
  await expect(page.locator('[data-testid="node-count"]')).toContainText('1');
});
```

**Performance Testing**
```typescript
// Example performance test
test('should handle large graphs efficiently', async ({ page }) => {
  await page.goto('/ui/graph');
  
  // Generate large dataset
  await page.click('text=Generate Large Dataset');
  await page.selectOption('select[name="nodeCount"]', '1000');
  await page.click('text=Generate');
  
  // Measure loading time
  const startTime = Date.now();
  await page.waitForSelector('.react-flow__node');
  const loadTime = Date.now() - startTime;
  
  expect(loadTime).toBeLessThan(10000); // Should load within 10 seconds
});
```

### Mock Server Setup

```typescript
// Mock server for testing
export const server = setupServer(
  // Node operations
  http.get('/api/v1/vexgraph/nodes', ({ request }) => {
    const url = new URL(request.url);
    const limit = parseInt(url.searchParams.get('limit') || '100');
    const offset = parseInt(url.searchParams.get('offset') || '0');
    
    return HttpResponse.json({
      success: true,
      data: {
        items: mockNodes.slice(offset, offset + limit),
        total: mockNodes.length,
        page: Math.floor(offset / limit) + 1,
        pageSize: limit,
      },
    });
  }),
  
  // Add more handlers...
);
```

## Performance Optimization

### Rendering Optimization

**Virtualization for Large Graphs**
```typescript
// Use React Flow's built-in virtualization
const GraphVisualization: React.FC<GraphVisualizationProps> = ({
  nodes,
  edges,
  ...props
}) => {
  // Only render visible nodes
  const visibleNodes = useMemo(() => {
    if (nodes.length > 1000) {
      return nodes.filter(node => isNodeVisible(node, viewport));
    }
    return nodes;
  }, [nodes, viewport]);
  
  return (
    <ReactFlow
      nodes={visibleNodes}
      edges={edges}
      onViewportChange={setViewport}
      {...props}
    />
  );
};
```

**Memoization and Optimization**
```typescript
// Memoize expensive calculations
const nodePositions = useMemo(() => {
  return calculateLayout(nodes, edges, layoutAlgorithm);
}, [nodes, edges, layoutAlgorithm]);

// Use React.memo for pure components
export const NodeComponent = React.memo<NodeComponentProps>(({ node }) => {
  return (
    <div className="node" style={getNodeStyle(node)}>
      {node.properties.name}
    </div>
  );
});

// Debounce expensive operations
const debouncedSearch = useMemo(
  () => debounce(performSearch, 300),
  [performSearch]
);
```

### Bundle Optimization

**Code Splitting**
```typescript
// Lazy load heavy components
const GraphAnalytics = lazy(() => import('./GraphAnalytics'));
const SemanticSearch = lazy(() => import('./SemanticSearch'));

// Use dynamic imports for large libraries
const loadD3 = () => import('d3').then(d3 => d3);
```

**Asset Optimization**
```typescript
// Optimize images and assets
import { defineConfig } from 'vite';

export default defineConfig({
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          vendor: ['react', 'react-dom'],
          ui: ['@mui/material', '@mui/icons-material'],
          graph: ['reactflow', 'd3'],
        },
      },
    },
  },
  optimizeDeps: {
    include: ['react', 'react-dom', '@mui/material'],
  },
});
```

## Development Workflow

### Getting Started

1. **Clone and Install**
```bash
git clone <repository-url>
cd vexfs-dashboard
npm install
```

2. **Environment Setup**
```bash
cp .env.example .env
# Edit .env with your configuration
```

3. **Start Development Server**
```bash
npm run dev
```

4. **Run Tests**
```bash
npm test                    # Run all tests
npm run test:ui            # Run tests with UI
npm run test:headed        # Run tests in headed mode
```

### Code Standards

**TypeScript Configuration**
- Strict mode enabled
- No implicit any
- Consistent import/export patterns
- Proper type definitions for all props and state

**ESLint Rules**
- React hooks rules
- TypeScript recommended rules
- Import order enforcement
- Unused variable detection

**Prettier Configuration**
- 2-space indentation
- Single quotes
- Trailing commas
- Line length: 100 characters

### Git Workflow

1. **Feature Branches**
```bash
git checkout -b feature/graph-analytics
git commit -m "feat: add centrality analysis"
git push origin feature/graph-analytics
```

2. **Commit Message Format**
```
type(scope): description

feat: new feature
fix: bug fix
docs: documentation
style: formatting
refactor: code restructuring
test: adding tests
chore: maintenance
```

3. **Pull Request Process**
- Create feature branch
- Implement changes with tests
- Update documentation
- Submit PR with description
- Address review feedback
- Merge after approval

## Deployment

### Build Process

```bash
# Production build
npm run build

# Preview build
npm run preview

# Analyze bundle
npm run analyze
```

### Environment Configuration

**Production Environment Variables**
```env
VITE_API_BASE_URL=https://api.vexfs.com
VITE_WEBSOCKET_URL=wss://ws.vexfs.com
VITE_ENVIRONMENT=production
VITE_SENTRY_DSN=your-sentry-dsn
```

**Docker Deployment**
```dockerfile
FROM node:18-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/nginx.conf
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

### Monitoring and Analytics

**Error Tracking**
```typescript
// Sentry integration
import * as Sentry from '@sentry/react';

Sentry.init({
  dsn: import.meta.env.VITE_SENTRY_DSN,
  environment: import.meta.env.VITE_ENVIRONMENT,
  integrations: [
    new Sentry.BrowserTracing(),
  ],
  tracesSampleRate: 1.0,
});
```

**Performance Monitoring**
```typescript
// Custom performance monitoring
export function usePerformanceMonitor() {
  const startMeasurement = useCallback((name: string) => {
    performance.mark(`${name}-start`);
    return name;
  }, []);
  
  const endMeasurement = useCallback((name: string) => {
    performance.mark(`${name}-end`);
    performance.measure(name, `${name}-start`, `${name}-end`);
    
    const measure = performance.getEntriesByName(name)[0];
    console.log(`${name}: ${measure.duration}ms`);
  }, []);
  
  return { startMeasurement, endMeasurement };
}
```

## Contributing

### Development Setup

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Update documentation
6. Submit a pull request

### Code Review Guidelines

- **Functionality**: Does the code work as intended?
- **Performance**: Are there any performance implications?
- **Security**: Are there any security concerns?
- **Testing**: Are there adequate tests?
- **Documentation**: Is the code well-documented?
- **Style**: Does the code follow project conventions?

### Release Process

1. **Version Bump**
```bash
npm version patch|minor|major
```

2. **Changelog Update**
- Document new features
- List bug fixes
- Note breaking changes

3. **Release Creation**
```bash
git tag v1.2.3
git push origin v1.2.3
```

4. **Deployment**
- Automated via CI/CD
- Manual verification
- Rollback plan ready

---

For questions or support, please refer to the project documentation or create an issue in the repository.