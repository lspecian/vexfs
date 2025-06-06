import React, { useState, useCallback, useMemo } from 'react';
import {
  Box,
  Typography,
  Tabs,
  Tab,
  Paper,
  Alert,
} from '@mui/material';
import { ErrorBoundary } from '../components/Common/ErrorBoundary';
import { usePerformanceMonitor } from '../hooks/usePerformanceMonitor';
import { useCollections } from '../hooks/useVexFS';

// Lazy load search components for better performance
const VectorSimilaritySearch = React.lazy(() =>
  import('../components/Search/VectorSimilaritySearch')
);
const MetadataFilterSearch = React.lazy(() =>
  import('../components/Search/MetadataFilterSearch')
);
const HybridSearch = React.lazy(() =>
  import('../components/Search/HybridSearch')
);
const SavedSearches = React.lazy(() =>
  import('../components/Search/SavedSearches')
);
const SearchAnalytics = React.lazy(() =>
  import('../components/Search/SearchAnalytics')
);

interface TabPanelProps {
  children?: React.ReactNode;
  index: number;
  value: number;
}

// Memoized TabPanel component
const TabPanel = React.memo<TabPanelProps>(
  ({ children, value, index, ...other }) => (
    <div
      role="tabpanel"
      hidden={value !== index}
      id={`search-tabpanel-${index}`}
      aria-labelledby={`search-tab-${index}`}
      {...other}
    >
      {value === index && <Box sx={{ py: 3 }}>{children}</Box>}
    </div>
  )
);

TabPanel.displayName = 'TabPanel';

// Memoized loading fallback
const SearchLoadingFallback = React.memo(() => (
  <Box
    display="flex"
    justifyContent="center"
    alignItems="center"
    minHeight="300px"
  >
    <Typography>Loading search component...</Typography>
  </Box>
));

SearchLoadingFallback.displayName = 'SearchLoadingFallback';

// Memoized error fallback
const SearchErrorFallback = React.memo(() => (
  <Alert severity="error" sx={{ mt: 2 }}>
    Failed to load search component. Please try refreshing the page.
  </Alert>
));

SearchErrorFallback.displayName = 'SearchErrorFallback';

