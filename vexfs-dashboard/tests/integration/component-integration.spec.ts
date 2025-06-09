import { test, expect } from '@playwright/test';
import { server } from '../setup/mock-server';

test.describe('VexGraph Component Integration Tests', () => {
  test.beforeAll(async () => {
    server.listen();
  });

  test.afterAll(async () => {
    server.close();
  });

  test.beforeEach(async ({ page }) => {
    await page.goto('/ui/graph');
  });

  test.describe('Visualization ↔ CRUD Integration', () => {
    test('should reflect node creation in visualization', async ({ page }) => {
      // Wait for graph to load
      await page.waitForSelector('[data-testid="graph-page-title"]');
      
      // Open node management
      await page.click('text=Create Node');
      
      // Fill node creation form
      await page.fill('input[name="nodeType"]', 'File');
      await page.fill('input[name="properties.name"]', 'test-integration-node');
      await page.fill('input[name="properties.size"]', '1024');
      
      // Submit form
      await page.click('button[type="submit"]');
      
      // Wait for success notification
      await expect(page.locator('text=Node created successfully')).toBeVisible();
      
      // Check if new node appears in visualization
      await page.waitForSelector('.react-flow__node', { timeout: 5000 });
      const nodes = page.locator('.react-flow__node');
      const nodeCount = await nodes.count();
      expect(nodeCount).toBeGreaterThan(0);
      
      // Verify node appears in node list
      await expect(page.locator('text=test-integration-node')).toBeVisible();
    });

    test('should reflect edge creation in visualization', async ({ page }) => {
      // Ensure we have at least 2 nodes
      await page.waitForSelector('.react-flow__node', { timeout: 5000 });
      
      // Open edge management
      await page.click('text=Create Edge');
      
      // Fill edge creation form
      await page.selectOption('select[name="sourceId"]', { index: 0 });
      await page.selectOption('select[name="targetId"]', { index: 1 });
      await page.selectOption('select[name="edgeType"]', 'Contains');
      await page.fill('input[name="weight"]', '0.8');
      
      // Submit form
      await page.click('button[type="submit"]');
      
      // Wait for success notification
      await expect(page.locator('text=Edge created successfully')).toBeVisible();
      
      // Check if new edge appears in visualization
      await page.waitForSelector('.react-flow__edge', { timeout: 5000 });
      const edges = page.locator('.react-flow__edge');
      const edgeCount = await edges.count();
      expect(edgeCount).toBeGreaterThan(0);
    });

    test('should update visualization when node is edited', async ({ page }) => {
      // Wait for nodes to load
      await page.waitForSelector('.react-flow__node', { timeout: 5000 });
      
      // Select a node
      await page.click('.react-flow__node >> nth=0');
      
      // Open edit dialog
      await page.click('text=Edit Selected');
      
      // Update node properties
      await page.fill('input[name="properties.name"]', 'updated-node-name');
      
      // Submit changes
      await page.click('button[type="submit"]');
      
      // Wait for success notification
      await expect(page.locator('text=Node updated successfully')).toBeVisible();
      
      // Verify updated name appears in visualization
      await expect(page.locator('text=updated-node-name')).toBeVisible();
    });

    test('should remove node from visualization when deleted', async ({ page }) => {
      // Wait for nodes to load
      await page.waitForSelector('.react-flow__node', { timeout: 5000 });
      const initialNodeCount = await page.locator('.react-flow__node').count();
      
      // Select a node
      await page.click('.react-flow__node >> nth=0');
      
      // Delete node
      await page.click('text=Delete Selected');
      
      // Confirm deletion
      await page.click('text=Confirm');
      
      // Wait for success notification
      await expect(page.locator('text=Node deleted successfully')).toBeVisible();
      
      // Verify node count decreased
      await page.waitForTimeout(1000); // Allow time for re-render
      const finalNodeCount = await page.locator('.react-flow__node').count();
      expect(finalNodeCount).toBe(initialNodeCount - 1);
    });
  });

  test.describe('Query Builder ↔ Visualization Integration', () => {
    test('should highlight query results in visualization', async ({ page }) => {
      // Wait for graph to load
      await page.waitForSelector('.react-flow__node', { timeout: 5000 });
      
      // Open query builder
      await page.click('text=Build Query');
      
      // Build a simple query
      await page.selectOption('select[name="algorithm"]', 'BreadthFirstSearch');
      await page.selectOption('select[name="startNode"]', { index: 0 });
      await page.fill('input[name="maxDepth"]', '2');
      
      // Execute query
      await page.click('text=Execute Query');
      
      // Wait for results
      await expect(page.locator('text=Query Results')).toBeVisible();
      
      // Check if results are highlighted in visualization
      const highlightedNodes = await page.locator('.react-flow__node.highlighted').count();
      expect(highlightedNodes).toBeGreaterThan(0);
      
      // Verify selection details show query results
      await expect(page.locator('text=Selected Nodes')).toBeVisible();
    });

    test('should update query results when graph changes', async ({ page }) => {
      // Execute initial query
      await page.click('text=Build Query');
      await page.selectOption('select[name="algorithm"]', 'BreadthFirstSearch');
      await page.selectOption('select[name="startNode"]', { index: 0 });
      await page.click('text=Execute Query');
      
      // Get initial result count
      const initialResults = await page.locator('text=/Found \\d+ nodes/').textContent();
      
      // Add a new node
      await page.click('text=Create Node');
      await page.fill('input[name="nodeType"]', 'File');
      await page.fill('input[name="properties.name"]', 'query-test-node');
      await page.click('button[type="submit"]');
      
      // Re-execute query
      await page.click('text=Execute Query');
      
      // Verify results may have changed
      await expect(page.locator('text=Query Results')).toBeVisible();
    });

    test('should save and load query templates', async ({ page }) => {
      // Build a query
      await page.click('text=Build Query');
      await page.selectOption('select[name="algorithm"]', 'DepthFirstSearch');
      await page.fill('input[name="maxDepth"]', '3');
      
      // Save as template
      await page.click('text=Save Template');
      await page.fill('input[name="templateName"]', 'DFS Depth 3');
      await page.fill('textarea[name="description"]', 'Depth-first search with max depth 3');
      await page.click('text=Save');
      
      // Clear query
      await page.click('text=Clear');
      
      // Load template
      await page.click('text=Load Template');
      await page.click('text=DFS Depth 3');
      
      // Verify template loaded correctly
      await expect(page.locator('select[name="algorithm"]')).toHaveValue('DepthFirstSearch');
      await expect(page.locator('input[name="maxDepth"]')).toHaveValue('3');
    });
  });

  test.describe('Semantic Search ↔ Visualization Integration', () => {
    test('should display semantic search results in graph', async ({ page }) => {
      // Open semantic search
      await page.click('text=Semantic Search');
      
      // Enter search query
      await page.fill('input[name="searchQuery"]', 'configuration files');
      
      // Execute search
      await page.click('text=Search');
      
      // Wait for results
      await expect(page.locator('text=Search Results')).toBeVisible();
      
      // Check if results are highlighted in visualization
      const searchResultNodes = await page.locator('.react-flow__node.search-result').count();
      expect(searchResultNodes).toBeGreaterThan(0);
      
      // Verify relevance scores are displayed
      await expect(page.locator('text=/Relevance: \\d+\\.\\d+/')).toBeVisible();
    });

    test('should filter search results by node type', async ({ page }) => {
      // Open semantic search
      await page.click('text=Semantic Search');
      
      // Enter search query
      await page.fill('input[name="searchQuery"]', 'files');
      
      // Apply node type filter
      await page.check('input[name="nodeTypes"][value="File"]');
      await page.uncheck('input[name="nodeTypes"][value="Directory"]');
      
      // Execute search
      await page.click('text=Search');
      
      // Verify only File nodes are in results
      const resultNodes = page.locator('.search-result-item');
      const nodeTypes = await resultNodes.locator('[data-testid="node-type"]').allTextContents();
      nodeTypes.forEach(type => expect(type).toBe('File'));
    });

    test('should save and manage search history', async ({ page }) => {
      // Perform multiple searches
      const searches = ['configuration files', 'log files', 'database files'];
      
      for (const query of searches) {
        await page.click('text=Semantic Search');
        await page.fill('input[name="searchQuery"]', query);
        await page.click('text=Search');
        await page.waitForSelector('text=Search Results');
      }
      
      // Open search history
      await page.click('text=Search History');
      
      // Verify all searches are in history
      for (const query of searches) {
        await expect(page.locator(`text=${query}`)).toBeVisible();
      }
      
      // Replay a search from history
      await page.click(`text=${searches[0]}`);
      
      // Verify search was replayed
      await expect(page.locator(`input[name="searchQuery"]`)).toHaveValue(searches[0]);
    });
  });

  test.describe('Analytics ↔ Real-Time Integration', () => {
    test('should update analytics when real-time changes occur', async ({ page }) => {
      // Navigate to analytics view
      await page.click('text=Analytics');
      
      // Get initial metrics
      const initialNodeCount = await page.locator('[data-testid="node-count-metric"]').textContent();
      
      // Simulate real-time node addition
      await page.evaluate(() => {
        // Simulate WebSocket message for new node
        window.dispatchEvent(new CustomEvent('vexgraph-node-added', {
          detail: {
            id: 'realtime-node-1',
            node_type: 'File',
            properties: { name: 'realtime-file.txt' }
          }
        }));
      });
      
      // Wait for analytics to update
      await page.waitForTimeout(1000);
      
      // Verify metrics updated
      const updatedNodeCount = await page.locator('[data-testid="node-count-metric"]').textContent();
      expect(parseInt(updatedNodeCount!)).toBeGreaterThan(parseInt(initialNodeCount!));
    });

    test('should show real-time performance metrics', async ({ page }) => {
      // Navigate to performance analytics
      await page.click('text=Performance');
      
      // Verify real-time charts are updating
      await expect(page.locator('[data-testid="query-performance-chart"]')).toBeVisible();
      await expect(page.locator('[data-testid="memory-usage-chart"]')).toBeVisible();
      
      // Execute a query to generate performance data
      await page.click('text=Build Query');
      await page.selectOption('select[name="algorithm"]', 'BreadthFirstSearch');
      await page.click('text=Execute Query');
      
      // Wait for performance metrics to update
      await page.waitForTimeout(2000);
      
      // Verify performance data is displayed
      await expect(page.locator('text=/Execution Time: \\d+ms/')).toBeVisible();
      await expect(page.locator('text=/Memory Used: \\d+MB/')).toBeVisible();
    });
  });

  test.describe('Schema Management ↔ CRUD Integration', () => {
    test('should enforce schema validation during node creation', async ({ page }) => {
      // Define a strict schema
      await page.click('text=Schema Management');
      await page.click('text=Add Node Type');
      
      await page.fill('input[name="nodeTypeName"]', 'StrictFile');
      await page.click('text=Add Required Property');
      await page.fill('input[name="propertyName"]', 'filename');
      await page.selectOption('select[name="propertyType"]', 'String');
      await page.click('text=Save Schema');
      
      // Try to create node without required property
      await page.click('text=Create Node');
      await page.selectOption('select[name="nodeType"]', 'StrictFile');
      // Don't fill required filename property
      await page.click('button[type="submit"]');
      
      // Verify validation error
      await expect(page.locator('text=filename is required')).toBeVisible();
      
      // Create node with required property
      await page.fill('input[name="properties.filename"]', 'valid-file.txt');
      await page.click('button[type="submit"]');
      
      // Verify successful creation
      await expect(page.locator('text=Node created successfully')).toBeVisible();
    });

    test('should validate edge types according to schema', async ({ page }) => {
      // Define edge type constraints
      await page.click('text=Schema Management');
      await page.click('text=Add Edge Type');
      
      await page.fill('input[name="edgeTypeName"]', 'FileContains');
      await page.selectOption('select[name="allowedSourceTypes"]', 'Directory');
      await page.selectOption('select[name="allowedTargetTypes"]', 'File');
      await page.click('text=Save Schema');
      
      // Try to create invalid edge
      await page.click('text=Create Edge');
      await page.selectOption('select[name="edgeType"]', 'FileContains');
      // Select File as source (should be Directory)
      await page.selectOption('select[name="sourceId"]', { index: 0 });
      await page.selectOption('select[name="targetId"]', { index: 1 });
      await page.click('button[type="submit"]');
      
      // Verify validation error
      await expect(page.locator('text=Invalid edge type for selected nodes')).toBeVisible();
    });

    test('should migrate data when schema evolves', async ({ page }) => {
      // Create initial schema
      await page.click('text=Schema Management');
      await page.click('text=Add Node Type');
      await page.fill('input[name="nodeTypeName"]', 'Document');
      await page.click('text=Save Schema');
      
      // Create nodes with initial schema
      await page.click('text=Create Node');
      await page.selectOption('select[name="nodeType"]', 'Document');
      await page.fill('input[name="properties.title"]', 'Test Document');
      await page.click('button[type="submit"]');
      
      // Evolve schema - add required property
      await page.click('text=Schema Management');
      await page.click('text=Edit Document Type');
      await page.click('text=Add Required Property');
      await page.fill('input[name="propertyName"]', 'author');
      await page.selectOption('select[name="propertyType"]', 'String');
      await page.fill('input[name="defaultValue"]', 'Unknown');
      await page.click('text=Save Schema');
      
      // Verify migration dialog appears
      await expect(page.locator('text=Schema Migration Required')).toBeVisible();
      await page.click('text=Migrate Data');
      
      // Verify existing nodes have default value
      await expect(page.locator('text=Migration completed successfully')).toBeVisible();
    });
  });

  test.describe('Real-Time ↔ All Components Integration', () => {
    test('should propagate real-time updates across all components', async ({ page }) => {
      // Open multiple component views
      await page.click('text=Split View');
      
      // Simulate real-time node addition
      await page.evaluate(() => {
        window.dispatchEvent(new CustomEvent('vexgraph-node-added', {
          detail: {
            id: 'realtime-node-2',
            node_type: 'Directory',
            properties: { name: 'realtime-dir' }
          }
        }));
      });
      
      // Verify update appears in visualization
      await expect(page.locator('.react-flow__node[data-id="realtime-node-2"]')).toBeVisible();
      
      // Verify update appears in node list
      await expect(page.locator('text=realtime-dir')).toBeVisible();
      
      // Verify analytics updated
      await page.click('text=Analytics');
      await expect(page.locator('[data-testid="recent-changes"]')).toContainText('realtime-node-2');
      
      // Verify schema validation still works
      await page.click('text=Schema Management');
      await expect(page.locator('text=Directory')).toBeVisible();
    });

    test('should handle real-time conflicts gracefully', async ({ page }) => {
      // Select a node
      await page.waitForSelector('.react-flow__node', { timeout: 5000 });
      await page.click('.react-flow__node >> nth=0');
      
      // Start editing
      await page.click('text=Edit Selected');
      await page.fill('input[name="properties.name"]', 'local-edit');
      
      // Simulate concurrent remote edit
      await page.evaluate(() => {
        window.dispatchEvent(new CustomEvent('vexgraph-node-updated', {
          detail: {
            id: 'node-1',
            properties: { name: 'remote-edit' },
            version: 2
          }
        }));
      });
      
      // Try to save local edit
      await page.click('button[type="submit"]');
      
      // Verify conflict resolution dialog appears
      await expect(page.locator('text=Conflict Detected')).toBeVisible();
      await expect(page.locator('text=local-edit')).toBeVisible();
      await expect(page.locator('text=remote-edit')).toBeVisible();
      
      // Resolve conflict
      await page.click('text=Use Remote Version');
      
      // Verify resolution
      await expect(page.locator('text=Conflict resolved')).toBeVisible();
      await expect(page.locator('text=remote-edit')).toBeVisible();
    });

    test('should maintain real-time connection status', async ({ page }) => {
      // Verify connection status indicator
      await expect(page.locator('[data-testid="connection-status"]')).toHaveText('Connected');
      
      // Simulate connection loss
      await page.evaluate(() => {
        window.dispatchEvent(new CustomEvent('vexgraph-connection-lost'));
      });
      
      // Verify disconnected state
      await expect(page.locator('[data-testid="connection-status"]')).toHaveText('Disconnected');
      await expect(page.locator('text=Real-time updates unavailable')).toBeVisible();
      
      // Simulate reconnection
      await page.evaluate(() => {
        window.dispatchEvent(new CustomEvent('vexgraph-connection-restored'));
      });
      
      // Verify reconnected state
      await expect(page.locator('[data-testid="connection-status"]')).toHaveText('Connected');
      await expect(page.locator('text=Real-time updates restored')).toBeVisible();
    });
  });
});