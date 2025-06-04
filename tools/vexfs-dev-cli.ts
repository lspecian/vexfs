#!/usr/bin/env node

/**
 * VexFS Developer CLI Tool
 * 
 * Comprehensive command-line interface for VexFS development and testing
 */

import { Command } from 'commander';
import { VexFSClient } from '../bindings/typescript/src/index';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

interface CLIConfig {
    mountPath: string;
    baseUrl: string;
    defaultCollection: string;
}

class VexFSDevCLI {
    private config: CLIConfig;
    private client: VexFSClient;

    constructor() {
        this.config = this.loadConfig();
        this.client = new VexFSClient({
            baseUrl: this.config.baseUrl,
            defaultCollection: this.config.defaultCollection
        });
    }

    /**
     * Load CLI configuration
     */
    private loadConfig(): CLIConfig {
        const configPath = path.join(os.homedir(), '.vexfs-cli.json');
        
        const defaultConfig: CLIConfig = {
            mountPath: '/mnt/vexfs',
            baseUrl: 'http://localhost:8000',
            defaultCollection: 'default'
        };

        try {
            if (fs.existsSync(configPath)) {
                const userConfig = JSON.parse(fs.readFileSync(configPath, 'utf8'));
                return { ...defaultConfig, ...userConfig };
            }
        } catch (error) {
            console.warn(`Warning: Could not load config from ${configPath}, using defaults`);
        }

        return defaultConfig;
    }

    /**
     * Save CLI configuration
     */
    private saveConfig(): void {
        const configPath = path.join(os.homedir(), '.vexfs-cli.json');
        try {
            fs.writeFileSync(configPath, JSON.stringify(this.config, null, 2));
            console.log(`✅ Configuration saved to ${configPath}`);
        } catch (error) {
            console.error(`❌ Failed to save configuration: ${error}`);
        }
    }

    /**
     * Initialize VexFS development environment
     */
    async init(): Promise<void> {
        console.log('🚀 Initializing VexFS Development Environment');
        console.log('============================================\n');

        try {
            // Check if VexFS is accessible
            const version = await this.client.version();
            console.log(`✅ VexFS Version: ${version}`);

            // Create default collection
            try {
                await this.client.createCollection(this.config.defaultCollection, {
                    description: 'Default development collection',
                    created_at: new Date().toISOString()
                });
                console.log(`✅ Created default collection: ${this.config.defaultCollection}`);
            } catch (error) {
                console.log(`ℹ️  Default collection already exists: ${this.config.defaultCollection}`);
            }

            console.log('\n🎉 VexFS development environment ready!');
            console.log('\nNext steps:');
            console.log('  - Use "vexfs-cli add-docs <file>" to index documents');
            console.log('  - Use "vexfs-cli search <query>" to search vectors');
            console.log('  - Use "vexfs-cli benchmark" to test performance');

        } catch (error) {
            console.error('❌ Failed to initialize VexFS environment:', error);
            console.log('\nTroubleshooting:');
            console.log('  1. Ensure VexFS is mounted and accessible');
            console.log('  2. Check that the VexFS daemon is running');
            console.log('  3. Verify mount path and permissions');
            process.exit(1);
        }
    }

    /**
     * Show VexFS status
     */
    async status(): Promise<void> {
        console.log('📊 VexFS Status');
        console.log('===============\n');

        try {
            // Get version
            const version = await this.client.version();
            console.log(`Version: ${version}`);

            // List collections
            const collections = await this.client.listCollections();
            console.log(`Collections: ${collections.length}`);
            
            collections.forEach(collection => {
                console.log(`  - ${collection.name} (ID: ${collection.id})`);
            });

            // Show configuration
            console.log('\nConfiguration:');
            console.log(`  Mount Path: ${this.config.mountPath}`);
            console.log(`  Base URL: ${this.config.baseUrl}`);
            console.log(`  Default Collection: ${this.config.defaultCollection}`);

        } catch (error) {
            console.error('❌ Failed to get VexFS status:', error);
            process.exit(1);
        }
    }

    /**
     * Add documents from file
     */
    async addDocuments(filePath: string, collection?: string): Promise<void> {
        console.log(`📄 Adding documents from ${filePath}`);
        console.log('=====================================\n');

        const targetCollection = collection || this.config.defaultCollection;

        try {
            // Read file
            if (!fs.existsSync(filePath)) {
                throw new Error(`File not found: ${filePath}`);
            }

            const content = fs.readFileSync(filePath, 'utf8');
            let documents: string[] = [];

            // Parse different file formats
            const ext = path.extname(filePath).toLowerCase();
            switch (ext) {
                case '.json':
                    const jsonData = JSON.parse(content);
                    if (Array.isArray(jsonData)) {
                        documents = jsonData.map(item => 
                            typeof item === 'string' ? item : JSON.stringify(item)
                        );
                    } else {
                        documents = [JSON.stringify(jsonData)];
                    }
                    break;
                case '.txt':
                    // Split by paragraphs or lines
                    documents = content.split(/\n\s*\n/).filter(doc => doc.trim().length > 0);
                    break;
                default:
                    documents = [content];
            }

            console.log(`📊 Found ${documents.length} documents to index`);

            // Index documents
            const startTime = Date.now();
            const docIds: string[] = [];

            for (let i = 0; i < documents.length; i++) {
                const doc = documents[i].trim();
                if (doc.length === 0) continue;

                try {
                    const docId = await this.client.add(doc, {
                        source_file: filePath,
                        document_index: i,
                        added_at: new Date().toISOString()
                    }, targetCollection);
                    
                    docIds.push(docId);
                    
                    if ((i + 1) % 10 === 0) {
                        console.log(`  📈 Indexed ${i + 1}/${documents.length} documents`);
                    }
                } catch (error) {
                    console.error(`❌ Failed to index document ${i}: ${error}`);
                }
            }

            const duration = Date.now() - startTime;
            const throughput = (docIds.length / duration) * 1000;

            console.log(`\n✅ Successfully indexed ${docIds.length} documents`);
            console.log(`⏱️  Time: ${duration}ms`);
            console.log(`⚡ Throughput: ${throughput.toFixed(2)} docs/sec`);

        } catch (error) {
            console.error('❌ Failed to add documents:', error);
            process.exit(1);
        }
    }

