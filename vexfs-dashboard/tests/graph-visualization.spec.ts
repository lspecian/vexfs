import { test, expect } from '@playwright/test';

test.describe('Graph Visualization Component', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/graph');
  });

  test('should display graph page title', async ({ page }) => {
    await expect(page.getByTestId('graph-page-title')).toHaveText('VexGraph');
  });

  test('should show demo when no backend data available', async ({ page }) => {
    // Wait for the page to load
    await page.waitForLoadState('networkidle');
    
    // Should show demo section
    await expect(page.getByText('Graph Visualization Demo')).toBeVisible();
    await expect(page.getByText('Generate New')).toBeVisible();
    await expect(page.getByText('Animate')).toBeVisible();
  });

  test('should generate new graph data', async ({ page }) => {
    // Wait for demo to load
    await page.waitForSelector('text=Graph Visualization Demo');
    
    // Get initial node count
    const initialNodeCount = await page.textContent('[data-testid="node-count"]') || 
                            await page.textContent('text=/\\d+ Nodes/');
    
    // Click generate new button
    await page.click('text=Generate New');
    
    // Wait a moment for the graph to update
    await page.waitForTimeout(500);
    
    // Verify the graph has been regenerated (nodes/edges might be different)
    await expect(page.locator('text=/\\d+ Nodes/')).toBeVisible();
    await expect(page.locator('text=/\\d+ Edges/')).toBeVisible();
  });

  test('should toggle animation', async ({ page }) => {
    // Wait for demo to load
    await page.waitForSelector('text=Graph Visualization Demo');
    
    // Initially should show "Animate" button
    await expect(page.getByText('Animate')).toBeVisible();
    
    // Click animate button
    await page.click('text=Animate');
    
    // Should now show "Stop" button
    await expect(page.getByText('Stop')).toBeVisible();
    
    // Click stop button
    await page.click('text=Stop');
    
    // Should show "Animate" button again
    await expect(page.getByText('Animate')).toBeVisible();
  });

  test('should display graph visualization controls', async ({ page }) => {
    // Wait for demo to load
    await page.waitForSelector('text=Graph Visualization Demo');
    
    // Check for React Flow controls (zoom, fit, etc.)
    await expect(page.locator('.react-flow__controls')).toBeVisible();
    
    // Check for minimap
    await expect(page.locator('.react-flow__minimap')).toBeVisible();
    
    // Check for background
    await expect(page.locator('.react-flow__background')).toBeVisible();
  });

  test('should handle node selection', async ({ page }) => {
    // Wait for demo to load
    await page.waitForSelector('text=Graph Visualization Demo');
    
    // Wait for graph nodes to be rendered
    await page.waitForSelector('.react-flow__node', { timeout: 5000 });
    
    // Click on a node
    const firstNode = page.locator('.react-flow__node').first();
    await firstNode.click();
    
    // Should show selection details
    await expect(page.getByText('Selection Details')).toBeVisible();
    await expect(page.getByText('Selected Nodes')).toBeVisible();
  });

  test('should handle edge selection', async ({ page }) => {
    // Wait for demo to load
    await page.waitForSelector('text=Graph Visualization Demo');
    
    // Wait for graph edges to be rendered
    await page.waitForSelector('.react-flow__edge', { timeout: 5000 });
    
    // Click on an edge
    const firstEdge = page.locator('.react-flow__edge').first();
    await firstEdge.click();
    
    // Should show selection details
    await expect(page.getByText('Selection Details')).toBeVisible();
    await expect(page.getByText('Selected Edges')).toBeVisible();
  });

  test('should display node and edge counts', async ({ page }) => {
    // Wait for demo to load
    await page.waitForSelector('text=Graph Visualization Demo');
    
    // Should display node and edge counts
    await expect(page.locator('text=/\\d+ Nodes/')).toBeVisible();
    await expect(page.locator('text=/\\d+ Edges/')).toBeVisible();
  });

  test('should show demo info alert', async ({ page }) => {
    // Wait for demo to load
    await page.waitForSelector('text=Graph Visualization Demo');
    
    // Should show demo features info
    await expect(page.getByText('Demo Features:')).toBeVisible();
    await expect(page.getByText(/Try clicking nodes\/edges to select them/)).toBeVisible();
  });

  test('should handle zoom controls', async ({ page }) => {
    // Wait for demo to load
    await page.waitForSelector('text=Graph Visualization Demo');
    
    // Wait for React Flow controls
    await page.waitForSelector('.react-flow__controls');
    
    // Test zoom in button
    const zoomInButton = page.locator('.react-flow__controls button[title*="zoom in"], .react-flow__controls button:has-text("+")');
    if (await zoomInButton.count() > 0) {
      await zoomInButton.click();
    }
    
    // Test zoom out button
    const zoomOutButton = page.locator('.react-flow__controls button[title*="zoom out"], .react-flow__controls button:has-text("-")');
    if (await zoomOutButton.count() > 0) {
      await zoomOutButton.click();
    }
    
    // Test fit view button
    const fitViewButton = page.locator('.react-flow__controls button[title*="fit view"]');
    if (await fitViewButton.count() > 0) {
      await fitViewButton.click();
    }
  });

  test('should handle double-click events', async ({ page }) => {
    // Wait for demo to load
    await page.waitForSelector('text=Graph Visualization Demo');
    
    // Wait for graph nodes to be rendered
    await page.waitForSelector('.react-flow__node', { timeout: 5000 });
    
    // Set up console listener to catch double-click events
    const consoleMessages: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'log') {
        consoleMessages.push(msg.text());
      }
    });
    
    // Double-click on a node
    const firstNode = page.locator('.react-flow__node').first();
    await firstNode.dblclick();
    
    // Wait a moment for console message
    await page.waitForTimeout(500);
    
    // Check if double-click was logged
    const hasDoubleClickLog = consoleMessages.some(msg => 
      msg.includes('Demo: Node double-clicked')
    );
    expect(hasDoubleClickLog).toBeTruthy();
  });

  test('should display different node types', async ({ page }) => {
    // Wait for demo to load
    await page.waitForSelector('text=Graph Visualization Demo');
    
    // Wait for graph nodes to be rendered
    await page.waitForSelector('.react-flow__node', { timeout: 5000 });
    
    // Should have nodes with different types
    const nodes = page.locator('.react-flow__node');
    const nodeCount = await nodes.count();
    expect(nodeCount).toBeGreaterThan(0);
    
    // Click on a node to see selection details
    await nodes.first().click();
    
    // Should show node type in selection details
    await expect(page.getByText(/File|Directory|Symlink|Device/)).toBeVisible();
  });

  test('should handle responsive layout', async ({ page }) => {
    // Test desktop view
    await page.setViewportSize({ width: 1200, height: 800 });
    await page.waitForSelector('text=Graph Visualization Demo');
    
    // Should show all controls
    await expect(page.getByText('Generate New')).toBeVisible();
    await expect(page.getByText('Animate')).toBeVisible();
    
    // Test mobile view
    await page.setViewportSize({ width: 375, height: 667 });
    await page.waitForTimeout(500);
    
    // Controls should still be visible but may be stacked
    await expect(page.getByText('Generate New')).toBeVisible();
    await expect(page.getByText('Animate')).toBeVisible();
  });

  test('should maintain graph state during interactions', async ({ page }) => {
    // Wait for demo to load
    await page.waitForSelector('text=Graph Visualization Demo');
    
    // Get initial counts
    const initialNodeText = await page.textContent('text=/\\d+ Nodes/');
    const initialEdgeText = await page.textContent('text=/\\d+ Edges/');
    
    // Interact with zoom controls
    await page.waitForSelector('.react-flow__controls');
    const zoomInButton = page.locator('.react-flow__controls button').first();
    await zoomInButton.click();
    
    // Counts should remain the same
    await expect(page.locator(`text=${initialNodeText}`)).toBeVisible();
    await expect(page.locator(`text=${initialEdgeText}`)).toBeVisible();
  });

  test('should clear selection when generating new graph', async ({ page }) => {
    // Wait for demo to load
    await page.waitForSelector('text=Graph Visualization Demo');
    
    // Wait for nodes and select one
    await page.waitForSelector('.react-flow__node', { timeout: 5000 });
    await page.locator('.react-flow__node').first().click();
    
    // Should show selection
    await expect(page.getByText('Selection Details')).toBeVisible();
    
    // Generate new graph
    await page.click('text=Generate New');
    await page.waitForTimeout(500);
    
    // Selection should be cleared
    await expect(page.getByText('Selection Details')).not.toBeVisible();
  });
});