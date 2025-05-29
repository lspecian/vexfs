import { spawn } from 'child_process';
import { promises as fs } from 'fs';
import { join } from 'path';

export interface VexFSResult {
  id: string;
  score: number;
  metadata: Record<string, string>;
}

export interface VexFSConfig {
  mountPoint?: string;
  vexctlPath?: string;
  timeout?: number;
}

export class VexFSClient {
  private mountPoint: string;
  private vexctlPath: string;
  private timeout: number;

  constructor(config: VexFSConfig = {}) {
    this.mountPoint = config.mountPoint || '/mnt/vexfs';
    this.vexctlPath = config.vexctlPath || 'vexctl';
    this.timeout = config.timeout || 30000;
  }

  async add(text: string, metadata: Record<string, string> = {}): Promise<string> {
    // Generate unique document ID
    const docId = `doc_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    
    // Write document to filesystem
    const docPath = join(this.mountPoint, 'documents', `${docId}.txt`);
    await fs.writeFile(docPath, text);
    
    // Add metadata if provided
    if (Object.keys(metadata).length > 0) {
      const metadataPath = join(this.mountPoint, 'metadata', `${docId}.json`);
      await fs.writeFile(metadataPath, JSON.stringify(metadata));
    }
    
    // Use vexctl to index the document
    await this.executeVexctl(['index', '--file', docPath, '--id', docId]);
    
    return docId;
  }

  async query(vector: number[], topK: number = 10): Promise<VexFSResult[]> {
    // Write vector to temporary file
    const vectorPath = join(this.mountPoint, 'tmp', `query_${Date.now()}.vec`);
    await fs.writeFile(vectorPath, vector.join(','));
    
    // Execute search using vexctl
    const output = await this.executeVexctl([
      'search',
      '--vector-file', vectorPath,
      '--top-k', topK.toString(),
      '--format', 'json'
    ]);
    
    // Clean up temporary file
    await fs.unlink(vectorPath);
    
    // Parse results
    return JSON.parse(output);
  }

  async delete(id: string): Promise<void> {
    // Remove document files
    const docPath = join(this.mountPoint, 'documents', `${id}.txt`);
    const metadataPath = join(this.mountPoint, 'metadata', `${id}.json`);
    
    try {
      await fs.unlink(docPath);
    } catch (error) {
      // Document file might not exist
    }
    
    try {
      await fs.unlink(metadataPath);
    } catch (error) {
      // Metadata file might not exist
    }
    
    // Remove from index using vexctl
    await this.executeVexctl(['delete', '--id', id]);
  }

  private async executeVexctl(args: string[]): Promise<string> {
    return new Promise((resolve, reject) => {
      const process = spawn(this.vexctlPath, args, {
        cwd: this.mountPoint,
        timeout: this.timeout
      });

      let stdout = '';
      let stderr = '';

      process.stdout.on('data', (data) => {
        stdout += data.toString();
      });

      process.stderr.on('data', (data) => {
        stderr += data.toString();
      });

      process.on('close', (code) => {
        if (code === 0) {
          resolve(stdout.trim());
        } else {
          reject(new Error(`vexctl failed with code ${code}: ${stderr}`));
        }
      });

      process.on('error', (error) => {
        reject(new Error(`Failed to execute vexctl: ${error.message}`));
      });
    });
  }
}

export default VexFSClient;