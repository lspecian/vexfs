import { test, expect } from '@playwright/test';
import { server } from '../setup/mock-server';

test.describe('VexGraph Complete Workflow Tests', () => {
  test.beforeAll(async () => {
    server.listen();
  });

  test.afterAll(async () => {
    server.close();
  });

  test.beforeEach(async ({ page }) => {
    await page.goto('/ui/graph');
    await page.waitForSelector('[data-testid="graph-page-title"]');
  });

  test.describe('Complete Graph Management Workflow', () => {
    test('should complete full graph lifecycle: schema → nodes → edges → query → analyze', async ({ page }) => {
      // Step 1: Define Schema
      await page.click('text=Schema Management');
      
      // Create File node type
      await page.click('text=Add Node Type');
      await page.fill('input[name="nodeTypeName"]', 'ConfigFile');
      await page.click('text=Add Required Property');
      await page.fill('input[name="propertyName"]', 'filename');
      await page.selectOption('select[name="propertyType"]', 'String');
      await page.click('text=Add Optional Property');
      await page.fill('input[name="propertyName"]', 'size');
      await page.selectOption('select[name="propertyType"]', 'Integer');
      await page.click('text=Save Node Type');
      
      // Create Directory node type
      await page.click('text=Add Node Type');
      await page.fill('input[name="nodeTypeName"]', 'ConfigDirectory');
      await page.click('text=Add Required Property');
      await page.fill('input[name="propertyName"]', 'path');
      await page.selectOption('select[name="propertyType"]', 'String');
      await page.click('text=Save Node Type');
      
      // Create Contains edge type
      await page.click('text=Add Edge Type');
      await page.fill('input[name="edgeTypeName"]', 'Contains');
      await page.selectOption('select[name="allowedSourceTypes"]', 'ConfigDirectory');
      await page.selectOption('select[name="allowedTargetTypes"]', 'ConfigFile');
      await page.click('text=Save Edge Type');
      
      await expect(page.locator('text=Schema saved successfully')).toBeVisible();
      
      // Step 2: Create Nodes
      await page.click('text=Node Management');
      
      // Create directory node
      await page.click('text=Create Node');
      await page.selectOption('select[name="nodeType"]', 'ConfigDirectory');
      await page.fill('input[name="properties.path"]', '/etc/config');
      await page.click('button[type="submit"]');
      await expect(page.locator('text=Node created successfully')).toBeVisible();
      
      // Create file nodes
      const files = [
        { name: 'app.conf', size: '1024' },
        { name: 'database.conf', size: '2048' },
        { name: 'logging.conf', size: '512' }
      ];
      
      for (const file of files) {
        await page.click('text=Create Node');
        await page.selectOption('select[name="nodeType"]', 'ConfigFile');
        await page.fill('input[name="properties.filename"]', file.name);
        await page.fill('input[name="properties.size"]', file.size);
        await page.click('button[type="submit"]');
        await expect(page.locator('text=Node created successfully')).toBeVisible();
      }
      
      // Step 3: Create Edges
      await page.click('text=Edge Management');
      
      // Connect directory to files
      for (let i = 0; i < files.length; i++) {
        await page.click('text=Create Edge');
        await page.selectOption('select[name="sourceId"]', { index: 0 }); // Directory
        await page.selectOption('select[name="targetId"]', { index: i + 1 }); // File
        await page.selectOption('select[name="edgeType"]', 'Contains');
        await page.fill('input[name="weight"]', '1.0');
        await page.click('button[type="submit"]');
        await expect(page.locator('text=Edge created successfully')).toBeVisible();
      }
      
      // Step 4: Execute Queries
      await page.click('text=Query Builder');
      
      // Breadth-first search from directory
      await page.selectOption('select[name="algorithm"]', 'BreadthFirstSearch');
      await page.selectOption('select[name="startNode"]', { index: 0 });
      await page.fill('input[name="maxDepth"]', '2');
      await page.click('text=Execute Query');
      
      await expect(page.locator('text=Query Results')).toBeVisible();
      await expect(page.locator('text=/Found \\d+ nodes/')).toBeVisible();
      
      // Step 5: Analyze Results
      await page.click('text=Analytics');
      
      // Verify graph statistics
      await expect(page.locator('[data-testid="node-count-metric"]')).toContainText('4'); // 1 dir + 3 files
      await expect(page.locator('[data-testid="edge-count-metric"]')).toContainText('3'); // 3 contains edges
      
      // Check centrality measures
      await expect(page.locator('text=Centrality Analysis')).toBeVisible();
      await expect(page.locator('text=Betweenness Centrality')).toBeVisible();
      
      // Verify the directory has highest centrality (hub node)
      const centralityResults = page.locator('[data-testid="centrality-results"]');
      await expect(centralityResults).toContainText('/etc/config');
    });

    test('should handle schema evolution with data migration', async ({ page }) => {
      // Create initial schema and data
      await page.click('text=Schema Management');
      await page.click('text=Add Node Type');
      await page.fill('input[name="nodeTypeName"]', 'Document');
      await page.click('text=Add Required Property');
      await page.fill('input[name="propertyName"]', 'title');
      await page.selectOption('select[name="propertyType"]', 'String');
      await page.click('text=Save Node Type');
      
      // Create some documents
      await page.click('text=Node Management');
      const documents = ['doc1.txt', 'doc2.txt', 'doc3.txt'];
      
      for (const doc of documents) {
        await page.click('text=Create Node');
        await page.selectOption('select[name="nodeType"]', 'Document');
        await page.fill('input[name="properties.title"]', doc);
        await page.click('button[type="submit"]');
        await expect(page.locator('text=Node created successfully')).toBeVisible();
      }
      
      // Evolve schema - add required author field
      await page.click('text=Schema Management');
      await page.click('text=Edit Document Type');
      await page.click('text=Add Required Property');
      await page.fill('input[name="propertyName"]', 'author');
      await page.selectOption('select[name="propertyType"]', 'String');
      await page.fill('input[name="defaultValue"]', 'Unknown Author');
      await page.click('text=Save Schema');
      
      // Handle migration
      await expect(page.locator('text=Schema Migration Required')).toBeVisible();
      await page.click('text=Migrate Existing Data');
      await expect(page.locator('text=Migration completed successfully')).toBeVisible();
      
      // Verify existing nodes have default author
      await page.click('text=Node Management');
      await page.click(`text=${documents[0]}`);
      await expect(page.locator('text=Unknown Author')).toBeVisible();
      
      // Verify new nodes require author
      await page.click('text=Create Node');
      await page.selectOption('select[name="nodeType"]', 'Document');
      await page.fill('input[name="properties.title"]', 'new-doc.txt');
      // Don't fill author - should fail
      await page.click('button[type="submit"]');
      await expect(page.locator('text=author is required')).toBeVisible();
      
      // Fill author and succeed
      await page.fill('input[name="properties.author"]', 'John Doe');
      await page.click('button[type="submit"]');
      await expect(page.locator('text=Node created successfully')).toBeVisible();
    });
  });

  test.describe('Collaborative Editing Workflow', () => {
    test('should handle multi-user real-time editing with conflict resolution', async ({ page, context }) => {
      // Open second tab to simulate another user
      const page2 = await context.newPage();
      await page2.goto('/ui/graph');
      await page2.waitForSelector('[data-testid="graph-page-title"]');
      
      // Both users see the same initial state
      await page.waitForSelector('.react-flow__node');
      await page2.waitForSelector('.react-flow__node');
      
      // User 1 selects and starts editing a node
      await page.click('.react-flow__node >> nth=0');
      await page.click('text=Edit Selected');
      await page.fill('input[name="properties.name"]', 'user1-edit');
      
      // User 2 selects the same node and starts editing
      await page2.click('.react-flow__node >> nth=0');
      await page2.click('text=Edit Selected');
      await page2.fill('input[name="properties.name"]', 'user2-edit');
      
      // User 2 saves first
      await page2.click('button[type="submit"]');
      await expect(page2.locator('text=Node updated successfully')).toBeVisible();
      
      // User 1 tries to save - should detect conflict
      await page.click('button[type="submit"]');
      await expect(page.locator('text=Conflict Detected')).toBeVisible();
      await expect(page.locator('text=user1-edit')).toBeVisible();
      await expect(page.locator('text=user2-edit')).toBeVisible();
      
      // User 1 resolves conflict by merging
      await page.click('text=Merge Changes');
      await page.fill('input[name="mergedValue"]', 'user1-user2-merged');
      await page.click('text=Save Merged');
      
      // Both users should see the merged result
      await expect(page.locator('text=user1-user2-merged')).toBeVisible();
      await expect(page2.locator('text=user1-user2-merged')).toBeVisible();
      
      await page2.close();
    });

    test('should synchronize real-time updates across multiple views', async ({ page }) => {
      // Open split view with multiple components
      await page.click('text=Split View');
      
      // Verify multiple panels are visible
      await expect(page.locator('[data-testid="visualization-panel"]')).toBeVisible();
      await expect(page.locator('[data-testid="analytics-panel"]')).toBeVisible();
      await expect(page.locator('[data-testid="query-panel"]')).toBeVisible();
      
      // Create a node in one panel
      await page.locator('[data-testid="visualization-panel"]').click();
      await page.click('text=Create Node');
      await page.fill('input[name="nodeType"]', 'File');
      await page.fill('input[name="properties.name"]', 'sync-test-file');
      await page.click('button[type="submit"]');
      
      // Verify update appears in all panels
      await expect(page.locator('[data-testid="visualization-panel"] text=sync-test-file')).toBeVisible();
      await expect(page.locator('[data-testid="analytics-panel"] text=sync-test-file')).toBeVisible();
      
      // Update analytics should reflect new node
      const nodeCount = await page.locator('[data-testid="analytics-panel"] [data-testid="node-count"]').textContent();
      expect(parseInt(nodeCount!)).toBeGreaterThan(0);
      
      // Query panel should include new node in results
      await page.locator('[data-testid="query-panel"]').click();
      await page.selectOption('select[name="algorithm"]', 'BreadthFirstSearch');
      await page.click('text=Execute Query');
      await expect(page.locator('[data-testid="query-results"]')).toContainText('sync-test-file');
    });
  });

  test.describe('Semantic Analysis Workflow', () => {
    test('should complete semantic search → query building → analytics pipeline', async ({ page }) => {
      // Step 1: Perform semantic search
      await page.click('text=Semantic Search');
      await page.fill('input[name="searchQuery"]', 'configuration and settings files');
      
      // Apply filters
      await page.check('input[name="nodeTypes"][value="File"]');
      await page.fill('input[name="minRelevance"]', '0.7');
      
      await page.click('text=Search');
      await expect(page.locator('text=Search Results')).toBeVisible();
      
      // Step 2: Build query from search results
      await page.click('text=Build Query from Results');
      
      // Verify query builder populated with search results
      await expect(page.locator('select[name="algorithm"]')).toHaveValue('SemanticTraversal');
      await expect(page.locator('input[name="semanticQuery"]')).toHaveValue('configuration and settings files');
      
      // Refine query
      await page.fill('input[name="maxDepth"]', '3');
      await page.fill('input[name="minSimilarity"]', '0.8');
      
      await page.click('text=Execute Query');
      await expect(page.locator('text=Query Results')).toBeVisible();
      
      // Step 3: Analyze semantic patterns
      await page.click('text=Semantic Analytics');
      
      // Verify semantic clustering results
      await expect(page.locator('text=Semantic Clusters')).toBeVisible();
      await expect(page.locator('text=Topic Distribution')).toBeVisible();
      
      // Check similarity network
      await expect(page.locator('[data-testid="similarity-network"]')).toBeVisible();
      
      // Verify semantic evolution over time
      await expect(page.locator('text=Semantic Evolution')).toBeVisible();
      await expect(page.locator('[data-testid="evolution-timeline"]')).toBeVisible();
      
      // Step 4: Export semantic insights
      await page.click('text=Export Insights');
      await page.selectOption('select[name="format"]', 'json');
      await page.check('input[name="includeClusters"]');
      await page.check('input[name="includeTopics"]');
      await page.check('input[name="includeSimilarities"]');
      
      await page.click('text=Generate Export');
      await expect(page.locator('text=Export generated successfully')).toBeVisible();
      
      // Verify export contains expected data
      const exportData = await page.locator('[data-testid="export-preview"]').textContent();
      expect(exportData).toContain('semantic_clusters');
      expect(exportData).toContain('topic_distribution');
      expect(exportData).toContain('similarity_scores');
    });

    test('should handle semantic search with vector similarity', async ({ page }) => {
      // Upload document for vectorization
      await page.click('text=Upload Document');
      await page.setInputFiles('input[type="file"]', {
        name: 'test-document.txt',
        mimeType: 'text/plain',
        buffer: new TextEncoder().encode('This is a configuration file for database settings and logging parameters.')
      });
      
      await page.click('text=Vectorize and Index');
      await expect(page.locator('text=Document vectorized successfully')).toBeVisible();
      
      // Perform vector similarity search
      await page.click('text=Vector Search');
      await page.fill('textarea[name="queryText"]', 'database configuration settings');
      await page.selectOption('select[name="similarityMetric"]', 'cosine');
      await page.fill('input[name="topK"]', '10');
      
      await page.click('text=Search Similar');
      await expect(page.locator('text=Vector Search Results')).toBeVisible();
      
      // Verify similarity scores
      const results = page.locator('[data-testid="vector-result-item"]');
      const count = await results.count();
      expect(count).toBeGreaterThan(0);
      
      // Check that results are sorted by similarity
      const scores = await results.locator('[data-testid="similarity-score"]').allTextContents();
      const numericScores = scores.map(s => parseFloat(s));
      
      for (let i = 1; i < numericScores.length; i++) {
        expect(numericScores[i]).toBeLessThanOrEqual(numericScores[i - 1]);
      }
      
      // Explore similar documents
      await results.first().click();
      await expect(page.locator('text=Document Details')).toBeVisible();
      await expect(page.locator('text=Vector Embedding')).toBeVisible();
      
      // Find related documents
      await page.click('text=Find Related');
      await expect(page.locator('text=Related Documents')).toBeVisible();
      
      // Verify related documents network
      await expect(page.locator('[data-testid="related-docs-network"]')).toBeVisible();
    });
  });

  test.describe('Schema Evolution Workflow', () => {
    test('should handle complex schema evolution with validation', async ({ page }) => {
      // Phase 1: Initial schema
      await page.click('text=Schema Management');
      
      // Create basic file schema
      await page.click('text=Add Node Type');
      await page.fill('input[name="nodeTypeName"]', 'BasicFile');
      await page.click('text=Add Required Property');
      await page.fill('input[name="propertyName"]', 'name');
      await page.selectOption('select[name="propertyType"]', 'String');
      await page.click('text=Save Node Type');
      
      // Create some files
      await page.click('text=Node Management');
      const files = ['file1.txt', 'file2.txt', 'file3.txt'];
      
      for (const file of files) {
        await page.click('text=Create Node');
        await page.selectOption('select[name="nodeType"]', 'BasicFile');
        await page.fill('input[name="properties.name"]', file);
        await page.click('button[type="submit"]');
      }
      
      // Phase 2: Add optional properties
      await page.click('text=Schema Management');
      await page.click('text=Edit BasicFile Type');
      await page.click('text=Add Optional Property');
      await page.fill('input[name="propertyName"]', 'size');
      await page.selectOption('select[name="propertyType"]', 'Integer');
      await page.fill('input[name="defaultValue"]', '0');
      
      await page.click('text=Add Optional Property');
      await page.fill('input[name="propertyName"]', 'created_at');
      await page.selectOption('select[name="propertyType"]', 'DateTime');
      await page.fill('input[name="defaultValue"]', 'now()');
      
      await page.click('text=Save Schema');
      await expect(page.locator('text=Schema updated successfully')).toBeVisible();
      
      // Phase 3: Make optional property required
      await page.click('text=Edit BasicFile Type');
      await page.click('[data-testid="make-required-size"]');
      await page.fill('input[name="migrationValue"]', '1024');
      await page.click('text=Save Schema');
      
      // Handle migration
      await expect(page.locator('text=Migration Required')).toBeVisible();
      await page.click('text=Apply Migration');
      await expect(page.locator('text=Migration completed')).toBeVisible();
      
      // Phase 4: Add validation rules
      await page.click('text=Edit BasicFile Type');
      await page.click('text=Add Validation Rule');
      await page.selectOption('select[name="property"]', 'size');
      await page.selectOption('select[name="rule"]', 'min_value');
      await page.fill('input[name="value"]', '1');
      
      await page.click('text=Add Validation Rule');
      await page.selectOption('select[name="property"]', 'name');
      await page.selectOption('select[name="rule"]', 'regex');
      await page.fill('input[name="value"]', '^[a-zA-Z0-9._-]+\\.[a-zA-Z]{2,4}$');
      
      await page.click('text=Save Schema');
      
      // Phase 5: Test validation
      await page.click('text=Node Management');
      await page.click('text=Create Node');
      await page.selectOption('select[name="nodeType"]', 'BasicFile');
      await page.fill('input[name="properties.name"]', 'invalid name!@#');
      await page.fill('input[name="properties.size"]', '0');
      await page.click('button[type="submit"]');
      
      // Should fail validation
      await expect(page.locator('text=Validation failed')).toBeVisible();
      await expect(page.locator('text=Invalid filename format')).toBeVisible();
      await expect(page.locator('text=Size must be at least 1')).toBeVisible();
      
      // Fix and retry
      await page.fill('input[name="properties.name"]', 'valid-file.txt');
      await page.fill('input[name="properties.size"]', '2048');
      await page.click('button[type="submit"]');
      await expect(page.locator('text=Node created successfully')).toBeVisible();
      
      // Phase 6: Schema versioning
      await page.click('text=Schema Management');
      await page.click('text=Version History');
      
      // Verify all schema versions are tracked
      await expect(page.locator('text=Version 1.0.0')).toBeVisible(); // Initial
      await expect(page.locator('text=Version 1.1.0')).toBeVisible(); // Added optional props
      await expect(page.locator('text=Version 1.2.0')).toBeVisible(); // Made size required
      await expect(page.locator('text=Version 1.3.0')).toBeVisible(); // Added validation
      
      // Test rollback capability
      await page.click('text=Rollback to 1.1.0');
      await expect(page.locator('text=Rollback Confirmation')).toBeVisible();
      await page.click('text=Confirm Rollback');
      await expect(page.locator('text=Schema rolled back successfully')).toBeVisible();
      
      // Verify rollback worked
      await page.click('text=Edit BasicFile Type');
      const sizeProperty = page.locator('[data-testid="property-size"]');
      await expect(sizeProperty.locator('text=Optional')).toBeVisible();
    });
  });
});