    /**
     * Search for similar documents
     */
    async search(query: string, topK: number = 10, collection?: string): Promise<void> {
        console.log(`🔍 Searching: "${query}"`);
        console.log('========================\n');

        const targetCollection = collection || this.config.defaultCollection;

        try {
            // Generate query embedding (simple hash-based for demo)
            const queryVector = this.generateQueryEmbedding(query);

            // Perform search
            const startTime = Date.now();
            const results = await this.client.query(queryVector, topK, targetCollection);
            const searchTime = Date.now() - startTime;

            console.log(`📊 Found ${results.length} results in ${searchTime}ms\n`);

            // Display results
            results.forEach((result, index) => {
                console.log(`${index + 1}. Score: ${result.score.toFixed(4)}`);
                console.log(`   ID: ${result.id}`);
                console.log(`   Text: ${result.document?.substring(0, 150)}...`);
                if (result.metadata.source_file) {
                    console.log(`   Source: ${result.metadata.source_file}`);
                }
                console.log('');
            });

        } catch (error) {
            console.error('❌ Search failed:', error);
            process.exit(1);
        }
    }

    /**
     * Run performance benchmark
     */
    async benchmark(): Promise<void> {
        console.log('🏁 VexFS Performance Benchmark');
        console.log('==============================\n');

        const testSizes = [100, 500, 1000];
        const results: Record<string, any> = {};

        for (const size of testSizes) {
            console.log(`📊 Testing with ${size} documents...`);

            // Generate test documents
            const testDocs = Array.from({ length: size }, (_, i) => 
                `Test document ${i}: This is a sample document for performance testing with various keywords and content.`
            );

            // Measure indexing performance
            const indexStart = Date.now();
            const docIds: string[] = [];

            for (let i = 0; i < testDocs.length; i++) {
                try {
                    const docId = await this.client.add(testDocs[i], {
                        benchmark: true,
                        index: i
                    }, 'benchmark');
                    docIds.push(docId);
                } catch (error) {
                    console.error(`Failed to index document ${i}: ${error}`);
                }
            }

            const indexTime = Date.now() - indexStart;
            const indexThroughput = (docIds.length / indexTime) * 1000;

            // Measure search performance
            const searchStart = Date.now();
            const queryVector = this.generateQueryEmbedding('test performance benchmark');
            await this.client.query(queryVector, 10, 'benchmark');
            const searchTime = Date.now() - searchStart;

            results[`${size}_docs`] = {
                documents: size,
                indexed: docIds.length,
                index_time_ms: indexTime,
                index_throughput_docs_per_sec: indexThroughput.toFixed(2),
                search_time_ms: searchTime
            };

            console.log(`  ✅ Indexed: ${docIds.length}/${size} docs in ${indexTime}ms`);
            console.log(`  ⚡ Throughput: ${indexThroughput.toFixed(2)} docs/sec`);
            console.log(`  🔍 Search time: ${searchTime}ms\n`);
        }

        console.log('📈 Benchmark Results Summary:');
        console.table(results);
    }

    /**
     * Configure CLI settings
     */
    async configure(key?: string, value?: string): Promise<void> {
        if (!key) {
            console.log('🔧 Current Configuration:');
            console.log('========================\n');
            console.log(JSON.stringify(this.config, null, 2));
            return;
        }

        if (!value) {
            console.log(`Current value for ${key}: ${(this.config as any)[key]}`);
            return;
        }

        // Update configuration
        (this.config as any)[key] = value;
        this.saveConfig();

        // Recreate client with new config
        this.client = new VexFSClient({
            baseUrl: this.config.baseUrl,
            defaultCollection: this.config.defaultCollection
        });

        console.log(`✅ Updated ${key} = ${value}`);
    }

    /**
     * Generate simple query embedding
     */
    private generateQueryEmbedding(text: string): number[] {
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
}

// CLI Setup
const program = new Command();
const cli = new VexFSDevCLI();

program
    .name('vexfs-cli')
    .description('VexFS Developer CLI Tool')
    .version('1.0.0');

program
    .command('init')
    .description('Initialize VexFS development environment')
    .action(() => cli.init());

program
    .command('status')
    .description('Show VexFS status and configuration')
    .action(() => cli.status());

program
    .command('add-docs <file>')
    .description('Add documents from file to VexFS')
    .option('-c, --collection <name>', 'Target collection name')
    .action((file, options) => cli.addDocuments(file, options.collection));

program
    .command('search <query>')
    .description('Search for similar documents')
    .option('-k, --top-k <number>', 'Number of results to return', '10')
    .option('-c, --collection <name>', 'Collection to search in')
    .action((query, options) => cli.search(query, parseInt(options.topK), options.collection));

program
    .command('benchmark')
    .description('Run performance benchmark')
    .action(() => cli.benchmark());

program
    .command('config [key] [value]')
    .description('View or update configuration')
    .action((key, value) => cli.configure(key, value));

// Parse command line arguments
program.parse();