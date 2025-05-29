import fetch from 'node-fetch';

export interface VexFSResult {
  id: string;
  score: number;
  metadata: Record<string, any>;
  document?: string;
}

export interface VexFSConfig {
  baseUrl?: string;
  timeout?: number;
  defaultCollection?: string;
}

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

export interface Collection {
  id: string;
  name: string;
  metadata?: Record<string, any>;
}

export interface QueryResponse {
  ids: string[][];
  distances?: number[][];
  metadatas?: Record<string, any>[][];
  documents?: string[][];
}

export class VexFSClient {
  private baseUrl: string;
  private timeout: number;
  private defaultCollection: string;

  constructor(config: VexFSConfig = {}) {
    this.baseUrl = config.baseUrl || 'http://localhost:8000';
    this.timeout = config.timeout || 30000;
    this.defaultCollection = config.defaultCollection || 'default';
  }

  /**
   * Get VexFS server version
   */
  async version(): Promise<string> {
    const response = await this.request<string>('GET', '/api/v1/version');
    return response.data || 'Unknown';
  }

  /**
   * List all collections
   */
  async listCollections(): Promise<Collection[]> {
    const response = await this.request<Collection[]>('GET', '/api/v1/collections');
    return response.data || [];
  }

  /**
   * Create a new collection
   */
  async createCollection(name: string, metadata?: Record<string, any>): Promise<Collection> {
    const response = await this.request<Collection>('POST', '/api/v1/collections', {
      name,
      metadata
    });
    
    if (!response.data) {
      throw new Error(response.error || 'Failed to create collection');
    }
    
    return response.data;
  }

  /**
   * Get a collection by name
   */
  async getCollection(name: string): Promise<Collection> {
    const response = await this.request<Collection>('GET', `/api/v1/collections/${name}`);
    
    if (!response.data) {
      throw new Error(response.error || 'Collection not found');
    }
    
    return response.data;
  }

  /**
   * Delete a collection
   */
  async deleteCollection(name: string): Promise<void> {
    const response = await this.request<string>('DELETE', `/api/v1/collections/${name}`);
    
    if (!response.success) {
      throw new Error(response.error || 'Failed to delete collection');
    }
  }

  /**
   * Add documents to a collection with automatic embedding generation
   */
  async add(
    text: string,
    metadata: Record<string, any> = {},
    collection?: string
  ): Promise<string> {
    const collectionName = collection || this.defaultCollection;
    
    // Ensure collection exists
    await this.ensureCollection(collectionName);
    
    // Generate document ID
    const docId = `doc_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    
    // Generate simple embedding (in a real implementation, this would use a proper embedding model)
    const embedding = this.generateSimpleEmbedding(text);
    
    const response = await this.request<string>('POST', `/api/v1/collections/${collectionName}/add`, {
      ids: [docId],
      embeddings: [embedding],
      documents: [text],
      metadatas: [metadata]
    });
    
    if (!response.success) {
      throw new Error(response.error || 'Failed to add document');
    }
    
    return docId;
  }

  /**
   * Query for similar documents using a vector
   */
  async query(
    vector: number[],
    topK: number = 10,
    collection?: string
  ): Promise<VexFSResult[]> {
    const collectionName = collection || this.defaultCollection;
    
    const response = await this.request<QueryResponse>('POST', `/api/v1/collections/${collectionName}/query`, {
      query_embeddings: [vector],
      n_results: topK,
      include: ['documents', 'metadatas', 'distances']
    });
    
    if (!response.data) {
      throw new Error(response.error || 'Query failed');
    }
    
    const results: VexFSResult[] = [];
    const queryData = response.data;
    
    if (queryData.ids && queryData.ids[0]) {
      for (let i = 0; i < queryData.ids[0].length; i++) {
        results.push({
          id: queryData.ids[0][i],
          score: queryData.distances?.[0]?.[i] || 0,
          metadata: queryData.metadatas?.[0]?.[i] || {},
          document: queryData.documents?.[0]?.[i]
        });
      }
    }
    
    return results;
  }

  /**
   * Delete a document by ID
   */
  async delete(docId: string, collection?: string): Promise<void> {
    const collectionName = collection || this.defaultCollection;
    
    // Note: The current server implementation doesn't have a delete documents endpoint
    // This would need to be implemented in the server
    throw new Error('Document deletion not yet implemented in server');
  }

  /**
   * Generate a simple embedding for text (placeholder implementation)
   */
  private generateSimpleEmbedding(text: string): number[] {
    const words = text.toLowerCase().split(/\s+/);
    const embedding = new Array(384).fill(0);
    
    // Simple hash-based embedding
    for (let i = 0; i < words.length; i++) {
      const word = words[i];
      const hash = this.simpleHash(word);
      const idx = hash % embedding.length;
      embedding[idx] += 1.0 / (i + 1);
    }
    
    // Normalize the embedding
    const magnitude = Math.sqrt(embedding.reduce((sum, val) => sum + val * val, 0));
    if (magnitude > 0) {
      for (let i = 0; i < embedding.length; i++) {
        embedding[i] /= magnitude;
      }
    }
    
    return embedding;
  }

  /**
   * Simple hash function for strings
   */
  private simpleHash(str: string): number {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash; // Convert to 32-bit integer
    }
    return Math.abs(hash);
  }

  /**
   * Ensure a collection exists, create it if it doesn't
   */
  private async ensureCollection(name: string): Promise<void> {
    try {
      await this.getCollection(name);
    } catch (error) {
      // Collection doesn't exist, create it
      await this.createCollection(name);
    }
  }

  /**
   * Make HTTP request to VexFS server
   */
  private async request<T>(
    method: string,
    path: string,
    body?: any
  ): Promise<ApiResponse<T>> {
    const url = `${this.baseUrl}${path}`;
    const options: any = {
      method,
      headers: {
        'Content-Type': 'application/json',
      },
      timeout: this.timeout,
    };

    if (body) {
      options.body = JSON.stringify(body);
    }

    try {
      const response = await fetch(url, options);
      const data = await response.json() as ApiResponse<T>;
      
      if (!response.ok) {
        return {
          success: false,
          error: data.error || `HTTP ${response.status}: ${response.statusText}`
        };
      }
      
      return data;
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error'
      };
    }
  }
}

export default VexFSClient;