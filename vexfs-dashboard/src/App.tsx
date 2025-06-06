import React, { Suspense } from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { ThemeProvider } from '@mui/material/styles';
import { CssBaseline, Box, CircularProgress } from '@mui/material';
import { SnackbarProvider } from 'notistack';
import { ErrorBoundary } from './components/Common/ErrorBoundary';
import { AuthProvider } from './components/Auth/AuthProvider';
import AppLayout from './components/Layout/AppLayout';
import PerformanceMonitor from './components/Common/PerformanceMonitor';
import { lightTheme } from './theme';

// Lazy load pages for better performance
const Dashboard = React.lazy(() => import('./pages/Dashboard'));
const Collections = React.lazy(() => import('./pages/Collections'));
const Search = React.lazy(() => import('./pages/Search'));
const Monitoring = React.lazy(() => import('./pages/Monitoring'));
const Settings = React.lazy(() => import('./pages/Settings'));

// Loading fallback component
const PageLoadingFallback: React.FC = () => (
  <Box
    display="flex"
    justifyContent="center"
    alignItems="center"
    minHeight="400px"
  >
    <CircularProgress />
  </Box>
);

// Error fallback for route-level errors
const RouteErrorFallback: React.FC = () => (
  <Box
    display="flex"
    flexDirection="column"
    alignItems="center"
    justifyContent="center"
    minHeight="400px"
    p={3}
  >
    <CircularProgress color="error" />
    <Box mt={2}>Failed to load page. Please try refreshing.</Box>
  </Box>
);

function App() {
  return (
    <ErrorBoundary
      fallback={<RouteErrorFallback />}
      onError={(error, errorInfo) => {
        // Log errors to console in development
        if (import.meta.env.DEV) {
          console.error('App-level error:', error, errorInfo);
        }
        // In production, you might want to send to error reporting service
      }}
    >
      <ThemeProvider theme={lightTheme}>
        <CssBaseline />
        <SnackbarProvider
          maxSnack={3}
          anchorOrigin={{
            vertical: 'bottom',
            horizontal: 'left',
          }}
          preventDuplicate
        >
          <AuthProvider>
            <Router basename="/ui">
              <Routes>
                <Route path="/" element={<AppLayout />}>
                  <Route
                    index
                    element={
                      <ErrorBoundary fallback={<RouteErrorFallback />}>
                        <Suspense fallback={<PageLoadingFallback />}>
                          <Dashboard />
                        </Suspense>
                      </ErrorBoundary>
                    }
                  />
                  <Route
                    path="collections"
                    element={
                      <ErrorBoundary fallback={<RouteErrorFallback />}>
                        <Suspense fallback={<PageLoadingFallback />}>
                          <Collections />
                        </Suspense>
                      </ErrorBoundary>
                    }
                  />
                  <Route
                    path="search"
                    element={
                      <ErrorBoundary fallback={<RouteErrorFallback />}>
                        <Suspense fallback={<PageLoadingFallback />}>
                          <Search />
                        </Suspense>
                      </ErrorBoundary>
                    }
                  />
                  <Route
                    path="monitoring"
                    element={
                      <ErrorBoundary fallback={<RouteErrorFallback />}>
                        <Suspense fallback={<PageLoadingFallback />}>
                          <Monitoring />
                        </Suspense>
                      </ErrorBoundary>
                    }
                  />
                  <Route
                    path="settings"
                    element={
                      <ErrorBoundary fallback={<RouteErrorFallback />}>
                        <Suspense fallback={<PageLoadingFallback />}>
                          <Settings />
                        </Suspense>
                      </ErrorBoundary>
                    }
                  />
                </Route>
              </Routes>
            </Router>
            <PerformanceMonitor />
          </AuthProvider>
        </SnackbarProvider>
      </ThemeProvider>
    </ErrorBoundary>
  );
}

export default App;
