import { test, expect } from '@playwright/test';
import { server, resetMockData } from './setup/mock-server';

test.describe('Collections Page', () => {
  test.beforeAll(async () => {
    // Start the mock server
    server.listen();
  });

  test.afterAll(async () => {
    // Clean up
    server.close();
  });

  test.beforeEach(async () => {
    // Reset mock data before each test
    resetMockData();
  });

  test('should display collections list', async ({ page }) => {
    await page.goto('/collections');
    
    // Wait for the page to load
    await expect(page.locator('h1')).toContainText('Collections');
    
    // Check if collections are displayed
    await expect(page.locator('[data-testid="collections-table"]')).toBeVisible();
    
    // Check if mock collections are displayed
    await expect(page.locator('text=Test Collection 1')).toBeVisible();
    await expect(page.locator('text=Test Collection 2')).toBeVisible();
    await expect(page.locator('text=Large Collection')).toBeVisible();
  });

  test('should search collections', async ({ page }) => {
    await page.goto('/collections');
    
    // Wait for collections to load
    await expect(page.locator('text=Test Collection 1')).toBeVisible();
    
    // Search for a specific collection
    const searchInput = page.locator('input[placeholder*="Search collections"]');
    await searchInput.fill('Large');
    
    // Should only show the Large Collection
    await expect(page.locator('text=Large Collection')).toBeVisible();
    await expect(page.locator('text=Test Collection 1')).not.toBeVisible();
    await expect(page.locator('text=Test Collection 2')).not.toBeVisible();
    
    // Clear search
    await searchInput.clear();
    
    // All collections should be visible again
    await expect(page.locator('text=Test Collection 1')).toBeVisible();
    await expect(page.locator('text=Test Collection 2')).toBeVisible();
    await expect(page.locator('text=Large Collection')).toBeVisible();
  });

  test('should sort collections by name', async ({ page }) => {
    await page.goto('/collections');
    
    // Wait for collections to load
    await expect(page.locator('text=Test Collection 1')).toBeVisible();
    
    // Click on name column header to sort
    await page.locator('text=Name').click();
    
    // Check if collections are sorted alphabetically
    const collectionNames = await page.locator('[data-testid="collection-name"]').allTextContents();
    const sortedNames = [...collectionNames].sort();
    expect(collectionNames).toEqual(sortedNames);
  });

  test('should open create collection dialog', async ({ page }) => {
    await page.goto('/collections');
    
    // Click create collection button
    await page.locator('[data-testid="create-collection-button"]').click();
    
    // Check if dialog is open
    await expect(page.locator('[data-testid="create-collection-dialog"]')).toBeVisible();
    await expect(page.locator('text=Create New Collection')).toBeVisible();
    
    // Check form fields
    await expect(page.locator('input[name="name"]')).toBeVisible();
    await expect(page.locator('input[name="description"]')).toBeVisible();
    await expect(page.locator('input[name="vectorSize"]')).toBeVisible();
    await expect(page.locator('select[name="distance"]')).toBeVisible();
  });

  test('should create a new collection', async ({ page }) => {
    await page.goto('/collections');
    
    // Open create dialog
    await page.locator('[data-testid="create-collection-button"]').click();
    
    // Fill form
    await page.locator('input[name="name"]').fill('New Test Collection');
    await page.locator('input[name="description"]').fill('A collection created by tests');
    await page.locator('input[name="vectorSize"]').fill('512');
    await page.locator('select[name="distance"]').selectOption('cosine');
    
    // Submit form
    await page.locator('[data-testid="create-collection-submit"]').click();
    
    // Check if collection was created
    await expect(page.locator('text=New Test Collection')).toBeVisible();
    await expect(page.locator('text=A collection created by tests')).toBeVisible();
  });

  test('should view collection details', async ({ page }) => {
    await page.goto('/collections');
    
    // Wait for collections to load
    await expect(page.locator('text=Test Collection 1')).toBeVisible();
    
    // Click view details button for first collection
    await page.locator('[data-testid="view-collection-collection-1"]').click();
    
    // Check if details page is loaded
    await expect(page.locator('h1')).toContainText('Test Collection 1');
    await expect(page.locator('text=384')).toBeVisible(); // Vector size
    await expect(page.locator('text=cosine')).toBeVisible(); // Distance metric
    await expect(page.locator('text=1000')).toBeVisible(); // Points count
  });

  test('should edit collection', async ({ page }) => {
    await page.goto('/collections');
    
    // Wait for collections to load
    await expect(page.locator('text=Test Collection 1')).toBeVisible();
    
    // Click edit button
    await page.locator('[data-testid="edit-collection-collection-1"]').click();
    
    // Check if edit dialog is open
    await expect(page.locator('[data-testid="edit-collection-dialog"]')).toBeVisible();
    
    // Update description
    const descriptionInput = page.locator('input[name="description"]');
    await descriptionInput.clear();
    await descriptionInput.fill('Updated description');
    
    // Submit changes
    await page.locator('[data-testid="edit-collection-submit"]').click();
    
    // Check if changes are reflected
    await expect(page.locator('text=Updated description')).toBeVisible();
  });

  test('should delete collection', async ({ page }) => {
    await page.goto('/collections');
    
    // Wait for collections to load
    await expect(page.locator('text=Test Collection 1')).toBeVisible();
    
    // Click delete button
    await page.locator('[data-testid="delete-collection-collection-1"]').click();
    
    // Check if confirmation dialog is open
    await expect(page.locator('[data-testid="delete-confirmation-dialog"]')).toBeVisible();
    await expect(page.locator('text=Are you sure')).toBeVisible();
    
    // Confirm deletion
    await page.locator('[data-testid="confirm-delete"]').click();
    
    // Check if collection is removed
    await expect(page.locator('text=Test Collection 1')).not.toBeVisible();
  });

  test('should handle pagination', async ({ page }) => {
    await page.goto('/collections');
    
    // Wait for collections to load
    await expect(page.locator('text=Test Collection 1')).toBeVisible();
    
    // Check pagination controls
    await expect(page.locator('[data-testid="pagination"]')).toBeVisible();
    
    // Change rows per page
    await page.locator('[data-testid="rows-per-page-select"]').selectOption('5');
    
    // Check if only 5 rows are displayed (if we have more than 5 collections)
    const visibleRows = await page.locator('[data-testid="collection-row"]').count();
    expect(visibleRows).toBeLessThanOrEqual(5);
  });

  test('should handle empty state', async ({ page }) => {
    // Mock empty collections response
    await page.route('/api/v1/collections', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ collections: [] }),
      });
    });
    
    await page.goto('/collections');
    
    // Check empty state
    await expect(page.locator('text=No Collections Found')).toBeVisible();
    await expect(page.locator('text=Create your first collection')).toBeVisible();
  });

  test('should handle loading state', async ({ page }) => {
    // Mock slow response
    await page.route('/api/v1/collections', async route => {
      await new Promise(resolve => setTimeout(resolve, 1000));
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ collections: [] }),
      });
    });
    
    await page.goto('/collections');
    
    // Check loading state
    await expect(page.locator('[data-testid="loading-spinner"]')).toBeVisible();
  });

  test('should handle error state', async ({ page }) => {
    // Mock error response
    await page.route('/api/v1/collections', async route => {
      await route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Internal server error' }),
      });
    });
    
    await page.goto('/collections');
    
    // Check error state
    await expect(page.locator('text=Failed to load collections')).toBeVisible();
    await expect(page.locator('text=Internal server error')).toBeVisible();
  });

  test('should be responsive on mobile', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    
    await page.goto('/collections');
    
    // Wait for collections to load
    await expect(page.locator('text=Test Collection 1')).toBeVisible();
    
    // Check if mobile layout is applied
    // This would depend on your responsive design implementation
    await expect(page.locator('[data-testid="mobile-collections-view"]')).toBeVisible();
  });
});