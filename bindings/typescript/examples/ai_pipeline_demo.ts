#!/usr/bin/env node

/**
 * VexFS AI Pipeline Demo
 * 
 * Demonstrates how to use VexFS for memory-driven AI pipelines:
 * - Vector indexing and search
 * - Real-time embedding storage
 * - Similarity search with metadata
 * - Performance monitoring
 */

import { VexFSClient, VexFSResult, Collection } from '../src/index';
import * as fs from 'fs';
import * as path from 'path';

interface EmbeddingData {
    id: string;
    vector: number[];
    metadata: {
        text: string;
        source: string;
        timestamp: number;
        category?: string;
        synthetic?: boolean;
        index?: number;
        [key: string]: any; // Allow additional properties
    };
}

class AIVectorPipeline {
    private vexfs: VexFSClient;
    private indexName: string;

    constructor(mountPath: string = '/mnt/vexfs', indexName: string = 'ai_pipeline') {
        this.vexfs = new VexFSClient({
            baseUrl: `file://${mountPath}`,
            defaultCollection: indexName
        });
        this.indexName = indexName;
    }

    /**
     * Initialize the AI pipeline with sample data
     */
    async initialize(): Promise<void> {
        console.log('🚀 Initializing VexFS AI Pipeline...');
        
        try {
            // Check VexFS version
            const version = await this.vexfs.version();
            console.log(`✅ VexFS Version: ${version}`);

            // Create collection for AI embeddings
            await this.createAICollection();
            
            console.log('✅ AI Pipeline initialized successfully');
        } catch (error) {
            console.error('❌ Failed to initialize pipeline:', error);
            throw error;
        }
    }

    /**
     * Create a specialized collection for AI embeddings
     */
    private async createAICollection(): Promise<void> {
        try {
            const collection = await this.vexfs.createCollection(this.indexName, {
                description: 'AI Pipeline Vector Storage',
                vector_dimension: 384,
                index_type: 'hnsw',
                distance_metric: 'cosine',
                created_at: new Date().toISOString()
            });
            console.log(`✅ Created AI collection: ${collection.name}`);
        } catch (error) {
            // Collection might already exist
            console.log(`ℹ️  Using existing collection: ${this.indexName}`);
        }
    }

    /**
     * Batch index embeddings from various sources
     */
    async batchIndexEmbeddings(embeddings: EmbeddingData[]): Promise<void> {
        console.log(`📊 Indexing ${embeddings.length} embeddings...`);
        
        const startTime = Date.now();
        let indexed = 0;

        for (const embedding of embeddings) {
            try {
                await this.vexfs.add(
                    embedding.metadata.text,
                    {
                        ...embedding.metadata,
                        vector_id: embedding.id,
                        indexed_at: new Date().toISOString()
                    },
                    this.indexName
                );
                indexed++;
                
                if (indexed % 100 === 0) {
                    console.log(`  📈 Indexed ${indexed}/${embeddings.length} embeddings`);
                }
            } catch (error) {
                console.error(`❌ Failed to index embedding ${embedding.id}:`, error);
            }
        }

        const duration = Date.now() - startTime;
        const throughput = (indexed / duration) * 1000; // embeddings per second
        
        console.log(`✅ Indexed ${indexed} embeddings in ${duration}ms`);
        console.log(`⚡ Throughput: ${throughput.toFixed(2)} embeddings/sec`);
    }

    /**
     * Perform semantic search with performance monitoring
     */
    async semanticSearch(
        queryText: string, 
        topK: number = 10,
        filters?: Record<string, any>
    ): Promise<VexFSResult[]> {
        console.log(`🔍 Searching for: "${queryText}" (top ${topK})`);
        
        const startTime = Date.now();
        
        try {
            // Generate query embedding (in production, use proper embedding model)
            const queryVector = this.generateQueryEmbedding(queryText);
            
            // Perform vector search
            const results = await this.vexfs.query(queryVector, topK, this.indexName);
            
            // Apply additional filters if provided
            const filteredResults = filters ? 
                results.filter(result => this.matchesFilters(result.metadata, filters)) : 
                results;

            const searchTime = Date.now() - startTime;
            
            console.log(`✅ Found ${filteredResults.length} results in ${searchTime}ms`);
            this.displaySearchResults(filteredResults);
            
            return filteredResults;
        } catch (error) {
            console.error('❌ Search failed:', error);
            throw error;
        }
    }

    /**
     * Real-time embedding ingestion simulation
     */
    async realTimeIngestion(durationMs: number = 30000): Promise<void> {
        console.log(`🔄 Starting real-time ingestion for ${durationMs/1000}s...`);
        
        const startTime = Date.now();
        let ingestedCount = 0;
        
        const interval = setInterval(async () => {
            try {
                // Simulate incoming data
                const embedding = this.generateSyntheticEmbedding(ingestedCount);
                
                await this.vexfs.add(
                    embedding.metadata.text,
                    embedding.metadata,
                    this.indexName
                );
                
                ingestedCount++;
                
                if (ingestedCount % 10 === 0) {
                    console.log(`  📊 Real-time ingested: ${ingestedCount} embeddings`);
                }
            } catch (error) {
                console.error('❌ Real-time ingestion error:', error);
            }
        }, 100); // Ingest every 100ms

        // Stop after duration
        setTimeout(() => {
            clearInterval(interval);
            const totalTime = Date.now() - startTime;
            const throughput = (ingestedCount / totalTime) * 1000;
            
            console.log(`✅ Real-time ingestion completed`);
            console.log(`📈 Ingested ${ingestedCount} embeddings in ${totalTime}ms`);
            console.log(`⚡ Real-time throughput: ${throughput.toFixed(2)} embeddings/sec`);
        }, durationMs);
    }

