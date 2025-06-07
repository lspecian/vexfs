import { http, HttpResponse } from 'msw';
import { setupServer } from 'msw/node';
import type { VexFSCollection, VexFSPoint, VexFSSearchResult } from '../../src/types';

// Mock data
const mockCollections: VexFSCollection[] = [
  {
    id: 'collection-1',
    name: 'Test Collection 1',
    description: 'A test collection for development',
    vectorSize: 384,
    distance: 'cosine',
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-01-01T00:00:00Z',
    pointsCount: 1000,
  },
  {
    id: 'collection-2',
    name: 'Test Collection 2',
    description: 'Another test collection',
    vectorSize: 768,
    distance: 'euclidean',
    createdAt: '2025-01-02T00:00:00Z',
    updatedAt: '2025-01-02T00:00:00Z',
    pointsCount: 500,
  },
  {
    id: 'collection-3',
    name: 'Large Collection',
    description: 'A large collection for testing pagination',
    vectorSize: 1536,
    distance: 'dot',
    createdAt: '2025-01-03T00:00:00Z',
    updatedAt: '2025-01-03T00:00:00Z',
    pointsCount: 10000,
  },
];

const mockPoints: Record<string, VexFSPoint[]> = {
  'collection-1': [
    {
      id: 'point-1',
      vector: Array.from({ length: 384 }, () => Math.random()),
      payload: { text: 'Sample text 1', category: 'test' },
    },
    {
      id: 'point-2',
      vector: Array.from({ length: 384 }, () => Math.random()),
      payload: { text: 'Sample text 2', category: 'demo' },
    },
  ],
  'collection-2': [
    {
      id: 'point-3',
      vector: Array.from({ length: 768 }, () => Math.random()),
      payload: { text: 'Sample text 3', category: 'test' },
    },
  ],
  'collection-3': Array.from({ length: 100 }, (_, i) => ({
    id: `point-${i + 4}`,
    vector: Array.from({ length: 1536 }, () => Math.random()),
    payload: { text: `Sample text ${i + 4}`, category: i % 2 === 0 ? 'test' : 'demo' },
  })),
};