const Search: React.FC = () => {
  const [activeTab, setActiveTab] = useState(0);
  const [selectedCollection, setSelectedCollection] = useState<string | null>(null);
  const [vectors, setVectors] = useState<any[]>([]);
  const [savedSearches, setSavedSearches] = useState<any[]>([]);

  // Get collections data
  const { collections, loading: collectionsLoading } = useCollections();

  // Performance monitoring
  const { startTiming, endTiming } = usePerformanceMonitor('Search');

  React.useEffect(() => {
    startTiming();
    return endTiming;
  });

  // Memoized tab change handler
  const handleTabChange = useCallback(
    (_: React.SyntheticEvent, newValue: number) => {
      setActiveTab(newValue);
    },
    []
  );

  // Memoized tab accessibility props
  const getTabProps = useCallback(
    (index: number) => ({
      id: `search-tab-${index}`,
      'aria-controls': `search-tabpanel-${index}`,
    }),
    []
  );

  // Memoized handlers
  const handleCollectionChange = useCallback((collectionId: string) => {
    setSelectedCollection(collectionId);
  }, []);

  const handleVectorSearch = useCallback((query: any) => {
    console.log('Vector search:', query);
    // TODO: Implement vector search
  }, []);

  const handleMetadataSearch = useCallback((query: any) => {
    console.log('Metadata search:', query);
    // TODO: Implement metadata search
  }, []);

  const handleHybridSearch = useCallback((query: any) => {
    console.log('Hybrid search:', query);
    // TODO: Implement hybrid search
  }, []);

  const handleSaveSearch = useCallback((search: any) => {
    setSavedSearches(prev => [...prev, { ...search, id: Date.now() }]);
  }, []);

  const handleDeleteSearch = useCallback((searchId: string) => {
    setSavedSearches(prev => prev.filter(s => s.id !== searchId));
  }, []);

  const handleUpdateSearch = useCallback((searchId: string, updates: any) => {
    setSavedSearches(prev =>
      prev.map(s => (s.id === searchId ? { ...s, ...updates } : s))
    );
  }, []);

  const handleExecuteSearch = useCallback((search: any) => {
    console.log('Execute saved search:', search);
    // TODO: Implement execute search
  }, []);

  // Mock analytics data
  const mockAnalytics = useMemo(
    () => ({
      totalSearches: 1234,
      avgResponseTime: 45,
      popularQueries: ['similarity search', 'metadata filter'],
      searchTrends: [],
    }),
    []
  );

  // Memoized tab content to prevent unnecessary re-renders
  const tabContent = useMemo(() => {
    const suspenseWrapper = (component: React.ReactNode) => (
      <ErrorBoundary fallback={<SearchErrorFallback />}>
        <React.Suspense fallback={<SearchLoadingFallback />}>
          {component}
        </React.Suspense>
      </ErrorBoundary>
    );

    return [
      suspenseWrapper(
        <VectorSimilaritySearch
          collections={collections}
          selectedCollection={selectedCollection}
          onCollectionChange={handleCollectionChange}
          onSearch={handleVectorSearch}
          vectors={vectors}
          loading={collectionsLoading}
        />
      ),
      suspenseWrapper(
        <MetadataFilterSearch
          collections={collections}
          selectedCollection={selectedCollection}
          onCollectionChange={handleCollectionChange}
          onSearch={handleMetadataSearch}
        />
      ),
      suspenseWrapper(
        <HybridSearch
          collections={collections}
          selectedCollection={selectedCollection}
          onCollectionChange={handleCollectionChange}
          onSearch={handleHybridSearch}
          vectors={vectors}
        />
      ),
      suspenseWrapper(
        <SavedSearches
          savedSearches={savedSearches}
          onExecuteSearch={handleExecuteSearch}
          onSaveSearch={handleSaveSearch}
          onDeleteSearch={handleDeleteSearch}
          onUpdateSearch={handleUpdateSearch}
        />
      ),
      suspenseWrapper(
        <SearchAnalytics analytics={mockAnalytics} />
      ),
    ];
  }, [
    collections,
    selectedCollection,
    vectors,
    savedSearches,
    collectionsLoading,
    handleCollectionChange,
    handleVectorSearch,
    handleMetadataSearch,
    handleHybridSearch,
    handleExecuteSearch,
    handleSaveSearch,
    handleDeleteSearch,
    handleUpdateSearch,
    mockAnalytics,
  ]);

  return (
    <ErrorBoundary>
      <Box>
        {/* Header */}
        <Typography variant="h4" component="h1" sx={{ fontWeight: 600, mb: 3 }}>
          Search
        </Typography>

        {/* Search Interface */}
        <Paper elevation={1}>
          <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
            <Tabs
              value={activeTab}
              onChange={handleTabChange}
              aria-label="search tabs"
              variant="scrollable"
              scrollButtons="auto"
            >
              <Tab label="Vector Similarity" {...getTabProps(0)} />
              <Tab label="Metadata Filter" {...getTabProps(1)} />
              <Tab label="Hybrid Search" {...getTabProps(2)} />
              <Tab label="Saved Searches" {...getTabProps(3)} />
              <Tab label="Analytics" {...getTabProps(4)} />
            </Tabs>
          </Box>

          {/* Tab Panels */}
          <TabPanel value={activeTab} index={0}>
            {tabContent[0]}
          </TabPanel>
          <TabPanel value={activeTab} index={1}>
            {tabContent[1]}
          </TabPanel>
          <TabPanel value={activeTab} index={2}>
            {tabContent[2]}
          </TabPanel>
          <TabPanel value={activeTab} index={3}>
            {tabContent[3]}
          </TabPanel>
          <TabPanel value={activeTab} index={4}>
            {tabContent[4]}
          </TabPanel>
        </Paper>
      </Box>
    </ErrorBoundary>
  );
};

export default React.memo(Search);
