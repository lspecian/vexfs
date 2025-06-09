import { test, expect } from '@playwright/test';
import { http, HttpResponse } from 'msw';
import { server } from './setup/mock-server';
import type { 
  NodeResponse, 
  EdgeResponse, 
  CreateNodeRequest, 
  CreateEdgeRequest,
  TraversalResult,
  GraphStatistics,
  GraphSearchResult,
  GraphAnalytics,
  GraphSchema
} from '../src/types/graph';

// Mock VexGraph data
const mockNodes: NodeResponse[] = [
  {
    id: 'node-1',
    inode_number: 12345,
    node_type: 'File',
    properties: { name: 'String', size: 'Integer' },
    outgoing_edges: ['edge-1', 'edge-2'],
    incoming_edges: ['edge-3'],
    created_at: '2025-01-01T00:00:00Z',
    updated_at: '2025-01-01T00:00:00Z',
  },
  {
    id: 'node-2',
    inode_number: 12346,
    node_type: 'Directory',
    properties: { name: 'String', permissions: 'String' },
    outgoing_edges: ['edge-3'],
    incoming_edges: ['edge-1'],
    created_at: '2025-01-01T00:00:00Z',
    updated_at: '2025-01-01T00:00:00Z',
  },
  {
    id: 'node-3',
    inode_number: 12347,
    node_type: 'File',
    properties: { name: 'String', size: 'Integer' },
    outgoing_edges: [],
    incoming_edges: ['edge-2'],
    created_at: '2025-01-01T00:00:00Z',
    updated_at: '2025-01-01T00:00:00Z',
  },
];

const mockEdges: EdgeResponse[] = [
  {
    id: 'edge-1',
    source_id: 'node-1',
    target_id: 'node-2',
    edge_type: 'Contains',
    weight: 1.0,
    properties: { relationship: 'String' },
    created_at: '2025-01-01T00:00:00Z',
    updated_at: '2025-01-01T00:00:00Z',
  },
  {
    id: 'edge-2',
    source_id: 'node-1',
    target_id: 'node-3',
    edge_type: 'References',
    weight: 0.8,
    properties: { type: 'String' },
    created_at: '2025-01-01T00:00:00Z',
    updated_at: '2025-01-01T00:00:00Z',
  },
  {
    id: 'edge-3',
    source_id: 'node-2',
    target_id: 'node-1',
    edge_type: 'DependsOn',
    weight: 0.9,
    properties: { dependency_type: 'String' },
    created_at: '2025-01-01T00:00:00Z',
    updated_at: '2025-01-01T00:00:00Z',
  },
];

const mockGraphStats: GraphStatistics = {
  node_count: 3,
  edge_count: 3,
  node_types: { File: 2, Directory: 1, Symlink: 0, Device: 0, Custom: 0 },
  edge_types: { Contains: 1, References: 1, DependsOn: 1, SimilarTo: 0, Custom: 0 },
  average_degree: 2.0,
  density: 0.5,
  connected_components: 1,
  largest_component_size: 3,
  clustering_coefficient: 0.33,
  diameter: 2,
};

const mockGraphAnalytics: GraphAnalytics = {
  degree_distribution: [
    { degree: 1, count: 1 },
    { degree: 2, count: 2 },
  ],
  centrality_measures: {
    betweenness: { 'node-1': 0.5, 'node-2': 0.3, 'node-3': 0.2 },
    closeness: { 'node-1': 0.8, 'node-2': 0.6, 'node-3': 0.4 },
    eigenvector: { 'node-1': 0.7, 'node-2': 0.5, 'node-3': 0.3 },
    pagerank: { 'node-1': 0.4, 'node-2': 0.35, 'node-3': 0.25 },
  },
  clustering_coefficients: { 'node-1': 0.5, 'node-2': 0.3, 'node-3': 0.0 },
  shortest_paths_stats: {
    average_path_length: 1.5,
    diameter: 2,
    radius: 1,
  },
  community_detection: {
    communities: [['node-1', 'node-2'], ['node-3']],
    modularity: 0.25,
  },
};

