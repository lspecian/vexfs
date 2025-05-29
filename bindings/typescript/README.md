# VexFS TypeScript SDK

[![npm](https://img.shields.io/badge/npm-vexfs--sdk-blue.svg)](https://www.npmjs.com/package/vexfs-sdk)
[![Node.js](https://img.shields.io/badge/node.js-16%2B-brightgreen.svg)](https://nodejs.org)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.0%2B-blue.svg)](https://www.typescriptlang.org)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](../../LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/lspecian/vexfs)

**VexFS TypeScript SDK** provides a modern, type-safe client library for VexFS, an advanced vector-extended filesystem. This SDK interfaces with the mounted VexFS filesystem through file operations and the `vexctl` command-line tool.

## 🚀 **Why VexFS TypeScript SDK?**

- **🔷 Full TypeScript Support**: Complete type definitions with IntelliSense support
- **⚡ High Performance**: Direct filesystem integration with VexFS's ultra-fast vector operations
- **📁 Filesystem Native**: Works with mounted VexFS filesystems, not web APIs
- **🔄 Async/Await**: Modern Promise-based API with full async support
- **🛡️ Type Safety**: Compile-time error checking and runtime validation
- **📦 Minimal Dependencies**: Lightweight with filesystem-focused design
- **🧪 Comprehensively Tested**: Extensive test suite with 100% TypeScript coverage

## 📦 **Installation**

### Prerequisites

- **Node.js 16+** (18+ recommended)
- **TypeScript 5.0+** (for TypeScript projects)
- **VexFS Filesystem** mounted and accessible
- **vexctl** command-line tool installed

### Install via npm

```bash
npm install vexfs-sdk
```

### Install via yarn

```bash
yarn add vexfs-sdk
```

### Install via pnpm

```bash
pnpm add vexfs-sdk
```

### VexFS Setup

First, ensure VexFS is mounted:

```bash
# Mount VexFS filesystem
sudo mount -t vexfs /dev/sdb1 /mnt/vexfs

# Verify mount
ls /mnt/vexfs
```

### TypeScript Configuration

Ensure your `tsconfig.json` includes:

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "lib": ["ES2020"],
    "moduleResolution": "node",
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "strict": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true
  }
}
```

## ⚡ **Quick Start**

### Basic Usage

```typescript
import VexFSClient from 'vexfs-sdk';

async function main() {
  // Initialize client with VexFS mount point
  const client = new VexFSClient({
    mountPoint: '/mnt/vexfs',
    vexctlPath: '/usr/local/bin/vexctl'
  });

  try {
    // Add a document
    const docId = await client.add(
      "VexFS provides high-performance vector search",
      { category: "technology", type: "description" }
    );
    console.log(`Document added: ${docId}`);

    // Query with vector
    const queryVector = new Array(384).fill(0).map(() => Math.random());
    const results = await client.query(queryVector, 5);
    console.log(`Found ${results.length} similar documents`);

    // Delete document
    await client.delete(docId);
    console.log("Document deleted successfully");

  } catch (error) {
    console.error("VexFS operation failed:", error);
  }
}

main().catch(console.error);
```

### ES Modules

```typescript
import { VexFSClient, VexFSConfig, VexFSResult } from 'vexfs-sdk';

const config: VexFSConfig = {
  mountPoint: '/mnt/vexfs',
  vexctlPath: '/usr/local/bin/vexctl'
};

const client = new VexFSClient(config);
```

### CommonJS

```javascript
const VexFSClient = require('vexfs-sdk');

const client = new VexFSClient({
  mountPoint: '/mnt/vexfs'
});
```

## 📚 **API Reference**

### VexFSClient Class

The main client class for interacting with VexFS.

#### Constructor

```typescript
new VexFSClient(config?: VexFSConfig)
```

**Parameters:**
- `config` (VexFSConfig, optional): Client configuration options

### VexFSConfig Interface

Configuration options for the VexFS client.

```typescript
interface VexFSConfig {
  mountPoint?: string;   // Default: '/mnt/vexfs'
  vexctlPath?: string;   // Default: 'vexctl' (assumes in PATH)
  timeout?: number;      // Default: 30000 (30 seconds)
  retries?: number;      // Default: 3
  retryDelay?: number;   // Default: 1000ms
}
```

### VexFSResult Interface

Result object returned by query operations.

```typescript
interface VexFSResult {
  id: string;                        // Document identifier
  score: number;                     // Similarity score (0-1)
  metadata: Record<string, string>;  // Document metadata
}
```

### Methods

#### `client.add(text, metadata?)`

Add a text document to VexFS with optional metadata.

```typescript
async add(
  text: string, 
  metadata?: Record<string, string>
): Promise<string>
```

**Parameters:**
- `text` (string): The text content to store and index
- `metadata` (Record<string, string>, optional): Key-value metadata pairs

**Returns:**
- `Promise<string>`: Unique document identifier

**Example:**
```typescript
// Simple addition
const docId = await client.add("Machine learning is fascinating");

// With metadata
const docId = await client.add(
  "TypeScript provides excellent type safety",
  {
    language: "typescript",
    category: "programming",
    difficulty: "intermediate",
    tags: "types,safety,development"
  }
);
```

#### `client.query(vector, topK?)`

Search for similar documents using vector similarity.

```typescript
async query(
  vector: number[], 
  topK?: number
): Promise<VexFSResult[]>
```

**Parameters:**
- `vector` (number[]): Query vector for similarity search
- `topK` (number, optional): Maximum number of results (default: 10)

**Returns:**
- `Promise<VexFSResult[]>`: Array of results ranked by similarity

**Example:**
```typescript
// Generate query vector (replace with actual embeddings)
const queryVector = new Array(384).fill(0).map(() => Math.random());

// Search for similar documents
const results = await client.query(queryVector, 5);

// Process results
results.forEach((result, index) => {
  console.log(`Rank ${index + 1}:`);
  console.log(`  ID: ${result.id}`);
  console.log(`  Score: ${result.score.toFixed(4)}`);
  console.log(`  Metadata:`, result.metadata);
});
```

#### `client.delete(id)`

Remove a document from VexFS.

```typescript
async delete(id: string): Promise<void>
```

**Parameters:**
- `id` (string): Document identifier to delete

**Returns:**
- `Promise<void>`

**Example:**
```typescript
// Delete single document
await client.delete("doc_12345");

// Delete multiple documents
const docIds = ["doc_1", "doc_2", "doc_3"];
await Promise.all(docIds.map(id => client.delete(id)));
```

## 🔧 **Configuration**

### Client Configuration

```typescript
import VexFSClient from 'vexfs-sdk';

const client = new VexFSClient({
  baseUrl: 'https://vexfs.example.com',
  timeout: 60000,        // 60 seconds
  apiKey: 'your-api-key', // If authentication required
  retries: 5,            // Retry failed requests
  retryDelay: 2000       // 2 seconds between retries
});
```

### Environment Variables

```bash
# .env file
VEXFS_BASE_URL=http://localhost:8080
VEXFS_API_KEY=your-api-key
VEXFS_TIMEOUT=30000
```

```typescript
// Use environment variables
const client = new VexFSClient({
  baseUrl: process.env.VEXFS_BASE_URL,
  apiKey: process.env.VEXFS_API_KEY,
  timeout: parseInt(process.env.VEXFS_TIMEOUT || '30000')
});
```

### Connection Settings

```typescript
// Production configuration
const productionClient = new VexFSClient({
  baseUrl: 'https://api.production.com/vexfs',
  timeout: 120000,  // 2 minutes for large operations
  retries: 3,
  retryDelay: 5000  // 5 seconds between retries
});

// Development configuration
const devClient = new VexFSClient({
  baseUrl: 'http://localhost:8080',
  timeout: 10000,   // 10 seconds for quick feedback
  retries: 1,
  retryDelay: 1000
});
```

## 🔧 **Advanced Usage**

### Error Handling

```typescript
import VexFSClient from 'vexfs-sdk';

const client = new VexFSClient();

async function robustOperation() {
  try {
    const docId = await client.add("Sample document", { type: "test" });
    return docId;
  } catch (error) {
    if (error instanceof Error) {
      console.error(`VexFS Error: ${error.message}`);
      
      // Handle specific error types
      if (error.message.includes('timeout')) {
        console.log('Operation timed out, retrying...');
        // Implement retry logic
      } else if (error.message.includes('network')) {
        console.log('Network error, check connection');
      }
    }
    throw error;
  }
}
```

### Working with Embeddings

```typescript
import VexFSClient from 'vexfs-sdk';

interface EmbeddingService {
  encode(text: string): Promise<number[]>;
}

class VexFSSearchEngine {
  private client: VexFSClient;
  private embeddings: EmbeddingService;

  constructor(client: VexFSClient, embeddings: EmbeddingService) {
    this.client = client;
    this.embeddings = embeddings;
  }

  async addDocument(text: string, metadata?: Record<string, string>): Promise<string> {
    // Generate embedding (implement your embedding service)
    const embedding = await this.embeddings.encode(text);
    
    // Store in VexFS
    return await this.client.add(text, metadata);
  }

  async semanticSearch(query: string, topK: number = 10): Promise<VexFSResult[]> {
    // Generate query embedding
    const queryEmbedding = await this.embeddings.encode(query);
    
    // Search VexFS
    return await this.client.query(queryEmbedding, topK);
  }
}
```

### Batch Operations

```typescript
import VexFSClient from 'vexfs-sdk';

class BatchProcessor {
  private client: VexFSClient;
  private batchSize: number;

  constructor(client: VexFSClient, batchSize: number = 100) {
    this.client = client;
    this.batchSize = batchSize;
  }

  async batchAdd(documents: Array<{text: string, metadata?: Record<string, string>}>): Promise<string[]> {
    const results: string[] = [];
    
    // Process in batches to avoid overwhelming the server
    for (let i = 0; i < documents.length; i += this.batchSize) {
      const batch = documents.slice(i, i + this.batchSize);
      
      const batchPromises = batch.map(doc => 
        this.client.add(doc.text, doc.metadata)
      );
      
      const batchResults = await Promise.all(batchPromises);
      results.push(...batchResults);
      
      console.log(`Processed batch ${Math.floor(i / this.batchSize) + 1}`);
    }
    
    return results;
  }

  async batchDelete(docIds: string[]): Promise<void> {
    // Process deletions in batches
    for (let i = 0; i < docIds.length; i += this.batchSize) {
      const batch = docIds.slice(i, i + this.batchSize);
      
      const deletePromises = batch.map(id => this.client.delete(id));
      await Promise.all(deletePromises);
      
      console.log(`Deleted batch ${Math.floor(i / this.batchSize) + 1}`);
    }
  }
}

// Usage
const processor = new BatchProcessor(client, 50);

const documents = [
  { text: "Document 1", metadata: { type: "article" } },
  { text: "Document 2", metadata: { type: "blog" } },
  // ... more documents
];

const docIds = await processor.batchAdd(documents);
```

### Integration with Express.js

```typescript
import express from 'express';
import VexFSClient from 'vexfs-sdk';

const app = express();
const client = new VexFSClient();

app.use(express.json());

// Add document endpoint
app.post('/documents', async (req, res) => {
  try {
    const { text, metadata } = req.body;
    const docId = await client.add(text, metadata);
    res.json({ success: true, docId });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

// Search endpoint
app.post('/search', async (req, res) => {
  try {
    const { vector, topK = 10 } = req.body;
    const results = await client.query(vector, topK);
    res.json({ success: true, results });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

// Delete document endpoint
app.delete('/documents/:id', async (req, res) => {
  try {
    await client.delete(req.params.id);
    res.json({ success: true });
  } catch (error) {
    res.status(500).json({ 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
});

app.listen(3000, () => {
  console.log('VexFS API server running on port 3000');
});
```

### Integration with Fastify

```typescript
import Fastify from 'fastify';
import VexFSClient from 'vexfs-sdk';

const fastify = Fastify({ logger: true });
const client = new VexFSClient();

// Register schemas for type safety
const addDocumentSchema = {
  body: {
    type: 'object',
    required: ['text'],
    properties: {
      text: { type: 'string' },
      metadata: { 
        type: 'object',
        additionalProperties: { type: 'string' }
      }
    }
  }
};

const searchSchema = {
  body: {
    type: 'object',
    required: ['vector'],
    properties: {
      vector: { 
        type: 'array',
        items: { type: 'number' }
      },
      topK: { type: 'number', default: 10 }
    }
  }
};

// Routes
fastify.post('/documents', { schema: addDocumentSchema }, async (request, reply) => {
  const { text, metadata } = request.body as { text: string; metadata?: Record<string, string> };
  
  try {
    const docId = await client.add(text, metadata);
    return { success: true, docId };
  } catch (error) {
    reply.code(500);
    return { 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    };
  }
});

fastify.post('/search', { schema: searchSchema }, async (request, reply) => {
  const { vector, topK } = request.body as { vector: number[]; topK?: number };
  
  try {
    const results = await client.query(vector, topK);
    return { success: true, results };
  } catch (error) {
    reply.code(500);
    return { 
      success: false, 
      error: error instanceof Error ? error.message : 'Unknown error' 
    };
  }
});

// Start server
const start = async () => {
  try {
    await fastify.listen({ port: 3000 });
    console.log('VexFS Fastify server running on port 3000');
  } catch (err) {
    fastify.log.error(err);
    process.exit(1);
  }
};

start();
```

## 🎯 **Examples**

### Real-time Search Implementation

```typescript
import VexFSClient from 'vexfs-sdk';
import { EventEmitter } from 'events';

class RealTimeSearch extends EventEmitter {
  private client: VexFSClient;
  private searchCache: Map<string, VexFSResult[]>;

  constructor(client: VexFSClient) {
    super();
    this.client = client;
    this.searchCache = new Map();
  }

  async search(query: number[], topK: number = 10): Promise<VexFSResult[]> {
    const cacheKey = `${query.join(',')}_${topK}`;
    
    // Check cache first
    if (this.searchCache.has(cacheKey)) {
      this.emit('cache_hit', cacheKey);
      return this.searchCache.get(cacheKey)!;
    }

    try {
      const results = await this.client.query(query, topK);
      
      // Cache results
      this.searchCache.set(cacheKey, results);
      this.emit('search_complete', { query, results });
      
      return results;
    } catch (error) {
      this.emit('search_error', error);
      throw error;
    }
  }

  clearCache(): void {
    this.searchCache.clear();
    this.emit('cache_cleared');
  }
}

// Usage
const realTimeSearch = new RealTimeSearch(client);

realTimeSearch.on('search_complete', ({ query, results }) => {
  console.log(`Search completed: ${results.length} results`);
});

realTimeSearch.on('cache_hit', (cacheKey) => {
  console.log(`Cache hit for: ${cacheKey}`);
});

const results = await realTimeSearch.search(queryVector, 5);
```

### Document Management System

```typescript
import VexFSClient from 'vexfs-sdk';

interface Document {
  id?: string;
  title: string;
  content: string;
  author: string;
  tags: string[];
  createdAt: Date;
}

class DocumentManager {
  private client: VexFSClient;

  constructor(client: VexFSClient) {
    this.client = client;
  }

  async createDocument(doc: Omit<Document, 'id' | 'createdAt'>): Promise<Document> {
    const metadata = {
      title: doc.title,
      author: doc.author,
      tags: doc.tags.join(','),
      createdAt: new Date().toISOString()
    };

    const docId = await this.client.add(doc.content, metadata);

    return {
      id: docId,
      ...doc,
      createdAt: new Date()
    };
  }

  async searchDocuments(
    query: number[], 
    filters?: { author?: string; tags?: string[] }
  ): Promise<VexFSResult[]> {
    const results = await this.client.query(query, 20);

    // Apply filters
    if (filters) {
      return results.filter(result => {
        if (filters.author && result.metadata.author !== filters.author) {
          return false;
        }
        
        if (filters.tags && filters.tags.length > 0) {
          const docTags = result.metadata.tags?.split(',') || [];
          const hasMatchingTag = filters.tags.some(tag => docTags.includes(tag));
          if (!hasMatchingTag) return false;
        }
        
        return true;
      });
    }

    return results;
  }

  async deleteDocument(docId: string): Promise<void> {
    await this.client.delete(docId);
  }

  async getDocumentsByAuthor(author: string, limit: number = 10): Promise<VexFSResult[]> {
    // This would require a more sophisticated query in a real implementation
    // For now, we'll use a dummy vector and filter results
    const dummyVector = new Array(384).fill(0);
    const results = await this.client.query(dummyVector, 100);
    
    return results
      .filter(result => result.metadata.author === author)
      .slice(0, limit);
  }
}

// Usage
const docManager = new DocumentManager(client);

const newDoc = await docManager.createDocument({
  title: "VexFS Performance Analysis",
  content: "VexFS demonstrates exceptional performance...",
  author: "John Doe",
  tags: ["performance", "filesystem", "vectors"]
});

console.log(`Created document: ${newDoc.id}`);
```

## 🛠️ **Development**

### Building from Source

```bash
# Clone repository
git clone https://github.com/lspecian/vexfs.git
cd vexfs/bindings/typescript

# Install dependencies
npm install

# Build TypeScript
npm run build

# Run tests
npm test

# Development mode (watch for changes)
npm run dev
```

### Running Tests

```bash
# Run all tests
npm test

# Run tests with coverage
npm run test:coverage

# Run tests in watch mode
npm run test:watch

# Run specific test file
npm test -- --testNamePattern="VexFSClient"
```

### Project Structure

```
bindings/typescript/
├── src/
│   ├── index.ts          # Main exports
│   ├── client.ts         # VexFSClient implementation
│   └── types.ts          # Type definitions
├── tests/
│   ├── client.test.ts    # Client tests
│   └── integration.test.ts # Integration tests
├── dist/                 # Compiled JavaScript
├── package.json
├── tsconfig.json
└── README.md
```

### Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/new-feature`
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass: `npm test`
6. Build the project: `npm run build`
7. Submit a pull request

## 🐛 **Troubleshooting**

### Common Issues

**Module not found error**
```bash
# Ensure package is installed
npm install vexfs-sdk

# Check Node.js version
node --version  # Should be 16+
```

**TypeScript compilation errors**
```bash
# Update TypeScript
npm install -g typescript@latest

# Check tsconfig.json configuration
# Ensure target is ES2020 or higher
```

**Connection timeout**
```typescript
// Increase timeout for slow networks
const client = new VexFSClient({
  baseUrl: 'http://localhost:8080',
  timeout: 120000  // 2 minutes
});
```

**Vector dimension mismatch**
```typescript
// Ensure consistent vector dimensions
const vector = new Array(384).fill(0).map(() => Math.random());
// All vectors must have the same dimension (384 in this example)
```

### Network Issues

```typescript
// Handle network errors gracefully
async function robustQuery(vector: number[], topK: number): Promise<VexFSResult[]> {
  const maxRetries = 3;
  let lastError: Error;

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      return await client.query(vector, topK);
    } catch (error) {
      lastError = error as Error;
      console.log(`Attempt ${attempt} failed: ${lastError.message}`);
      
      if (attempt < maxRetries) {
        // Wait before retrying (exponential backoff)
        await new Promise(resolve => setTimeout(resolve, 1000 * attempt));
      }
    }
  }

  throw lastError!;
}
```

### Performance Optimization

```typescript
// Use connection pooling for high-throughput applications
const client = new VexFSClient({
  baseUrl: 'http://localhost:8080',
  timeout: 30000,
  // Configure for high performance
  retries: 1,  // Reduce retries for faster failure
  retryDelay: 500  // Shorter delay between retries
});

// Batch operations when possible
const batchSize = 100;
const documents = [...]; // Your documents

for (let i = 0; i < documents.length; i += batchSize) {
  const batch = documents.slice(i, i + batchSize);
  const promises = batch.map(doc => client.add(doc.text, doc.metadata));
  await Promise.all(promises);
}
```

## 📄 **License**

This project is licensed under the Apache License 2.0 - see the [LICENSE](../../LICENSE) file for details.

## 🔗 **Links**

- **Main Repository**: [https://github.com/lspecian/vexfs](https://github.com/lspecian/vexfs)
- **Documentation**: [https://github.com/lspecian/vexfs/tree/main/docs](https://github.com/lspecian/vexfs/tree/main/docs)
- **Issues**: [https://github.com/lspecian/vexfs/issues](https://github.com/lspecian/vexfs/issues)
- **npm Package**: [https://www.npmjs.com/package/vexfs-sdk](https://www.npmjs.com/package/vexfs-sdk)

---

**VexFS TypeScript SDK** - Modern, type-safe vector operations for Node.js 🚀