// API handlers
export const handlers = [
  // Health check
  http.get('/health', () => {
    return HttpResponse.json({
      status: 'healthy',
      timestamp: Date.now(),
      collections_count: mockCollections.length,
      dialects: [
        { name: 'ChromaDB', status: 'active', endpoint: '/api/v1' },
        { name: 'Qdrant', status: 'active', endpoint: '/collections' },
        { name: 'Native VexFS', status: 'active', endpoint: '/vexfs/v1' },
      ],
    });
  }),

  // ChromaDB API - Collections
  http.get('/api/v1/collections', () => {
    return HttpResponse.json({ collections: mockCollections });
  }),

  http.post('/api/v1/collections', async ({ request }) => {
    const body = await request.json() as Partial<VexFSCollection>;
    const newCollection: VexFSCollection = {
      id: `collection-${Date.now()}`,
      name: body.name || 'New Collection',
      description: body.description,
      vectorSize: body.vectorSize || 384,
      distance: body.distance || 'cosine',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      pointsCount: 0,
    };
    mockCollections.push(newCollection);
    return HttpResponse.json(newCollection, { status: 201 });
  }),

  http.get('/api/v1/collections/:id', ({ params }) => {
    const collection = mockCollections.find(c => c.id === params.id);
    if (!collection) {
      return HttpResponse.json({ error: 'Collection not found' }, { status: 404 });
    }
    return HttpResponse.json(collection);
  }),

  http.put('/api/v1/collections/:id', async ({ params, request }) => {
    const collectionIndex = mockCollections.findIndex(c => c.id === params.id);
    if (collectionIndex === -1) {
      return HttpResponse.json({ error: 'Collection not found' }, { status: 404 });
    }
    
    const body = await request.json() as Partial<VexFSCollection>;
    mockCollections[collectionIndex] = {
      ...mockCollections[collectionIndex],
      ...body,
      updatedAt: new Date().toISOString(),
    };
    
    return HttpResponse.json(mockCollections[collectionIndex]);
  }),

  http.delete('/api/v1/collections/:id', ({ params }) => {
    const collectionIndex = mockCollections.findIndex(c => c.id === params.id);
    if (collectionIndex === -1) {
      return HttpResponse.json({ error: 'Collection not found' }, { status: 404 });
    }
    
    mockCollections.splice(collectionIndex, 1);
    delete mockPoints[params.id as string];
    return HttpResponse.json({ success: true });
  }),

  // Points/Vectors
  http.get('/api/v1/collections/:id/points', ({ params, request }) => {
    const url = new URL(request.url);
    const limit = parseInt(url.searchParams.get('limit') || '10');
    const offset = parseInt(url.searchParams.get('offset') || '0');
    
    const points = mockPoints[params.id as string] || [];
    const paginatedPoints = points.slice(offset, offset + limit);
    
    return HttpResponse.json({
      points: paginatedPoints,
      total: points.length,
      limit,
      offset,
    });
  }),

  http.post('/api/v1/collections/:id/add', async ({ params, request }) => {
    const body = await request.json() as { points: VexFSPoint[] };
    const collectionId = params.id as string;
    
    if (!mockPoints[collectionId]) {
      mockPoints[collectionId] = [];
    }
    
    mockPoints[collectionId].push(...body.points);
    
    // Update collection points count
    const collection = mockCollections.find(c => c.id === collectionId);
    if (collection) {
      collection.pointsCount = mockPoints[collectionId].length;
      collection.updatedAt = new Date().toISOString();
    }
    
    return HttpResponse.json({ success: true, added: body.points.length });
  }),

  http.post('/api/v1/collections/:id/query', async ({ params, request }) => {
    const body = await request.json() as {
      query_embeddings: number[][];
      n_results?: number;
      where?: Record<string, unknown>;
    };
    
    const collectionId = params.id as string;
    const points = mockPoints[collectionId] || [];
    const limit = body.n_results || 10;
    
    // Mock similarity search - just return random points with scores
    const results: VexFSSearchResult[] = points
      .slice(0, limit)
      .map((point, index) => ({
        id: point.id,
        score: Math.random() * 0.5 + 0.5, // Random score between 0.5 and 1
        payload: point.payload,
        vector: point.vector,
      }))
      .sort((a, b) => b.score - a.score);
    
    return HttpResponse.json({
      ids: [results.map(r => r.id)],
      distances: [results.map(r => 1 - r.score)],
      metadatas: [results.map(r => r.payload)],
      embeddings: [results.map(r => r.vector)],
    });
  }),

  // Qdrant API compatibility
  http.get('/collections', () => {
    return HttpResponse.json({
      result: {
        collections: mockCollections.map(c => ({
          name: c.name,
          vectors_count: c.pointsCount,
          config: {
            params: {
              vectors: {
                size: c.vectorSize,
                distance: c.distance.toUpperCase(),
              },
            },
          },
        })),
      },
    });
  }),

  // Native VexFS API
  http.get('/vexfs/v1/collections', () => {
    return HttpResponse.json({ collections: mockCollections });
  }),

  http.get('/vexfs/v1/health', () => {
    return HttpResponse.json({
      status: 'healthy',
      version: '1.0.0',
      uptime: 3600,
      collections: mockCollections.length,
      total_points: Object.values(mockPoints).reduce((sum, points) => sum + points.length, 0),
    });
  }),

  // Metrics endpoint
  http.get('/metrics', () => {
    return HttpResponse.text(`
# HELP vexfs_collections_total Total number of collections
# TYPE vexfs_collections_total counter
vexfs_collections_total ${mockCollections.length}

# HELP vexfs_points_total Total number of points across all collections
# TYPE vexfs_points_total counter
vexfs_points_total ${Object.values(mockPoints).reduce((sum, points) => sum + points.length, 0)}

# HELP vexfs_memory_usage_bytes Memory usage in bytes
# TYPE vexfs_memory_usage_bytes gauge
vexfs_memory_usage_bytes 1048576

# HELP vexfs_requests_total Total number of requests
# TYPE vexfs_requests_total counter
vexfs_requests_total 1000
    `.trim());
  }),
];

// Create and export the server
export const server = setupServer(...handlers);

// Helper functions for tests
export const resetMockData = () => {
  // Reset to initial state
  mockCollections.length = 0;
  mockCollections.push(
    {
      id: 'collection-1',
      name: 'Test Collection 1',
      description: 'A test collection for development',
      vectorSize: 384,
      distance: 'cosine',
      createdAt: '2025-01-01T00:00:00Z',
      updatedAt: '2025-01-01T00:00:00Z',
      pointsCount: 1000,
    },
    {
      id: 'collection-2',
      name: 'Test Collection 2',
      description: 'Another test collection',
      vectorSize: 768,
      distance: 'euclidean',
      createdAt: '2025-01-02T00:00:00Z',
      updatedAt: '2025-01-02T00:00:00Z',
      pointsCount: 500,
    },
    {
      id: 'collection-3',
      name: 'Large Collection',
      description: 'A large collection for testing pagination',
      vectorSize: 1536,
      distance: 'dot',
      createdAt: '2025-01-03T00:00:00Z',
      updatedAt: '2025-01-03T00:00:00Z',
      pointsCount: 10000,
    }
  );
};

export const addMockCollection = (collection: VexFSCollection) => {
  mockCollections.push(collection);
};

export const getMockCollections = () => [...mockCollections];
export const getMockPoints = (collectionId: string) => mockPoints[collectionId] || [];