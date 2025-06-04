#!/usr/bin/env node

/**
 * VexFS Basic Usage Example
 * 
 * Simple example showing how to get started with VexFS for vector storage and search
 */

import { VexFSClient } from '../src/index';

async function basicExample() {
    console.log('🚀 VexFS Basic Usage Example');
    console.log('============================\n');

    // Initialize VexFS client
    const client = new VexFSClient({
        baseUrl: 'http://localhost:8000', // Adjust based on your setup
        defaultCollection: 'my_documents'
    });

    try {
        // 1. Check VexFS version
        console.log('📋 Checking VexFS version...');
        const version = await client.version();
        console.log(`✅ VexFS Version: ${version}\n`);

        // 2. Create a collection
        console.log('📁 Creating collection...');
        await client.createCollection('my_documents', {
            description: 'My document collection',
            created_at: new Date().toISOString()
        });
        console.log('✅ Collection created\n');

        // 3. Add some documents
        console.log('📄 Adding documents...');
        const documents = [
            'VexFS is a high-performance vector filesystem for AI applications',
            'Machine learning models require efficient vector storage and retrieval',
            'Similarity search is essential for recommendation systems',
            'Vector databases enable semantic search capabilities',
            'AI pipelines benefit from optimized vector operations'
        ];

        const docIds: string[] = [];
        for (let i = 0; i < documents.length; i++) {
            const docId = await client.add(documents[i], {
                category: 'tech',
                index: i,
                added_at: new Date().toISOString()
            });
            docIds.push(docId);
            console.log(`  ✅ Added document ${i + 1}: ${docId}`);
        }
        console.log('');

        // 4. Perform a search
        console.log('🔍 Searching for similar documents...');
        const queryText = 'vector storage for machine learning';
        
        // Generate a simple query vector (in production, use proper embedding model)
        const queryVector = generateSimpleEmbedding(queryText);
        
        const results = await client.query(queryVector, 3);
        
        console.log(`📊 Found ${results.length} similar documents:`);
        results.forEach((result, index) => {
            console.log(`  ${index + 1}. Score: ${result.score.toFixed(4)}`);
            console.log(`     Text: ${result.document}`);
            console.log(`     Metadata: ${JSON.stringify(result.metadata)}`);
            console.log('');
        });

        // 5. List collections
        console.log('📋 Listing all collections...');
        const collections = await client.listCollections();
        console.log('Collections:');
        collections.forEach(collection => {
            console.log(`  - ${collection.name} (ID: ${collection.id})`);
        });

        console.log('\n✅ Basic example completed successfully!');

    } catch (error) {
        console.error('❌ Error:', error);
        process.exit(1);
    }
}

/**
 * Generate a simple embedding for demonstration
 * In production, use a proper embedding model like OpenAI, Sentence Transformers, etc.
 */
function generateSimpleEmbedding(text: string): number[] {
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

function simpleHash(str: string): number {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        const char = str.charCodeAt(i);
        hash = ((hash << 5) - hash) + char;
        hash = hash & hash;
    }
    return Math.abs(hash);
}

// Run example if this file is executed directly
if (require.main === module) {
    basicExample().catch(console.error);
}

export { basicExample };