const mockGraphSchema: GraphSchema = {
  node_types: [
    {
      type: 'File',
      required_properties: ['name'],
      optional_properties: ['size', 'permissions'],
      property_types: { name: 'String', size: 'Integer', permissions: 'String' },
    },
    {
      type: 'Directory',
      required_properties: ['name'],
      optional_properties: ['permissions'],
      property_types: { name: 'String', permissions: 'String' },
    },
  ],
  edge_types: [
    {
      type: 'Contains',
      allowed_source_types: ['Directory'],
      allowed_target_types: ['File', 'Directory'],
      required_properties: [],
      optional_properties: ['relationship'],
      property_types: { relationship: 'String' },
    },
  ],
  version: '1.0.0',
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
};

// VexGraph API handlers
const vexgraphHandlers = [
  // Node CRUD operations
  http.get('/api/v1/vexgraph/nodes', ({ request }) => {
    const url = new URL(request.url);
    const limit = parseInt(url.searchParams.get('limit') || '100');
    const offset = parseInt(url.searchParams.get('offset') || '0');
    
    const paginatedNodes = mockNodes.slice(offset, offset + limit);
    
    return HttpResponse.json({
      success: true,
      data: {
        items: paginatedNodes,
        total: mockNodes.length,
        page: Math.floor(offset / limit) + 1,
        pageSize: limit,
        hasNext: offset + limit < mockNodes.length,
        hasPrev: offset > 0,
      },
    });
  }),

  http.get('/api/v1/vexgraph/nodes/:nodeId', ({ params }) => {
    const node = mockNodes.find(n => n.id === params.nodeId);
    if (!node) {
      return HttpResponse.json({ success: false, error: 'Node not found' }, { status: 404 });
    }
    return HttpResponse.json({ success: true, data: node });
  }),

  http.post('/api/v1/vexgraph/nodes', async ({ request }) => {
    const body = await request.json() as CreateNodeRequest;
    const newNode: NodeResponse = {
      id: `node-${Date.now()}`,
      inode_number: body.inode_number,
      node_type: body.node_type,
      properties: body.properties || {},
      outgoing_edges: [],
      incoming_edges: [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
    mockNodes.push(newNode);
    return HttpResponse.json({ success: true, data: newNode }, { status: 201 });
  }),

  http.patch('/api/v1/vexgraph/nodes/:nodeId', async ({ params, request }) => {
    const nodeIndex = mockNodes.findIndex(n => n.id === params.nodeId);
    if (nodeIndex === -1) {
      return HttpResponse.json({ success: false, error: 'Node not found' }, { status: 404 });
    }
    
    const body = await request.json() as { properties: Record<string, any> };
    mockNodes[nodeIndex] = {
      ...mockNodes[nodeIndex],
      properties: { ...mockNodes[nodeIndex].properties, ...body.properties },
      updated_at: new Date().toISOString(),
    };
    
    return HttpResponse.json({ success: true, data: mockNodes[nodeIndex] });
  }),

  http.delete('/api/v1/vexgraph/nodes/:nodeId', ({ params }) => {
    const nodeIndex = mockNodes.findIndex(n => n.id === params.nodeId);
    if (nodeIndex === -1) {
      return HttpResponse.json({ success: false, error: 'Node not found' }, { status: 404 });
    }
    
    mockNodes.splice(nodeIndex, 1);
    return HttpResponse.json({ success: true });
  }),

  // Edge CRUD operations
  http.get('/api/v1/vexgraph/edges', ({ request }) => {
    const url = new URL(request.url);
    const limit = parseInt(url.searchParams.get('limit') || '100');
    const offset = parseInt(url.searchParams.get('offset') || '0');
    
    const paginatedEdges = mockEdges.slice(offset, offset + limit);
    
    return HttpResponse.json({
      success: true,
      data: {
        items: paginatedEdges,
        total: mockEdges.length,
        page: Math.floor(offset / limit) + 1,
        pageSize: limit,
        hasNext: offset + limit < mockEdges.length,
        hasPrev: offset > 0,
      },
    });
  }),

  http.get('/api/v1/vexgraph/edges/:edgeId', ({ params }) => {
    const edge = mockEdges.find(e => e.id === params.edgeId);
    if (!edge) {
      return HttpResponse.json({ success: false, error: 'Edge not found' }, { status: 404 });
    }
    return HttpResponse.json({ success: true, data: edge });
  }),

  http.post('/api/v1/vexgraph/edges', async ({ request }) => {
    const body = await request.json() as CreateEdgeRequest;
    const newEdge: EdgeResponse = {
      id: `edge-${Date.now()}`,
      source_id: body.source_id,
      target_id: body.target_id,
      edge_type: body.edge_type,
      weight: body.weight || 1.0,
      properties: body.properties || {},
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
    mockEdges.push(newEdge);
    return HttpResponse.json({ success: true, data: newEdge }, { status: 201 });
  }),

  http.delete('/api/v1/vexgraph/edges/:edgeId', ({ params }) => {
    const edgeIndex = mockEdges.findIndex(e => e.id === params.edgeId);
    if (edgeIndex === -1) {
      return HttpResponse.json({ success: false, error: 'Edge not found' }, { status: 404 });
    }
    
    mockEdges.splice(edgeIndex, 1);
    return HttpResponse.json({ success: true });
  }),

  // Traversal operations
  http.post('/api/v1/vexgraph/traversal', async ({ request }) => {
    const body = await request.json() as any;
    const mockTraversalResult: TraversalResult = {
      algorithm: body.algorithm,
      start_node: body.start_node,
      end_node: body.end_node,
      path: body.end_node ? [body.start_node, body.end_node] : undefined,
      visited_nodes: [body.start_node, 'node-2', 'node-3'],
      traversed_edges: ['edge-1', 'edge-2'],
      total_weight: 1.8,
      execution_time_ms: 15,
      success: true,
    };
    return HttpResponse.json({ success: true, data: mockTraversalResult });
  }),

  http.get('/api/v1/vexgraph/nodes/:nodeId/neighbors', ({ params }) => {
    const nodeId = params.nodeId as string;
    const neighbors = mockNodes.filter(n => n.id !== nodeId).slice(0, 2);
    return HttpResponse.json({ success: true, data: neighbors });
  }),

  // Search operations
  http.post('/api/v1/vexgraph/search', async ({ request }) => {
    const body = await request.json() as any;
    const mockSearchResult: GraphSearchResult = {
      nodes: mockNodes.slice(0, body.max_results || 10),
      edges: mockEdges.slice(0, body.max_results || 10),
      relevance_scores: {
        'node-1': 0.95,
        'node-2': 0.87,
        'edge-1': 0.92,
      },
      execution_time_ms: 25,
      total_results: mockNodes.length + mockEdges.length,
    };
    return HttpResponse.json({ success: true, data: mockSearchResult });
  }),

  // Statistics operations
  http.get('/api/v1/vexgraph/stats', () => {
    return HttpResponse.json({ success: true, data: mockGraphStats });
  }),

  http.get('/api/v1/vexgraph/nodes/:nodeId/stats', ({ params }) => {
    const nodeStats = {
      id: params.nodeId,
      degree: 2,
      in_degree: 1,
      out_degree: 1,
      clustering_coefficient: 0.5,
      betweenness_centrality: 0.3,
      closeness_centrality: 0.7,
    };
    return HttpResponse.json({ success: true, data: nodeStats });
  }),

  http.get('/api/v1/vexgraph/edges/:edgeId/stats', ({ params }) => {
    const edgeStats = {
      id: params.edgeId,
      weight: 1.0,
      betweenness: 0.2,
      usage_frequency: 15,
    };
    return HttpResponse.json({ success: true, data: edgeStats });
  }),

  http.get('/api/v1/vexgraph/analytics', () => {
    return HttpResponse.json({ success: true, data: mockGraphAnalytics });
  }),

  // Schema operations
  http.get('/api/v1/vexgraph/schema', () => {
    return HttpResponse.json({ success: true, data: mockGraphSchema });
  }),

  http.patch('/api/v1/vexgraph/schema', async ({ request }) => {
    const body = await request.json() as Partial<GraphSchema>;
    const updatedSchema = { ...mockGraphSchema, ...body, updated_at: new Date().toISOString() };
    return HttpResponse.json({ success: true, data: updatedSchema });
  }),

  // Health and version
  http.get('/api/v1/vexgraph/health', () => {
    return HttpResponse.json({ success: true });
  }),

  http.get('/api/v1/vexgraph/version', () => {
    return HttpResponse.json({ success: true, data: { version: '1.0.0' } });
  }),

  // Batch operations
  http.post('/api/v1/vexgraph/nodes/batch', async ({ request }) => {
    const body = await request.json() as { nodes: CreateNodeRequest[] };
    const createdNodes = body.nodes.map((nodeData, index) => ({
      id: `batch-node-${Date.now()}-${index}`,
      inode_number: nodeData.inode_number,
      node_type: nodeData.node_type,
      properties: nodeData.properties || {},
      outgoing_edges: [],
      incoming_edges: [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    }));
    return HttpResponse.json({ success: true, data: createdNodes });
  }),

  http.post('/api/v1/vexgraph/edges/batch', async ({ request }) => {
    const body = await request.json() as { edges: CreateEdgeRequest[] };
    const createdEdges = body.edges.map((edgeData, index) => ({
      id: `batch-edge-${Date.now()}-${index}`,
      source_id: edgeData.source_id,
      target_id: edgeData.target_id,
      edge_type: edgeData.edge_type,
      weight: edgeData.weight || 1.0,
      properties: edgeData.properties || {},
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    }));
    return HttpResponse.json({ success: true, data: createdEdges });
  }),

  http.delete('/api/v1/vexgraph/nodes/batch', async ({ request }) => {
    const body = await request.json() as { node_ids: string[] };
    // Mock deletion - just return success
    return HttpResponse.json({ success: true, deleted: body.node_ids.length });
  }),

  http.delete('/api/v1/vexgraph/edges/batch', async ({ request }) => {
    const body = await request.json() as { edge_ids: string[] };
    // Mock deletion - just return success
    return HttpResponse.json({ success: true, deleted: body.edge_ids.length });
  }),
];

test.describe('VexGraph API Service', () => {
  test.beforeAll(async () => {
    // Add VexGraph handlers to the existing server
    server.use(...vexgraphHandlers);
  });

  test.beforeEach(async ({ page }) => {
    // Navigate to a page that uses the VexGraph API
    await page.goto('/');
  });

  test.describe('Node CRUD Operations', () => {
    test('should create a new node', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.createNode({
          inode_number: 99999,
          node_type: 'File',
          properties: { name: 'String', size: 'Integer' },
        });
      });

      expect(result.inode_number).toBe(99999);
      expect(result.node_type).toBe('File');
      expect(result.properties.name).toBe('test-file.txt');
      expect(result.id).toMatch(/^node-\d+$/);
    });

    test('should get a node by ID', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.getNode('node-1');
      });

      expect(result).not.toBeNull();
      expect(result!.id).toBe('node-1');
      expect(result!.node_type).toBe('File');
      expect(result!.properties.name).toBe('test.txt');
    });

    test('should return null for non-existent node', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.getNode('non-existent-node');
      });

      expect(result).toBeNull();
    });

    test('should update a node', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.updateNode('node-1', {
          properties: { name: 'String', size: 'Integer' },
        });
      });

      expect(result.id).toBe('node-1');
      expect(result.properties.name).toBe('updated-test.txt');
      expect(result.properties.size).toBe(4096);
    });

    test('should delete a node', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.deleteNode('node-1');
      });

      expect(result).toBe(true);
    });

    test('should list nodes with pagination', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.listNodes(undefined, 2, 0);
      });

      expect(result.items).toHaveLength(2);
      expect(result.total).toBe(3);
      expect(result.page).toBe(1);
      expect(result.pageSize).toBe(2);
      expect(result.hasNext).toBe(true);
      expect(result.hasPrev).toBe(false);
    });
  });

  test.describe('Edge CRUD Operations', () => {
    test('should create a new edge', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.createEdge({
          source_id: 'node-1',
          target_id: 'node-2',
          edge_type: 'SimilarTo',
          weight: 0.95,
          properties: { similarity_score: 'Float' },
        });
      });

      expect(result.source_id).toBe('node-1');
      expect(result.target_id).toBe('node-2');
      expect(result.edge_type).toBe('SimilarTo');
      expect(result.weight).toBe(0.95);
      expect(result.id).toMatch(/^edge-\d+$/);
    });

    test('should get an edge by ID', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.getEdge('edge-1');
      });

      expect(result).not.toBeNull();
      expect(result!.id).toBe('edge-1');
      expect(result!.edge_type).toBe('Contains');
      expect(result!.source_id).toBe('node-1');
      expect(result!.target_id).toBe('node-2');
    });

    test('should delete an edge', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.deleteEdge('edge-1');
      });

      expect(result).toBe(true);
    });

    test('should list edges with pagination', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.listEdges(undefined, 2, 0);
      });

      expect(result.items).toHaveLength(2);
      expect(result.total).toBe(3);
      expect(result.hasNext).toBe(true);
    });
  });

  test.describe('Graph Traversal Operations', () => {
    test('should execute traversal query', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.executeTraversal({
          algorithm: 'BreadthFirstSearch',
          start_node: 'node-1',
          max_depth: 3,
          max_results: 10,
        });
      });

      expect(result.algorithm).toBe('BreadthFirstSearch');
      expect(result.start_node).toBe('node-1');
      expect(result.visited_nodes).toContain('node-1');
      expect(result.success).toBe(true);
      expect(result.execution_time_ms).toBeGreaterThan(0);
    });

    test('should find shortest path', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.findShortestPath('node-1', 'node-3', 5);
      });

      expect(result.start_node).toBe('node-1');
      expect(result.end_node).toBe('node-3');
      expect(result.path).toEqual(['node-1', 'node-3']);
      expect(result.success).toBe(true);
    });

    test('should perform breadth-first search', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.breadthFirstSearch('node-1', 3, 10);
      });

      expect(result.algorithm).toBe('BreadthFirstSearch');
      expect(result.start_node).toBe('node-1');
      expect(result.visited_nodes).toContain('node-1');
    });

    test('should get node neighbors', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.getNodeNeighbors('node-1', {
          direction: 'both',
          max_depth: 1,
        });
      });

      expect(Array.isArray(result)).toBe(true);
      expect(result.length).toBeGreaterThan(0);
    });
  });

  test.describe('Graph Query Operations', () => {
    test('should execute semantic search', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.semanticSearch('test files', 5);
      });

      expect(result.nodes).toBeDefined();
      expect(result.edges).toBeDefined();
      expect(result.execution_time_ms).toBeGreaterThan(0);
      expect(result.total_results).toBeGreaterThan(0);
      expect(result.relevance_scores).toBeDefined();
    });

    test('should execute property search', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.propertySearch('test', ['File'], ['Contains'], 10);
      });

      expect(result.nodes).toBeDefined();
      expect(result.edges).toBeDefined();
      expect(result.execution_time_ms).toBeGreaterThan(0);
    });

    test('should execute general query', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.executeQuery({
          query: 'find all files',
          search_type: 'property',
          node_types: ['File'],
          max_results: 20,
        });
      });

      expect(result.nodes).toBeDefined();
      expect(result.edges).toBeDefined();
      expect(result.total_results).toBeGreaterThan(0);
    });
  });

  test.describe('Graph Statistics Operations', () => {
    test('should get graph statistics', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.getGraphStats();
      });

      expect(result.node_count).toBe(3);
      expect(result.edge_count).toBe(3);
      expect(result.node_types).toBeDefined();
      expect(result.edge_types).toBeDefined();
      expect(result.average_degree).toBe(2.0);
      expect(result.density).toBe(0.5);
    });

    test('should get node statistics', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.getNodeStats('node-1');
      });

      expect(result.id).toBe('node-1');
      expect(result.degree).toBe(2);
      expect(result.in_degree).toBe(1);
      expect(result.out_degree).toBe(1);
    });

    test('should get edge statistics', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.getEdgeStats('edge-1');
      });

      expect(result.id).toBe('edge-1');
      expect(result.weight).toBe(1.0);
      expect(result.betweenness).toBeDefined();
    });

    test('should get graph analytics', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.getGraphAnalytics();
      });

      expect(result.degree_distribution).toBeDefined();
      expect(result.centrality_measures).toBeDefined();
      expect(result.centrality_measures.betweenness).toBeDefined();
      expect(result.centrality_measures.closeness).toBeDefined();
      expect(result.shortest_paths_stats).toBeDefined();
      expect(result.community_detection).toBeDefined();
    });
  });

  test.describe('Graph Schema Operations', () => {
    test('should get graph schema', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.getGraphSchema();
      });

      expect(result.node_types).toBeDefined();
      expect(result.edge_types).toBeDefined();
      expect(result.version).toBe('1.0.0');
      expect(result.node_types.length).toBeGreaterThan(0);
    });

    test('should update graph schema', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.updateGraphSchema({
          version: '1.1.0',
        });
      });

      expect(result.version).toBe('1.1.0');
      expect(result.updated_at).toBeDefined();
    });
  });

  test.describe('Utility Methods', () => {
    test('should check VexGraph health', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.checkVexGraphHealth();
      });

      expect(result).toBe(true);
    });

    test('should get VexGraph version', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.getVexGraphVersion();
      });

      expect(result).toBe('1.0.0');
    });
  });

  test.describe('Batch Operations', () => {
    test('should batch create nodes', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.batchCreateNodes([
          {
            inode_number: 11111,
            node_type: 'File',
            properties: { name: 'String', size: 'Integer' },
          },
          {
            inode_number: 11112,
            node_type: 'Directory',
            properties: { name: 'String' },
          },
        ]);
      });

      expect(Array.isArray(result)).toBe(true);
      expect(result.length).toBe(2);
      expect(result[0].node_type).toBe('File');
      expect(result[1].node_type).toBe('Directory');
    });

    test('should batch create edges', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.batchCreateEdges([
          {
            source_id: 'node-1',
            target_id: 'node-2',
            edge_type: 'Contains',
            weight: 1.0,
          },
          {
            source_id: 'node-2',
            target_id: 'node-3',
            edge_type: 'References',
            weight: 0.8,
          },
        ]);
      });

      expect(Array.isArray(result)).toBe(true);
      expect(result.length).toBe(2);
      expect(result[0].edge_type).toBe('Contains');
      expect(result[1].edge_type).toBe('References');
    });

    test('should batch delete nodes', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.batchDeleteNodes(['node-1', 'node-2']);
      });

      expect(result).toBe(true);
    });

    test('should batch delete edges', async ({ page }) => {
      const result = await page.evaluate(async () => {
        const { vexfsApi } = await import('../src/services/api');
        return await vexfsApi.batchDeleteEdges(['edge-1', 'edge-2']);
      });

      expect(result).toBe(true);
    });
  });
});