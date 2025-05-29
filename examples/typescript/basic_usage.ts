#!/usr/bin/env node
/**
 * VexFS TypeScript SDK Basic Usage Example
 * 
 * This example demonstrates how to use the VexFS TypeScript SDK for:
 * - Connecting to VexFS server
 * - Creating collections
 * - Adding documents with metadata
 * - Querying for similar documents
 * - Managing collections
 */

import { VexFSClient, VexFSResult, Collection } from '../../bindings/typescript/src/client';

async function main(): Promise<void> {
    console.log('🚀 VexFS TypeScript SDK Basic Usage Example');
    console.log('='.repeat(50));

    // Initialize client
    const client = new VexFSClient({
        baseUrl: 'http://localhost:8000',
        timeout: 30000,
        defaultCollection: 'example_collection'
    });

    try {
        // Check server version
        console.log('\n📡 Connecting to VexFS server...');
        const version = await client.version();
        console.log(`Server version: ${version}`);

        // Example 1: Collection Management
        console.log('\n📚 Example 1: Collection Management');
        
        const collectionName = 'example_collection';
        
        try {
            // Create a new collection
            console.log(`Creating collection: ${collectionName}`);
            const collection = await client.createCollection(collectionName, {
                description: 'Example collection for testing',
                created_at: new Date().toISOString()
            });
            console.log(`✅ Created collection: ${collection.name} (ID: ${collection.id})`);
        } catch (error) {
            console.log(`Collection might already exist: ${error}`);
        }

        // List all collections
        console.log('\n📋 Listing all collections:');
        const collections = await client.listCollections();
        collections.forEach((col, index) => {
            console.log(`  ${index + 1}. ${col.name} (ID: ${col.id})`);
            if (col.metadata) {
                console.log(`     Metadata: ${JSON.stringify(col.metadata)}`);
            }
        });

        // Example 2: Document Operations
        console.log('\n📝 Example 2: Document Operations');

        // Add documents
        console.log('Adding documents...');
        
        const doc1Id = await client.add(
            'The quick brown fox jumps over the lazy dog',
            { category: 'animals', type: 'sentence', language: 'english' },
            collectionName
        );
        console.log(`✅ Added document 1: ${doc1Id}`);

        const doc2Id = await client.add(
            'Machine learning is a subset of artificial intelligence that focuses on algorithms',
            { category: 'technology', type: 'definition', language: 'english' },
            collectionName
        );
        console.log(`✅ Added document 2: ${doc2Id}`);

        const doc3Id = await client.add(
            'Vector databases enable semantic search capabilities for modern applications',
            { category: 'technology', type: 'explanation', language: 'english' },
            collectionName
        );
        console.log(`✅ Added document 3: ${doc3Id}`);

        const doc4Id = await client.add(
            'Natural language processing helps computers understand human language',
            { category: 'technology', type: 'explanation', language: 'english' },
            collectionName
        );
        console.log(`✅ Added document 4: ${doc4Id}`);

        // Example 3: Querying Documents
        console.log('\n🔍 Example 3: Querying Documents');

        // Generate sample query vectors
        const queries = [
            {
                name: 'Technology Query',
                vector: generateSampleVector('technology machine learning artificial intelligence'),
                description: 'Looking for technology-related content'
            },
            {
                name: 'Animals Query', 
                vector: generateSampleVector('animals fox dog lazy brown'),
                description: 'Looking for animal-related content'
            },
            {
                name: 'Language Query',
                vector: generateSampleVector('language processing natural understanding'),
                description: 'Looking for language processing content'
            }
        ];

        for (const query of queries) {
            console.log(`\n🎯 ${query.name}: ${query.description}`);
            
            try {
                const results = await client.query(query.vector, 3, collectionName);
                
                if (results.length === 0) {
                    console.log('   No results found');
                } else {
                    console.log(`   Found ${results.length} results:`);
                    results.forEach((result, index) => {
                        console.log(`   ${index + 1}. Score: ${result.score.toFixed(4)}`);
                        console.log(`      ID: ${result.id}`);
                        console.log(`      Text: ${result.document || 'N/A'}`);
                        console.log(`      Metadata: ${JSON.stringify(result.metadata)}`);
                        console.log();
                    });
                }
            } catch (error) {
                console.log(`   ❌ Query failed: ${error}`);
            }
        }

        // Example 4: Collection Information
        console.log('\n📊 Example 4: Collection Information');
        
        try {
            const collectionInfo = await client.getCollection(collectionName);
            console.log(`Collection: ${collectionInfo.name}`);
            console.log(`ID: ${collectionInfo.id}`);
            console.log(`Metadata: ${JSON.stringify(collectionInfo.metadata, null, 2)}`);
        } catch (error) {
            console.log(`❌ Failed to get collection info: ${error}`);
        }

        console.log('\n✅ Example completed successfully!');
        console.log('\n💡 Tips:');
        console.log('   - Make sure the VexFS server is running on http://localhost:8000');
        console.log('   - You can start the server with: cargo run --bin vexfs_server');
        console.log('   - Check the server logs for any errors');

    } catch (error) {
        console.error(`❌ Error: ${error}`);
        console.log('\n🔧 Troubleshooting:');
        console.log('   1. Ensure VexFS server is running: cargo run --bin vexfs_server');
        console.log('   2. Check server is accessible at http://localhost:8000');
        console.log('   3. Verify no firewall is blocking the connection');
        process.exit(1);
    }
}

/**
 * Generate a simple embedding vector based on text content
 * In a real application, this would use a proper embedding model
 */
function generateSampleVector(text: string): number[] {
    const words = text.toLowerCase().split(/\s+/);
    const embedding = new Array(384).fill(0);
    
    // Simple hash-based embedding
    for (let i = 0; i < words.length; i++) {
        const word = words[i];
        const hash = simpleHash(word);
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
function simpleHash(str: string): number {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        const char = str.charCodeAt(i);
        hash = ((hash << 5) - hash) + char;
        hash = hash & hash; // Convert to 32-bit integer
    }
    return Math.abs(hash);
}

// Run the example
if (require.main === module) {
    main().catch(console.error);
}