# VexFS Dashboard Testing Workflow

This document outlines a comprehensive testing workflow for the VexFS Dashboard frontend, including Playwright tests and a mock backend for isolated testing.

## Overview

The testing workflow includes:
1. **Mock Backend Server** - A lightweight mock server that simulates the VexFS API
2. **Playwright Tests** - End-to-end tests for the frontend UI
3. **Test Data Management** - Consistent test data across test runs
4. **CI/CD Integration** - Automated testing in continuous integration

## Quick Start

### 1. Install Dependencies

```bash
cd vexfs-dashboard
npm install
npx playwright install
```

### 2. Run Tests

```bash
# Run all tests
npm run test

# Run tests with UI
npm run test:ui

# Run tests in headed mode (see browser)
npm run test:headed

# Debug tests
npm run test:debug
```

### 3. Start Mock Server for Development

```bash
# Start the mock server on port 3001
npm run mock-server

# In another terminal, start the frontend
npm run dev
```

## Test Structure

### Mock Server (`tests/setup/mock-server.ts`)

The mock server provides:
- **ChromaDB API compatibility** (`/api/v1/*`)
- **Qdrant API compatibility** (`/collections/*`)
- **Native VexFS API** (`/vexfs/v1/*`)
- **Health endpoints** (`/health`, `/metrics`)
- **Realistic test data** with multiple collections and vectors

Key features:
- Stateful mock data that persists during test runs
- Realistic response times and error scenarios
- Support for all CRUD operations
- Pagination and filtering support

### Test Files

#### `tests/collections.spec.ts`
Comprehensive tests for the Collections page:
- ✅ Display collections list
- ✅ Search and filter collections
- ✅ Sort collections by different fields
- ✅ Create new collections
- ✅ Edit existing collections
- ✅ Delete collections with confirmation
- ✅ Pagination handling
- ✅ Empty state handling
- ✅ Loading state handling
- ✅ Error state handling
- ✅ Responsive design (mobile)

#### `tests/dashboard.spec.ts` (TODO)
Tests for the main dashboard:
- System metrics display
- Real-time updates
- Performance charts
- Health indicators

#### `tests/search.spec.ts` (TODO)
Tests for vector search functionality:
- Vector similarity search
- Metadata filtering
- Hybrid search
- Search analytics

#### `tests/vectors.spec.ts` (TODO)
Tests for vector management:
- Add vectors to collections
- View vector details
- Vector visualization
- Bulk operations

## Mock Data

The mock server includes realistic test data:

### Collections
- **Test Collection 1**: 384-dimensional vectors, cosine distance, 1000 points
- **Test Collection 2**: 768-dimensional vectors, euclidean distance, 500 points
- **Large Collection**: 1536-dimensional vectors, dot product distance, 10000 points

### Vectors
- Randomly generated vectors with appropriate dimensions
- Realistic metadata (text, categories, timestamps)
- Proper payload structure

## Testing Scenarios

### Happy Path Tests
- Normal user workflows
- Expected interactions
- Standard data operations

### Edge Cases
- Empty collections
- Large datasets
- Network timeouts
- Invalid inputs

### Error Handling
- API failures
- Network errors
- Validation errors
- Permission errors

### Performance Tests
- Large collection handling
- Pagination performance
- Search response times
- UI responsiveness

## Browser Coverage

Tests run on:
- **Desktop**: Chrome, Firefox, Safari
- **Mobile**: Chrome Mobile, Safari Mobile
- **Accessibility**: Screen reader compatibility

## CI/CD Integration

### GitHub Actions (Example)

```yaml
name: Frontend Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'
      - run: cd vexfs-dashboard && npm ci
      - run: cd vexfs-dashboard && npx playwright install
      - run: cd vexfs-dashboard && npm run test
      - uses: actions/upload-artifact@v3
        if: always()
        with:
          name: playwright-report
          path: vexfs-dashboard/playwright-report/
```

## Development Workflow

### 1. Fix Frontend Issues

When you encounter JavaScript errors like "TypeError: E is undefined":

1. **Start the mock server**:
   ```bash
   npm run mock-server
   ```

2. **Run the frontend against the mock**:
   ```bash
   npm run dev
   ```

3. **Debug with browser dev tools**:
   - Open browser console
   - Check network requests
   - Inspect component state

4. **Write tests for the fix**:
   ```bash
   npm run test:debug
   ```

### 2. Iterative Testing

1. **Write failing test** for the bug
2. **Fix the implementation**
3. **Verify test passes**
4. **Add edge case tests**
5. **Refactor if needed**

### 3. Mock Server Development

To add new API endpoints:

1. **Add handler to `mock-server.ts`**
2. **Add corresponding test data**
3. **Update tests to use new endpoint**
4. **Verify against real API**

## Debugging Tips

### Common Issues

1. **"E is undefined" errors**:
   - Check import statements
   - Verify component exports
   - Check for circular dependencies

2. **API call failures**:
   - Verify mock server is running
   - Check network tab in dev tools
   - Validate request/response format

3. **Test flakiness**:
   - Add proper wait conditions
   - Use data-testid attributes
   - Avoid timing-dependent assertions

### Debug Commands

```bash
# Run specific test file
npx playwright test collections.spec.ts

# Run tests with debug info
npx playwright test --debug

# Generate test report
npx playwright show-report

# Record new tests
npx playwright codegen localhost:3000
```

## Best Practices

### Test Writing
- Use descriptive test names
- Group related tests in describe blocks
- Use data-testid attributes for reliable selectors
- Test user workflows, not implementation details
- Include both positive and negative test cases

### Mock Data
- Keep mock data realistic
- Use consistent IDs across tests
- Reset state between tests
- Include edge cases in mock responses

### Maintenance
- Update tests when UI changes
- Keep mock server in sync with real API
- Review and update test data regularly
- Monitor test execution times

## Future Enhancements

### Planned Features
- **Visual regression testing** with screenshot comparison
- **Performance monitoring** with Lighthouse integration
- **Accessibility testing** with axe-core
- **API contract testing** with schema validation
- **Load testing** with k6 integration

### Integration Options
- **Storybook integration** for component testing
- **Jest unit tests** for utility functions
- **Cypress alternative** for comparison
- **Docker containerization** for consistent environments

This testing workflow provides a solid foundation for maintaining high-quality frontend code while enabling rapid iteration and debugging.