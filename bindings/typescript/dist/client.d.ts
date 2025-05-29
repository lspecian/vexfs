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
export declare class VexFSClient {
    private baseUrl;
    private timeout;
    private defaultCollection;
    constructor(config?: VexFSConfig);
    /**
     * Get VexFS server version
     */
    version(): Promise<string>;
    /**
     * List all collections
     */
    listCollections(): Promise<Collection[]>;
    /**
     * Create a new collection
     */
    createCollection(name: string, metadata?: Record<string, any>): Promise<Collection>;
    /**
     * Get a collection by name
     */
    getCollection(name: string): Promise<Collection>;
    /**
     * Delete a collection
     */
    deleteCollection(name: string): Promise<void>;
    /**
     * Add documents to a collection with automatic embedding generation
     */
    add(text: string, metadata?: Record<string, any>, collection?: string): Promise<string>;
    /**
     * Query for similar documents using a vector
     */
    query(vector: number[], topK?: number, collection?: string): Promise<VexFSResult[]>;
    /**
     * Delete a document by ID
     */
    delete(docId: string, collection?: string): Promise<void>;
    /**
     * Generate a simple embedding for text (placeholder implementation)
     */
    private generateSimpleEmbedding;
    /**
     * Simple hash function for strings
     */
    private simpleHash;
    /**
     * Ensure a collection exists, create it if it doesn't
     */
    private ensureCollection;
    /**
     * Make HTTP request to VexFS server
     */
    private request;
}
export default VexFSClient;
//# sourceMappingURL=client.d.ts.map