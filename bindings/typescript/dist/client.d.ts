export interface VexFSResult {
    id: string;
    score: number;
    metadata: Record<string, string>;
}
export interface VexFSConfig {
    baseUrl?: string;
    timeout?: number;
}
export declare class VexFSClient {
    private baseUrl;
    private timeout;
    constructor(config?: VexFSConfig);
    add(text: string, metadata?: Record<string, string>): Promise<string>;
    query(vector: number[], topK?: number): Promise<VexFSResult[]>;
    delete(id: string): Promise<void>;
}
export default VexFSClient;
//# sourceMappingURL=client.d.ts.map