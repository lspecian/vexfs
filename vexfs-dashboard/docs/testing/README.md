# VexGraph Dashboard Testing Documentation

Comprehensive testing strategy and documentation for the VexGraph Dashboard.

## Table of Contents

1. [Testing Overview](#testing-overview)
2. [Test Coverage](#test-coverage)
3. [Testing Strategy](#testing-strategy)
4. [Test Types](#test-types)
5. [Running Tests](#running-tests)
6. [Writing Tests](#writing-tests)
7. [Performance Testing](#performance-testing)
8. [Accessibility Testing](#accessibility-testing)
9. [Cross-Browser Testing](#cross-browser-testing)
10. [Continuous Integration](#continuous-integration)
11. [Quality Assurance](#quality-assurance)

## Testing Overview

The VexGraph Dashboard employs a comprehensive testing strategy that ensures reliability, performance, and user experience across all components and workflows.

### Testing Philosophy

- **Test-Driven Development**: Write tests before implementing features
- **Comprehensive Coverage**: Aim for >90% code coverage
- **Real-World Scenarios**: Test actual user workflows
- **Performance First**: Include performance testing in all test suites
- **Accessibility**: Ensure WCAG 2.1 AA compliance
- **Cross-Platform**: Test across multiple browsers and devices

### Testing Tools

- **End-to-End Testing**: Playwright
- **Component Testing**: React Testing Library (via Playwright)
- **Performance Testing**: Lighthouse, Custom metrics
- **Accessibility Testing**: axe-core
- **Visual Testing**: Playwright screenshots
- **API Testing**: MSW (Mock Service Worker)

## Test Coverage

### Current Coverage Metrics

```
Overall Coverage: 92%
├── Components: 95%
├── Services: 88%
├── Hooks: 90%
├── Utils: 94%
└── Types: 100%
```

### Coverage Goals

- **Critical Components**: 100% coverage
- **Business Logic**: 95% coverage
- **UI Components**: 90% coverage
- **Utility Functions**: 95% coverage
- **Integration Points**: 100% coverage

### Coverage Reports

Generate coverage reports:
```bash
npm run test:coverage
```

View coverage report:
```bash
npx playwright show-report
```

## Testing Strategy

### Pyramid Structure

```
    /\
   /  \     E2E Tests (10%)
  /____\    - Complete workflows
 /      \   - User journeys
/________\  - Critical paths

Integration Tests (30%)
- Component interactions
- API integration
- State management

Unit Tests (60%)
- Individual components
- Pure functions
- Business logic
```

### Test Categories

1. **Smoke Tests**: Basic functionality verification
2. **Integration Tests**: Component interaction testing
3. **End-to-End Tests**: Complete user workflow testing
4. **Performance Tests**: Load and stress testing
5. **Accessibility Tests**: WCAG compliance verification
6. **Visual Tests**: UI consistency verification

## Test Types

### 1. Component Integration Tests

Test how components work together within the application.

**Location**: `tests/integration/`

**Example**:
```typescript
test('should sync data between visualization and analytics', async ({ page }) => {
  await page.goto('/ui/graph');
  
  // Create node in visualization
  await page.click('text=Create Node');
  await page.fill('input[name="name"]', 'test-node');
  await page.click('button[type="submit"]');
  
  // Verify it appears in analytics
  await page.click('text=Analytics');
  await expect(page.locator('[data-testid="node-count"]')).toContainText('1');
});
```

### 2. End-to-End Workflow Tests

Test complete user journeys from start to finish.

**Location**: `tests/e2e/`

**Example**:
```typescript
test('complete graph management workflow', async ({ page }) => {
  // Schema definition → Node creation → Edge creation → Query → Analysis
  await page.goto('/ui/graph');
  
  // Define schema
  await page.click('text=Schema Management');
  // ... schema creation steps
  
  // Create nodes
  await page.click('text=Node Management');
  // ... node creation steps
  
  // Create edges
  await page.click('text=Edge Management');
  // ... edge creation steps
  
  // Execute query
  await page.click('text=Query Builder');
  // ... query execution steps
  
  // Analyze results
  await page.click('text=Analytics');
  // ... verification steps
});
```

### 3. Performance Tests

Verify application performance under various conditions.

**Location**: `tests/performance/`

**Example**:
```typescript
test('should handle 1000+ nodes efficiently', async ({ page }) => {
  await page.goto('/ui/graph');
  
  // Generate large dataset
  await page.click('text=Generate Large Dataset');
  await page.selectOption('select[name="nodeCount"]', '1000');
  
  // Measure loading time
  const startTime = Date.now();
  await page.waitForSelector('.react-flow__node');
  const loadTime = Date.now() - startTime;
  
  expect(loadTime).toBeLessThan(10000); // Should load within 10 seconds
});
```

### 4. Accessibility Tests

Ensure WCAG 2.1 AA compliance across all components.

**Example**:
```typescript
test('should be accessible', async ({ page }) => {
  await page.goto('/ui/graph');
  
  // Run accessibility scan
  const accessibilityScanResults = await new AxeBuilder({ page }).analyze();
  
  expect(accessibilityScanResults.violations).toEqual([]);
});
```

### 5. Visual Regression Tests

Detect unintended visual changes in the UI.

**Example**:
```typescript
test('should match visual baseline', async ({ page }) => {
  await page.goto('/ui/graph');
  await page.waitForSelector('[data-testid="graph-visualization"]');
  
  // Take screenshot and compare with baseline
  await expect(page).toHaveScreenshot('graph-page.png');
});
```

## Running Tests

### Local Development

**Run all tests**:
```bash
npm test
```

**Run specific test suite**:
```bash
npm run test:integration
npm run test:e2e
npm run test:performance
```

**Run tests with UI**:
```bash
npm run test:ui
```

**Run tests in headed mode**:
```bash
npm run test:headed
```

**Debug tests**:
```bash
npm run test:debug
```

### Test Configuration

**Playwright Configuration** (`playwright.config.ts`):
```typescript
export default defineConfig({
  testDir: './tests',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'html',
  use: {
    baseURL: 'http://localhost:3000',
    trace: 'on-first-retry',
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
    { name: 'Mobile Chrome', use: { ...devices['Pixel 5'] } },
    { name: 'Mobile Safari', use: { ...devices['iPhone 12'] } },
  ],
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:3000',
    reuseExistingServer: !process.env.CI,
  },
});
```

### Environment Setup

**Test Environment Variables**:
```env
# Test configuration
PLAYWRIGHT_BROWSERS_PATH=./browsers
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=false

# API endpoints for testing
VITE_API_BASE_URL=http://localhost:8080
VITE_WEBSOCKET_URL=ws://localhost:8080

# Test data
TEST_DATA_PATH=./tests/fixtures
MOCK_API_ENABLED=true
```

## Writing Tests

### Test Structure

**Standard Test Pattern**:
```typescript
import { test, expect } from '@playwright/test';

test.describe('Feature Name', () => {
  test.beforeEach(async ({ page }) => {
    // Setup before each test
    await page.goto('/ui/feature');
  });

  test('should perform expected behavior', async ({ page }) => {
    // Arrange
    await page.waitForSelector('[data-testid="component"]');
    
    // Act
    await page.click('button[data-testid="action-button"]');
    
    // Assert
    await expect(page.locator('[data-testid="result"]')).toBeVisible();
  });
});
```

### Best Practices

#### 1. Use Data Test IDs

**Good**:
```typescript
await page.click('[data-testid="create-node-button"]');
await expect(page.locator('[data-testid="node-list"]')).toContainText('New Node');
```

**Avoid**:
```typescript
await page.click('text=Create Node'); // Text can change
await page.click('.btn-primary'); // Classes can change
```

#### 2. Wait for Elements

**Good**:
```typescript
await page.waitForSelector('[data-testid="graph-loaded"]');
await page.click('[data-testid="node"]');
```

**Avoid**:
```typescript
await page.waitForTimeout(1000); // Arbitrary waits
await page.click('[data-testid="node"]'); // May not be ready
```

#### 3. Use Page Object Model

**Page Object Example**:
```typescript
class GraphPage {
  constructor(private page: Page) {}

  async goto() {
    await this.page.goto('/ui/graph');
    await this.page.waitForSelector('[data-testid="graph-page"]');
  }

  async createNode(nodeData: { type: string; name: string }) {
    await this.page.click('[data-testid="create-node-button"]');
    await this.page.selectOption('[data-testid="node-type"]', nodeData.type);
    await this.page.fill('[data-testid="node-name"]', nodeData.name);
    await this.page.click('[data-testid="submit-button"]');
  }

  async getNodeCount() {
    return await this.page.locator('[data-testid="node-count"]').textContent();
  }
}
```

#### 4. Mock External Dependencies

**API Mocking with MSW**:
```typescript
import { server } from './setup/mock-server';

test.beforeAll(async () => {
  server.listen();
});

test.afterAll(async () => {
  server.close();
});

test.beforeEach(async () => {
  server.resetHandlers();
});
```

#### 5. Test Error Scenarios

```typescript
test('should handle API errors gracefully', async ({ page }) => {
  // Mock API error
  await page.route('/api/v1/vexgraph/nodes', route => {
    route.fulfill({
      status: 500,
      body: JSON.stringify({ error: 'Internal server error' }),
    });
  });

  await page.goto('/ui/graph');
  
  // Verify error handling
  await expect(page.locator('[data-testid="error-message"]')).toBeVisible();
  await expect(page.locator('text=Failed to load nodes')).toBeVisible();
});
```

## Performance Testing

### Performance Metrics

**Core Web Vitals**:
- **Largest Contentful Paint (LCP)**: < 2.5s
- **First Input Delay (FID)**: < 100ms
- **Cumulative Layout Shift (CLS)**: < 0.1

**Custom Metrics**:
- **Graph Load Time**: < 5s for 1000 nodes
- **Query Execution Time**: < 2s for complex queries
- **Real-time Update Latency**: < 100ms

### Performance Test Examples

**Load Time Testing**:
```typescript
test('should load large graphs within performance budget', async ({ page }) => {
  await page.goto('/ui/graph');
  
  // Start performance measurement
  await page.evaluate(() => performance.mark('graph-load-start'));
  
  // Generate large dataset
  await page.click('[data-testid="generate-large-dataset"]');
  await page.selectOption('[data-testid="node-count"]', '1000');
  await page.click('[data-testid="generate"]');
  
  // Wait for completion
  await page.waitForSelector('[data-testid="graph-loaded"]');
  
  // End performance measurement
  const loadTime = await page.evaluate(() => {
    performance.mark('graph-load-end');
    performance.measure('graph-load', 'graph-load-start', 'graph-load-end');
    const measure = performance.getEntriesByName('graph-load')[0];
    return measure.duration;
  });
  
  expect(loadTime).toBeLessThan(5000); // 5 second budget
});
```

**Memory Usage Testing**:
```typescript
test('should not have memory leaks', async ({ page }) => {
  const getMemoryUsage = () => page.evaluate(() => {
    return (performance as any).memory?.usedJSHeapSize || 0;
  });

  await page.goto('/ui/graph');
  const initialMemory = await getMemoryUsage();

  // Perform memory-intensive operations
  for (let i = 0; i < 10; i++) {
    await page.click('[data-testid="generate-graph"]');
    await page.waitForSelector('[data-testid="graph-loaded"]');
    await page.click('[data-testid="clear-graph"]');
  }

  const finalMemory = await getMemoryUsage();
  const memoryIncrease = finalMemory - initialMemory;
  const memoryIncreasePercent = (memoryIncrease / initialMemory) * 100;

  expect(memoryIncreasePercent).toBeLessThan(50); // < 50% increase
});
```

## Accessibility Testing

### WCAG 2.1 AA Compliance

**Required Standards**:
- **Perceivable**: Text alternatives, captions, color contrast
- **Operable**: Keyboard accessible, no seizures, navigable
- **Understandable**: Readable, predictable, input assistance
- **Robust**: Compatible with assistive technologies

### Accessibility Test Examples

**Keyboard Navigation**:
```typescript
test('should be fully keyboard navigable', async ({ page }) => {
  await page.goto('/ui/graph');
  
  // Tab through all interactive elements
  await page.keyboard.press('Tab');
  await expect(page.locator(':focus')).toHaveAttribute('data-testid', 'main-nav');
  
  await page.keyboard.press('Tab');
  await expect(page.locator(':focus')).toHaveAttribute('data-testid', 'graph-controls');
  
  // Test keyboard shortcuts
  await page.keyboard.press('Control+f');
  await expect(page.locator('[data-testid="search-dialog"]')).toBeVisible();
});
```

**Screen Reader Support**:
```typescript
test('should have proper ARIA labels', async ({ page }) => {
  await page.goto('/ui/graph');
  
  // Check main landmarks
  await expect(page.locator('[role="main"]')).toHaveAttribute('aria-label', 'VexGraph Dashboard');
  await expect(page.locator('[role="navigation"]')).toHaveAttribute('aria-label', 'Main navigation');
  
  // Check interactive elements
  await expect(page.locator('[data-testid="create-node"]')).toHaveAttribute('aria-label', 'Create new node');
  await expect(page.locator('[data-testid="graph-view"]')).toHaveAttribute('aria-label', 'Graph visualization');
});
```

**Color Contrast**:
```typescript
test('should meet color contrast requirements', async ({ page }) => {
  await page.goto('/ui/graph');
  
  // Run axe accessibility scan
  const accessibilityScanResults = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21aa'])
    .analyze();
  
  expect(accessibilityScanResults.violations).toEqual([]);
});
```

## Cross-Browser Testing

### Supported Browsers

**Desktop**:
- Chrome (latest 2 versions)
- Firefox (latest 2 versions)
- Safari (latest 2 versions)
- Edge (latest 2 versions)

**Mobile**:
- Chrome Mobile (latest)
- Safari Mobile (latest)

### Browser-Specific Tests

```typescript
test.describe('Cross-browser compatibility', () => {
  ['chromium', 'firefox', 'webkit'].forEach(browserName => {
    test(`should work in ${browserName}`, async ({ page }) => {
      await page.goto('/ui/graph');
      
      // Test core functionality
      await expect(page.locator('[data-testid="graph-page"]')).toBeVisible();
      
      // Test browser-specific features
      if (browserName === 'webkit') {
        // Safari-specific tests
        await expect(page.locator('[data-testid="safari-warning"]')).not.toBeVisible();
      }
    });
  });
});
```

## Continuous Integration

### GitHub Actions Workflow

```yaml
name: Test Suite

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '18'
          cache: 'npm'
      
      - name: Install dependencies
        run: npm ci
      
      - name: Install Playwright browsers
        run: npx playwright install --with-deps
      
      - name: Run tests
        run: npm test
      
      - name: Upload test results
        uses: actions/upload-artifact@v3
        if: always()
        with:
          name: playwright-report
          path: playwright-report/
          retention-days: 30
```

### Test Reporting

**HTML Reports**:
```bash
npx playwright show-report
```

**JUnit Reports** (for CI):
```typescript
// playwright.config.ts
export default defineConfig({
  reporter: [
    ['html'],
    ['junit', { outputFile: 'test-results/junit.xml' }],
  ],
});
```

## Quality Assurance

### Code Quality Gates

**Pre-commit Hooks**:
```json
{
  "husky": {
    "hooks": {
      "pre-commit": "lint-staged"
    }
  },
  "lint-staged": {
    "*.{ts,tsx}": [
      "eslint --fix",
      "prettier --write",
      "npm run test:changed"
    ]
  }
}
```

**Quality Metrics**:
- **Test Coverage**: > 90%
- **Performance Budget**: LCP < 2.5s, FID < 100ms
- **Accessibility**: 0 violations
- **Bundle Size**: < 500KB gzipped
- **Lighthouse Score**: > 90

### Test Maintenance

**Regular Tasks**:
- Update test data monthly
- Review and update selectors quarterly
- Performance baseline updates
- Browser compatibility matrix updates
- Accessibility standard updates

**Test Debt Management**:
- Identify flaky tests
- Remove obsolete tests
- Refactor duplicate test logic
- Update test documentation

---

This testing documentation ensures comprehensive coverage and quality assurance for the VexGraph Dashboard. Regular updates and maintenance of the test suite are essential for maintaining high quality standards.