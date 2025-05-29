"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.VexFSClient = void 0;
const node_fetch_1 = __importDefault(require("node-fetch"));
class VexFSClient {
    constructor(config = {}) {
        this.baseUrl = config.baseUrl || 'http://localhost:8000';
        this.timeout = config.timeout || 30000;
        this.defaultCollection = config.defaultCollection || 'default';
    }
    /**
     * Get VexFS server version
     */
    async version() {
        const response = await this.request('GET', '/api/v1/version');
        return response.data || 'Unknown';
    }
    /**
     * List all collections
     */
    async listCollections() {
        const response = await this.request('GET', '/api/v1/collections');
        return response.data || [];
    }
    /**
     * Create a new collection
     */
    async createCollection(name, metadata) {
        const response = await this.request('POST', '/api/v1/collections', {
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
    async getCollection(name) {
        const response = await this.request('GET', `/api/v1/collections/${name}`);
        if (!response.data) {
            throw new Error(response.error || 'Collection not found');
        }
        return response.data;
    }
    /**
     * Delete a collection
     */
    async deleteCollection(name) {
        const response = await this.request('DELETE', `/api/v1/collections/${name}`);
        if (!response.success) {
            throw new Error(response.error || 'Failed to delete collection');
        }
    }
    /**
     * Add documents to a collection with automatic embedding generation
     */
    async add(text, metadata = {}, collection) {
        const collectionName = collection || this.defaultCollection;
        // Ensure collection exists
        await this.ensureCollection(collectionName);
        // Generate document ID
        const docId = `doc_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        // Generate simple embedding (in a real implementation, this would use a proper embedding model)
        const embedding = this.generateSimpleEmbedding(text);
        const response = await this.request('POST', `/api/v1/collections/${collectionName}/add`, {
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
    async query(vector, topK = 10, collection) {
        const collectionName = collection || this.defaultCollection;
        const response = await this.request('POST', `/api/v1/collections/${collectionName}/query`, {
            query_embeddings: [vector],
            n_results: topK,
            include: ['documents', 'metadatas', 'distances']
        });
        if (!response.data) {
            throw new Error(response.error || 'Query failed');
        }
        const results = [];
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
    async delete(docId, collection) {
        const collectionName = collection || this.defaultCollection;
        // Note: The current server implementation doesn't have a delete documents endpoint
        // This would need to be implemented in the server
        throw new Error('Document deletion not yet implemented in server');
    }
    /**
     * Generate a simple embedding for text (placeholder implementation)
     */
    generateSimpleEmbedding(text) {
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
    simpleHash(str) {
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
    async ensureCollection(name) {
        try {
            await this.getCollection(name);
        }
        catch (error) {
            // Collection doesn't exist, create it
            await this.createCollection(name);
        }
    }
    /**
     * Make HTTP request to VexFS server
     */
    async request(method, path, body) {
        const url = `${this.baseUrl}${path}`;
        const options = {
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
            const response = await (0, node_fetch_1.default)(url, options);
            const data = await response.json();
            if (!response.ok) {
                return {
                    success: false,
                    error: data.error || `HTTP ${response.status}: ${response.statusText}`
                };
            }
            return data;
        }
        catch (error) {
            return {
                success: false,
                error: error instanceof Error ? error.message : 'Unknown error'
            };
        }
    }
}
exports.VexFSClient = VexFSClient;
exports.default = VexFSClient;
//# sourceMappingURL=client.js.map