    /**
     * Performance benchmark
     */
    async runBenchmark(): Promise<void> {
        console.log('🏁 Running VexFS Performance Benchmark...');
        
        // Test different batch sizes
        const batchSizes = [10, 100, 1000];
        const results: Record<string, any> = {};
        
        for (const batchSize of batchSizes) {
            console.log(`\n📊 Testing batch size: ${batchSize}`);
            
            // Generate test data
            const testEmbeddings = Array.from({ length: batchSize }, (_, i) => 
                this.generateSyntheticEmbedding(i)
            );
            
            // Measure indexing performance
            const indexStart = Date.now();
            await this.batchIndexEmbeddings(testEmbeddings);
            const indexTime = Date.now() - indexStart;
            
            // Measure search performance
            const searchStart = Date.now();
            await this.semanticSearch('test query', 10);
            const searchTime = Date.now() - searchStart;
            
            results[`batch_${batchSize}`] = {
                indexing_time_ms: indexTime,
                indexing_throughput: (batchSize / indexTime) * 1000,
                search_time_ms: searchTime,
                batch_size: batchSize
            };
        }
        
        console.log('\n📈 Benchmark Results:');
        console.table(results);
    }

    /**
     * Generate synthetic embedding for testing
     */
    private generateSyntheticEmbedding(index: number): EmbeddingData {
        const categories = ['technology', 'science', 'business', 'health', 'education'];
        const sources = ['web', 'document', 'api', 'upload'];
        
        return {
            id: `synthetic_${index}_${Date.now()}`,
            vector: Array.from({ length: 384 }, () => Math.random() * 2 - 1),
            metadata: {
                text: `Synthetic document ${index} about ${categories[index % categories.length]}`,
                source: sources[index % sources.length],
                category: categories[index % categories.length],
                timestamp: Date.now(),
                synthetic: true,
                index: index
            }
        };
    }

    /**
     * Generate query embedding (placeholder - use real embedding model in production)
     */
    private generateQueryEmbedding(text: string): number[] {
        // Simple hash-based embedding for demo
        const words = text.toLowerCase().split(/\s+/);
        const embedding = new Array(384).fill(0);
        
        for (let i = 0; i < words.length; i++) {
            const word = words[i];
            const hash = this.simpleHash(word);
            const idx = hash % embedding.length;
            embedding[idx] += 1.0 / (i + 1);
        }
        
        // Normalize
        const magnitude = Math.sqrt(embedding.reduce((sum, val) => sum + val * val, 0));
        if (magnitude > 0) {
            for (let i = 0; i < embedding.length; i++) {
                embedding[i] /= magnitude;
            }
        }
        
        return embedding;
    }

    private simpleHash(str: string): number {
        let hash = 0;
        for (let i = 0; i < str.length; i++) {
            const char = str.charCodeAt(i);
            hash = ((hash << 5) - hash) + char;
            hash = hash & hash;
        }
        return Math.abs(hash);
    }

    /**
     * Check if metadata matches filters
     */
    private matchesFilters(metadata: Record<string, any>, filters: Record<string, any>): boolean {
        for (const [key, value] of Object.entries(filters)) {
            if (metadata[key] !== value) {
                return false;
            }
        }
        return true;
    }

    /**
     * Display search results in a formatted way
     */
    private displaySearchResults(results: VexFSResult[]): void {
        console.log('\n🎯 Search Results:');
        console.log('==================');
        
        results.forEach((result, index) => {
            console.log(`${index + 1}. Score: ${result.score.toFixed(4)}`);
            console.log(`   ID: ${result.id}`);
            console.log(`   Text: ${result.document?.substring(0, 100)}...`);
            console.log(`   Category: ${result.metadata.category || 'N/A'}`);
            console.log(`   Source: ${result.metadata.source || 'N/A'}`);
            console.log('');
        });
    }
}

/**
 * Main demo function
 */
async function runDemo(): Promise<void> {
    console.log('🎯 VexFS AI Pipeline Demo');
    console.log('=========================\n');

    const pipeline = new AIVectorPipeline();

    try {
        // Initialize pipeline
        await pipeline.initialize();

        // Generate and index sample data
        console.log('\n📊 Generating sample embeddings...');
        const sampleEmbeddings = Array.from({ length: 500 }, (_, i) => 
            pipeline['generateSyntheticEmbedding'](i)
        );
        
        await pipeline.batchIndexEmbeddings(sampleEmbeddings);

        // Perform semantic searches
        console.log('\n🔍 Testing semantic search...');
        await pipeline.semanticSearch('technology innovation', 5);
        await pipeline.semanticSearch('health and wellness', 5, { category: 'health' });

        // Run performance benchmark
        console.log('\n🏁 Running performance benchmark...');
        await pipeline.runBenchmark();

        // Simulate real-time ingestion
        console.log('\n🔄 Testing real-time ingestion...');
        await pipeline.realTimeIngestion(10000); // 10 seconds

        console.log('\n✅ Demo completed successfully!');
        
    } catch (error) {
        console.error('❌ Demo failed:', error);
        process.exit(1);
    }
}

// Run demo if this file is executed directly
if (require.main === module) {
    runDemo().catch(console.error);
}

export { AIVectorPipeline };