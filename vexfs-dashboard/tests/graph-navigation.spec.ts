import { test, expect } from '@playwright/test';

test.describe('Graph Navigation', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/ui');
  });

  test('should display Graph navigation item in sidebar', async ({ page }) => {
    // Check if the Graph navigation item is visible
    const graphNavItem = page.locator('[data-testid="nav-graph"]');
    await expect(graphNavItem).toBeVisible();
    
    // Check the text content
    await expect(graphNavItem).toContainText('Graph');
  });

  test('should navigate to Graph page when clicking Graph nav item', async ({ page }) => {
    // Click on the Graph navigation item
    const graphNavItem = page.locator('[data-testid="nav-graph"]');
    await graphNavItem.click();

    // Check if we're on the graph page
    await expect(page).toHaveURL('/ui/graph');
    
    // Check if the Graph page content is displayed
    await expect(page.locator('[data-testid="graph-page-title"]')).toContainText('VexGraph');
    await expect(page.locator('text=Explore and analyze the filesystem graph structure')).toBeVisible();
  });

  test('should highlight Graph nav item when on Graph page', async ({ page }) => {
    // Navigate to Graph page
    await page.goto('/ui/graph');

    // Check if the Graph navigation item has aria-current="page"
    const graphNavItem = page.locator('[data-testid="nav-graph"]');
    await expect(graphNavItem).toHaveAttribute('aria-current', 'page');
  });

  test('should display Graph page features correctly', async ({ page }) => {
    // Navigate to Graph page
    await page.goto('/ui/graph');

    // Check for key feature cards
    await expect(page.locator('text=Graph Visualization')).toBeVisible();
    await expect(page.locator('text=Graph Search')).toBeVisible();
    await expect(page.locator('text=Graph Analytics')).toBeVisible();
    await expect(page.locator('text=Graph Configuration')).toBeVisible();

    // Check for status banner
    await expect(page.locator('[data-testid="graph-status-banner"]')).toBeVisible();
    await expect(page.locator('text=Graph Interface Ready')).toBeVisible();
    await expect(page.locator('text=Coming Soon')).toBeVisible();

    // Check for technical overview
    await expect(page.locator('text=Technical Overview')).toBeVisible();
    await expect(page.locator('text=Node Management')).toBeVisible();
    await expect(page.locator('text=Edge Relationships')).toBeVisible();
  });

  test('should have proper accessibility attributes', async ({ page }) => {
    // Navigate to Graph page
    await page.goto('/ui/graph');

    // Check for proper heading hierarchy
    const h1 = page.locator('h1');
    await expect(h1).toHaveText('VexGraph');

    // Check for proper semantic structure
    const main = page.locator('[role="main"]');
    await expect(main).toBeVisible();
    
    // Check aria-label on main content
    await expect(main).toHaveAttribute('aria-label', 'VexGraph Dashboard');
  });

  test('should support keyboard navigation', async ({ page }) => {
    // Start from the beginning and tab to the Graph navigation
    await page.keyboard.press('Tab');
    
    // Navigate through the sidebar items until we find Graph
    let attempts = 0;
    while (attempts < 15) {
      const focused = page.locator(':focus');
      const testId = await focused.getAttribute('data-testid');
      
      if (testId === 'nav-graph') {
        // Press Enter to navigate to Graph page
        await page.keyboard.press('Enter');
        break;
      }
      
      await page.keyboard.press('Tab');
      attempts++;
    }

    // Verify we're on the Graph page
    await expect(page).toHaveURL('/ui/graph');
  });

  test('should have proper ARIA labels for navigation', async ({ page }) => {
    // Check the Graph navigation item has proper ARIA label
    const graphNavItem = page.locator('[data-testid="nav-graph"]');
    await expect(graphNavItem).toHaveAttribute('aria-label', 'Navigate to Graph');
  });

  test('should display AccountTree icon for Graph navigation', async ({ page }) => {
    // Check if the Graph navigation item contains the AccountTree icon
    const graphNavItem = page.locator('[data-testid="nav-graph"]');
    const icon = graphNavItem.locator('svg');
    await expect(icon).toBeVisible();
  });
});