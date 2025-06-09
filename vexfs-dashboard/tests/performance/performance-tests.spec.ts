import { test, expect } from '@playwright/test';
import { server } from '../setup/mock-server';

test.describe('VexGraph Performance Tests', () => {
  test.beforeAll(async () => {
    server.listen();
  });

  test.afterAll(async () => {
    server.close();
  });

  test.describe('Large Graph Handling', () => {
    test('should handle 1000+ nodes efficiently', async ({ page }) => {
      await page.goto('/ui/graph');
      
      // Generate large dataset
      await page.click('text=Performance Testing');
      await page.click('text=Generate Large Dataset');
      await page.selectOption('select[name="nodeCount"]', '1000');
      await page.selectOption('select[name="edgeCount"]', '2000');
      await page.click('text=Generate');
      
      // Measure loading time
      const startTime = Date.now();
      await page.waitForSelector('.react-flow__node', { timeout: 30000 });
      const loadTime = Date.now() - startTime;
      
      // Should load within 10 seconds
      expect(loadTime).toBeLessThan(10000);
      
      // Verify all nodes are rendered
      const nodeCount = await page.locator('.react-flow__node').count();
      expect(nodeCount).toBe(1000);
      
      // Test zoom performance
      const zoomStartTime = Date.now();
      await page.mouse.wheel(0, -500); // Zoom in
      await page.waitForTimeout(100);
      const zoomTime = Date.now() - zoomStartTime;
      
      // Zoom should be responsive (< 500ms)
      expect(zoomTime).toBeLessThan(500);
      
      // Test pan performance
      const panStartTime = Date.now();
      await page.mouse.move(400, 300);
      await page.mouse.down();
      await page.mouse.move(500, 400);
      await page.mouse.up();
      const panTime = Date.now() - panStartTime;
      
      // Pan should be responsive (< 300ms)
      expect(panTime).toBeLessThan(300);
    });

    test('should handle large edge counts without performance degradation', async ({ page }) => {
      await page.goto('/ui/graph');
      
      // Generate dense graph
      await page.click('text=Performance Testing');
      await page.click('text=Generate Dense Graph');
      await page.selectOption('select[name="nodeCount"]', '500');
      await page.selectOption('select[name="edgeCount"]', '5000'); // 10 edges per node average
      await page.click('text=Generate');
      
      const startTime = Date.now();
      await page.waitForSelector('.react-flow__edge', { timeout: 30000 });
      const loadTime = Date.now() - startTime;
      
      // Should handle dense graphs efficiently
      expect(loadTime).toBeLessThan(15000);
      
      // Verify edge rendering
      const edgeCount = await page.locator('.react-flow__edge').count();
      expect(edgeCount).toBe(5000);
      
      // Test selection performance with many edges
      const selectionStartTime = Date.now();
      await page.click('.react-flow__node >> nth=0');
      await page.waitForSelector('.react-flow__node.selected');
      const selectionTime = Date.now() - selectionStartTime;
      
      expect(selectionTime).toBeLessThan(200);
    });

    test('should maintain performance during real-time updates', async ({ page }) => {
      await page.goto('/ui/graph');
      
      // Start with moderate graph
      await page.click('text=Performance Testing');
      await page.click('text=Generate Moderate Graph');
      await page.selectOption('select[name="nodeCount"]', '200');
      await page.selectOption('select[name="edgeCount"]', '400');
      await page.click('text=Generate');
      
      await page.waitForSelector('.react-flow__node');
      
      // Enable real-time updates
      await page.click('text=Enable Real-time');
      
      // Simulate rapid updates
      const updateTimes: number[] = [];
      
      for (let i = 0; i < 50; i++) {
        const updateStartTime = Date.now();
        
        // Simulate node addition via WebSocket
        await page.evaluate((index) => {
          window.dispatchEvent(new CustomEvent('vexgraph-node-added', {
            detail: {
              id: `perf-node-${index}`,
              node_type: 'File',
              properties: { name: `perf-file-${index}.txt` }
            }
          }));
        }, i);
        
        // Wait for update to be processed
        await page.waitForSelector(`[data-id="perf-node-${i}"]`, { timeout: 1000 });
        
        const updateTime = Date.now() - updateStartTime;
        updateTimes.push(updateTime);
      }
      
      // Calculate average update time
      const avgUpdateTime = updateTimes.reduce((a, b) => a + b, 0) / updateTimes.length;
      
      // Real-time updates should be fast (< 100ms average)
      expect(avgUpdateTime).toBeLessThan(100);
      
      // No update should take longer than 500ms
      const maxUpdateTime = Math.max(...updateTimes);
      expect(maxUpdateTime).toBeLessThan(500);
    });
  });

  test.describe('Query Performance', () => {
    test('should execute complex queries efficiently', async ({ page }) => {
      await page.goto('/ui/graph');
      
      // Generate test graph
      await page.click('text=Performance Testing');
      await page.click('text=Generate Complex Graph');
      await page.selectOption('select[name="nodeCount"]', '1000');
      await page.selectOption('select[name="complexity"]', 'high');
      await page.click('text=Generate');
      
      await page.waitForSelector('.react-flow__node');
      
      // Test different query types
      const queryTests = [
        { algorithm: 'BreadthFirstSearch', maxDepth: '5', expectedTime: 2000 },
        { algorithm: 'DepthFirstSearch', maxDepth: '10', expectedTime: 3000 },
        { algorithm: 'ShortestPath', maxDepth: '15', expectedTime: 5000 },
        { algorithm: 'PageRank', iterations: '10', expectedTime: 8000 }
      ];
      
      for (const queryTest of queryTests) {
        await page.click('text=Query Builder');
        await page.selectOption('select[name="algorithm"]', queryTest.algorithm);
        
        if (queryTest.maxDepth) {
          await page.fill('input[name="maxDepth"]', queryTest.maxDepth);
        }
        if (queryTest.iterations) {
          await page.fill('input[name="iterations"]', queryTest.iterations);
        }
        
        const queryStartTime = Date.now();
        await page.click('text=Execute Query');
        await page.waitForSelector('text=Query Results');
        const queryTime = Date.now() - queryStartTime;
        
        expect(queryTime).toBeLessThan(queryTest.expectedTime);
        
        // Verify results are displayed
        await expect(page.locator('text=/Found \\d+ nodes/')).toBeVisible();
        
        // Clear results for next test
        await page.click('text=Clear Results');
      }
    });

    test('should handle concurrent queries without blocking UI', async ({ page }) => {
      await page.goto('/ui/graph');
      
      // Generate test graph
      await page.click('text=Performance Testing');
      await page.click('text=Generate Test Graph');
      await page.selectOption('select[name="nodeCount"]', '500');
      await page.click('text=Generate');
      
      await page.waitForSelector('.react-flow__node');
      
      // Start multiple concurrent queries
      const queryPromises: Promise<void>[] = [];
      
      for (let i = 0; i < 5; i++) {
        const queryPromise = (async () => {
          await page.click('text=Query Builder');
          await page.selectOption('select[name="algorithm"]', 'BreadthFirstSearch');
          await page.fill('input[name="maxDepth"]', '3');
          await page.click('text=Execute Query');
          await page.waitForSelector('text=Query Results');
        })();
        
        queryPromises.push(queryPromise);
      }
      
      // Test UI responsiveness during concurrent queries
      const uiStartTime = Date.now();
      await page.click('text=Analytics'); // Switch tabs
      await page.waitForSelector('text=Graph Analytics');
      const uiResponseTime = Date.now() - uiStartTime;
      
      // UI should remain responsive (< 500ms)
      expect(uiResponseTime).toBeLessThan(500);
      
      // Wait for all queries to complete
      await Promise.all(queryPromises);
    });

    test('should optimize semantic search performance', async ({ page }) => {
      await page.goto('/ui/graph');
      
      // Generate graph with semantic content
      await page.click('text=Performance Testing');
      await page.click('text=Generate Semantic Graph');
      await page.selectOption('select[name="nodeCount"]', '1000');
      await page.check('input[name="includeVectors"]');
      await page.click('text=Generate');
      
      await page.waitForSelector('.react-flow__node');
      
      // Test semantic search performance
      const searchQueries = [
        'configuration files',
        'database settings',
        'logging parameters',
        'security policies',
        'network configuration'
      ];
      
      for (const query of searchQueries) {
        await page.click('text=Semantic Search');
        await page.fill('input[name="searchQuery"]', query);
        
        const searchStartTime = Date.now();
        await page.click('text=Search');
        await page.waitForSelector('text=Search Results');
        const searchTime = Date.now() - searchStartTime;
        
        // Semantic search should complete within 3 seconds
        expect(searchTime).toBeLessThan(3000);
        
        // Verify results quality
        const resultCount = await page.locator('[data-testid="search-result-item"]').count();
        expect(resultCount).toBeGreaterThan(0);
        
        // Check relevance scores
        const scores = await page.locator('[data-testid="relevance-score"]').allTextContents();
        const numericScores = scores.map(s => parseFloat(s));
        
        // Results should be sorted by relevance
        for (let i = 1; i < numericScores.length; i++) {
          expect(numericScores[i]).toBeLessThanOrEqual(numericScores[i - 1]);
        }
        
        await page.click('text=Clear Search');
      }
    });
  });

  test.describe('Visualization Performance', () => {
    test('should render large graphs smoothly', async ({ page }) => {
      await page.goto('/ui/graph');
      
      // Test different layout algorithms
      const layoutTests = [
        { layout: 'force', nodeCount: 500, expectedTime: 5000 },
        { layout: 'hierarchical', nodeCount: 1000, expectedTime: 8000 },
        { layout: 'circular', nodeCount: 300, expectedTime: 3000 },
        { layout: 'grid', nodeCount: 1000, expectedTime: 2000 }
      ];
      
      for (const layoutTest of layoutTests) {
        await page.click('text=Performance Testing');
        await page.click('text=Generate Layout Test');
        await page.selectOption('select[name="nodeCount"]', layoutTest.nodeCount.toString());
        await page.selectOption('select[name="layout"]', layoutTest.layout);
        await page.click('text=Generate');
        
        const layoutStartTime = Date.now();
        await page.waitForSelector('.react-flow__node');
        
        // Wait for layout to stabilize
        await page.waitForTimeout(1000);
        const layoutTime = Date.now() - layoutStartTime;
        
        expect(layoutTime).toBeLessThan(layoutTest.expectedTime);
        
        // Test animation performance
        await page.click('text=Animate Layout');
        
        const animationStartTime = Date.now();
        await page.waitForSelector('.react-flow__node.animating');
        await page.waitForSelector('.react-flow__node:not(.animating)', { timeout: 10000 });
        const animationTime = Date.now() - animationStartTime;
        
        // Animation should complete within reasonable time
        expect(animationTime).toBeLessThan(layoutTest.expectedTime);
        
        await page.click('text=Clear Graph');
      }
    });

    test('should handle zoom and pan operations efficiently', async ({ page }) => {
      await page.goto('/ui/graph');
      
      // Generate large graph for stress testing
      await page.click('text=Performance Testing');
      await page.click('text=Generate Large Graph');
      await page.selectOption('select[name="nodeCount"]', '2000');
      await page.click('text=Generate');
      
      await page.waitForSelector('.react-flow__node');
      
      // Test zoom performance at different levels
      const zoomLevels = [0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0];
      
      for (const zoomLevel of zoomLevels) {
        const zoomStartTime = Date.now();
        
        await page.evaluate((zoom) => {
          const reactFlowInstance = (window as any).reactFlowInstance;
          if (reactFlowInstance) {
            reactFlowInstance.setViewport({ x: 0, y: 0, zoom });
          }
        }, zoomLevel);
        
        await page.waitForTimeout(100); // Allow zoom to complete
        const zoomTime = Date.now() - zoomStartTime;
        
        // Zoom should be responsive at all levels
        expect(zoomTime).toBeLessThan(200);
      }
      
      // Test pan performance
      const panOperations = [
        { x: 100, y: 100 },
        { x: -200, y: 150 },
        { x: 300, y: -100 },
        { x: -150, y: -200 }
      ];
      
      for (const pan of panOperations) {
        const panStartTime = Date.now();
        
        await page.evaluate((panOffset) => {
          const reactFlowInstance = (window as any).reactFlowInstance;
          if (reactFlowInstance) {
            const currentViewport = reactFlowInstance.getViewport();
            reactFlowInstance.setViewport({
              x: currentViewport.x + panOffset.x,
              y: currentViewport.y + panOffset.y,
              zoom: currentViewport.zoom
            });
          }
        }, pan);
        
        await page.waitForTimeout(50);
        const panTime = Date.now() - panStartTime;
        
        expect(panTime).toBeLessThan(100);
      }
    });

    test('should maintain performance during node selection', async ({ page }) => {
      await page.goto('/ui/graph');
      
      // Generate graph for selection testing
      await page.click('text=Performance Testing');
      await page.click('text=Generate Selection Test');
      await page.selectOption('select[name="nodeCount"]', '1000');
      await page.click('text=Generate');
      
      await page.waitForSelector('.react-flow__node');
      
      // Test single node selection performance
      const singleSelectionTimes: number[] = [];
      
      for (let i = 0; i < 20; i++) {
        const selectionStartTime = Date.now();
        await page.click(`.react-flow__node >> nth=${i}`);
        await page.waitForSelector('.react-flow__node.selected');
        const selectionTime = Date.now() - selectionStartTime;
        
        singleSelectionTimes.push(selectionTime);
      }
      
      const avgSingleSelectionTime = singleSelectionTimes.reduce((a, b) => a + b, 0) / singleSelectionTimes.length;
      expect(avgSingleSelectionTime).toBeLessThan(50);
      
      // Test multi-selection performance
      await page.keyboard.down('Control');
      
      const multiSelectionStartTime = Date.now();
      for (let i = 0; i < 50; i++) {
        await page.click(`.react-flow__node >> nth=${i}`);
      }
      await page.keyboard.up('Control');
      
      const multiSelectionTime = Date.now() - multiSelectionStartTime;
      expect(multiSelectionTime).toBeLessThan(2000);
      
      // Verify all nodes are selected
      const selectedCount = await page.locator('.react-flow__node.selected').count();
      expect(selectedCount).toBe(50);
    });
  });

  test.describe('Memory Usage', () => {
    test('should not have memory leaks during extended usage', async ({ page }) => {
      await page.goto('/ui/graph');
      
      // Monitor memory usage
      const getMemoryUsage = async () => {
        return await page.evaluate(() => {
          return (performance as any).memory ? {
            usedJSHeapSize: (performance as any).memory.usedJSHeapSize,
            totalJSHeapSize: (performance as any).memory.totalJSHeapSize,
            jsHeapSizeLimit: (performance as any).memory.jsHeapSizeLimit
          } : null;
        });
      };
      
      const initialMemory = await getMemoryUsage();
      
      // Perform memory-intensive operations
      for (let cycle = 0; cycle < 10; cycle++) {
        // Generate and clear graphs repeatedly
        await page.click('text=Performance Testing');
        await page.click('text=Generate Large Graph');
        await page.selectOption('select[name="nodeCount"]', '500');
        await page.click('text=Generate');
        
        await page.waitForSelector('.react-flow__node');
        await page.waitForTimeout(1000);
        
        await page.click('text=Clear Graph');
        await page.waitForTimeout(500);
        
        // Force garbage collection if available
        await page.evaluate(() => {
          if ((window as any).gc) {
            (window as any).gc();
          }
        });
      }
      
      const finalMemory = await getMemoryUsage();
      
      if (initialMemory && finalMemory) {
        const memoryIncrease = finalMemory.usedJSHeapSize - initialMemory.usedJSHeapSize;
        const memoryIncreasePercent = (memoryIncrease / initialMemory.usedJSHeapSize) * 100;
        
        // Memory increase should be reasonable (< 50% after 10 cycles)
        expect(memoryIncreasePercent).toBeLessThan(50);
      }
    });

    test('should handle large datasets without excessive memory usage', async ({ page }) => {
      await page.goto('/ui/graph');
      
      const getMemoryUsage = async () => {
        return await page.evaluate(() => {
          return (performance as any).memory ? {
            usedJSHeapSize: (performance as any).memory.usedJSHeapSize,
            totalJSHeapSize: (performance as any).memory.totalJSHeapSize
          } : null;
        });
      };
      
      const baselineMemory = await getMemoryUsage();
      
      // Load progressively larger datasets
      const datasetSizes = [100, 500, 1000, 2000];
      
      for (const size of datasetSizes) {
        await page.click('text=Performance Testing');
        await page.click('text=Generate Memory Test');
        await page.selectOption('select[name="nodeCount"]', size.toString());
        await page.click('text=Generate');
        
        await page.waitForSelector('.react-flow__node');
        await page.waitForTimeout(2000); // Allow rendering to complete
        
        const currentMemory = await getMemoryUsage();
        
        if (baselineMemory && currentMemory) {
          const memoryPerNode = (currentMemory.usedJSHeapSize - baselineMemory.usedJSHeapSize) / size;
          
          // Memory per node should be reasonable (< 10KB per node)
          expect(memoryPerNode).toBeLessThan(10240);
        }
        
        await page.click('text=Clear Graph');
        await page.waitForTimeout(1000);
      }
    });
  });

  test.describe('Network Performance', () => {
    test('should handle slow network conditions gracefully', async ({ page }) => {
      // Simulate slow network
      await page.route('**/*', async (route) => {
        await new Promise(resolve => setTimeout(resolve, 100)); // 100ms delay
        await route.continue();
      });
      
      await page.goto('/ui/graph');
      
      // Test loading with network delay
      const loadStartTime = Date.now();
      await page.waitForSelector('[data-testid="graph-page-title"]');
      const loadTime = Date.now() - loadStartTime;
      
      // Should handle slow network gracefully
      expect(loadTime).toBeLessThan(10000);
      
      // Test operations with network delay
      await page.click('text=Create Node');
      await page.fill('input[name="nodeType"]', 'File');
      await page.fill('input[name="properties.name"]', 'slow-network-test');
      
      const createStartTime = Date.now();
      await page.click('button[type="submit"]');
      await expect(page.locator('text=Node created successfully')).toBeVisible();
      const createTime = Date.now() - createStartTime;
      
      // Should complete within reasonable time even with network delay
      expect(createTime).toBeLessThan(5000);
    });

    test('should batch API requests efficiently', async ({ page }) => {
      let requestCount = 0;
      
      // Monitor API requests
      await page.route('/api/v1/vexgraph/**', async (route) => {
        requestCount++;
        await route.continue();
      });
      
      await page.goto('/ui/graph');
      
      // Perform operations that could trigger multiple requests
      await page.click('text=Performance Testing');
      await page.click('text=Generate Batch Test');
      await page.selectOption('select[name="nodeCount"]', '100');
      await page.selectOption('select[name="edgeCount"]', '200');
      await page.click('text=Generate');
      
      await page.waitForSelector('.react-flow__node');
      
      // Should use batch operations to minimize requests
      // Creating 100 nodes + 200 edges should use batch endpoints (2-3 requests)
      // instead of individual requests (300 requests)
      expect(requestCount).toBeLessThan(10);
    });
  });
});