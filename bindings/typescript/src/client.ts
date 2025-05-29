export interface VexFSResult {
  id: string;
  score: number;
  metadata: Record<string, string>;
}

export interface VexFSConfig {
  baseUrl?: string;
  timeout?: number;
}

export class VexFSClient {
  private baseUrl: string;
  private timeout: number;

  constructor(config: VexFSConfig = {}) {
    this.baseUrl = config.baseUrl || 'http://localhost:8080';
    this.timeout = config.timeout || 30000;
  }

  async add(text: string, metadata: Record<string, string> = {}): Promise<string> {
    // TODO: Implement REST API call
    throw new Error('Not implemented yet');
  }

  async query(vector: number[], topK: number = 10): Promise<VexFSResult[]> {
    // TODO: Implement REST API call
    throw new Error('Not implemented yet');
  }

  async delete(id: string): Promise<void> {
    // TODO: Implement REST API call
    throw new Error('Not implemented yet');
  }
}

export default VexFSClient;