import React, { useState, useCallback, useMemo } from 'react';
import {
  Box,
  Typography,
  Grid,
  Card,
  CardContent,
  Switch,
  FormControlLabel,
  Alert,
} from '@mui/material';
import { ErrorBoundary } from '../components/Common/ErrorBoundary';
import { usePerformanceMonitor } from '../hooks/usePerformanceMonitor';

// Lazy load monitoring components for better performance
const SystemMetrics = React.lazy(() =>
  import('../components/Monitoring/SystemMetrics')
);
const PerformanceCharts = React.lazy(() =>
  import('../components/Monitoring/PerformanceCharts')
);
const HealthIndicators = React.lazy(() =>
  import('../components/Monitoring/HealthIndicators')
);
const AlertsPanel = React.lazy(() =>
  import('../components/Monitoring/AlertsPanel')
);
const RealTimeUpdates = React.lazy(() =>
  import('../components/Monitoring/RealTimeUpdates')
);

// Memoized loading fallback
const MonitoringLoadingFallback = React.memo(() => (
  <Card>
    <CardContent>
      <Box
        display="flex"
        justifyContent="center"
        alignItems="center"
        minHeight="200px"
      >
        <Typography>Loading monitoring data...</Typography>
      </Box>
    </CardContent>
  </Card>
));

MonitoringLoadingFallback.displayName = 'MonitoringLoadingFallback';

// Memoized error fallback
const MonitoringErrorFallback = React.memo(() => (
  <Alert severity="error" sx={{ mt: 2 }}>
    Failed to load monitoring component. Please try refreshing the page.
  </Alert>
));

MonitoringErrorFallback.displayName = 'MonitoringErrorFallback';

// Memoized header component
const MonitoringHeader = React.memo<{
  realTimeEnabled: boolean;
  onRealTimeToggle: (enabled: boolean) => void;
}>(({ realTimeEnabled, onRealTimeToggle }) => (
  <Box
    sx={{
      display: 'flex',
      justifyContent: 'space-between',
      alignItems: 'center',
      mb: 3,
    }}
  >
    <Typography variant="h4" component="h1" sx={{ fontWeight: 600 }}>
      System Monitoring
    </Typography>
    <FormControlLabel
      control={
        <Switch
          checked={realTimeEnabled}
          onChange={(e) => onRealTimeToggle(e.target.checked)}
        />
      }
      label="Real-time Updates"
    />
  </Box>
));

MonitoringHeader.displayName = 'MonitoringHeader';

const Monitoring: React.FC = () => {
  const [realTimeEnabled, setRealTimeEnabled] = useState(true);

  // Performance monitoring
  const { startTiming, endTiming } = usePerformanceMonitor('Monitoring');

  React.useEffect(() => {
    startTiming();
    return endTiming;
  });

  // Memoized handlers
  const handleRealTimeToggle = useCallback((enabled: boolean) => {
    setRealTimeEnabled(enabled);
  }, []);

  // Mock data for monitoring components
  const mockSystemMetrics = useMemo(
    () => ({
      cpu: { usage: 45, cores: 8 },
      memory: { used: 8.2, total: 16 },
      disk: { used: 120, total: 500 },
      network: { inbound: 1.2, outbound: 0.8 },
    }),
    []
  );

  const mockPerformanceData = useMemo(
    () => ({
      responseTime: [
        { time: '00:00', value: 45 },
        { time: '00:05', value: 52 },
        { time: '00:10', value: 38 },
      ],
      throughput: [
        { time: '00:00', value: 1200 },
        { time: '00:05', value: 1350 },
        { time: '00:10', value: 1180 },
      ],
    }),
    []
  );

  const mockHealthData = useMemo(
    () => ({
      database: { status: 'healthy', responseTime: 12 },
      api: { status: 'healthy', responseTime: 45 },
      storage: { status: 'warning', responseTime: 89 },
    }),
    []
  );

  const mockAlerts = useMemo(
    () => [
      {
        id: '1',
        type: 'warning',
        message: 'High memory usage detected',
        timestamp: new Date().toISOString(),
      },
      {
        id: '2',
        type: 'info',
        message: 'System backup completed',
        timestamp: new Date().toISOString(),
      },
    ],
    []
  );

  // Memoized component wrappers
  const suspenseWrapper = useCallback(
    (component: React.ReactNode) => (
      <ErrorBoundary fallback={<MonitoringErrorFallback />}>
        <React.Suspense fallback={<MonitoringLoadingFallback />}>
          {component}
        </React.Suspense>
      </ErrorBoundary>
    ),
    []
  );

  return (
    <ErrorBoundary>
      <Box>
        {/* Header */}
        <MonitoringHeader
          realTimeEnabled={realTimeEnabled}
          onRealTimeToggle={handleRealTimeToggle}
        />

        {/* Monitoring Grid */}
        <Grid container spacing={3}>
          {/* System Metrics */}
          <Grid item xs={12} md={6}>
            {suspenseWrapper(
              <SystemMetrics
                metrics={mockSystemMetrics}
                realTimeEnabled={realTimeEnabled}
              />
            )}
          </Grid>

          {/* Health Indicators */}
          <Grid item xs={12} md={6}>
            {suspenseWrapper(
              <HealthIndicators
                health={mockHealthData}
                realTimeEnabled={realTimeEnabled}
              />
            )}
          </Grid>

          {/* Performance Charts */}
          <Grid item xs={12}>
            {suspenseWrapper(
              <PerformanceCharts
                data={mockPerformanceData}
                realTimeEnabled={realTimeEnabled}
              />
            )}
          </Grid>

          {/* Alerts Panel */}
          <Grid item xs={12} md={8}>
            {suspenseWrapper(
              <AlertsPanel
                alerts={mockAlerts}
                onDismissAlert={(alertId) => console.log('Dismiss alert:', alertId)}
                onUpdateRule={(ruleId, updates) => console.log('Update rule:', ruleId, updates)}
                onDeleteRule={(ruleId) => console.log('Delete rule:', ruleId)}
              />
            )}
          </Grid>

          {/* Real-time Updates */}
          <Grid item xs={12} md={4}>
            {suspenseWrapper(
              <RealTimeUpdates
                enabled={realTimeEnabled}
                updates={[]}
              />
            )}
          </Grid>
        </Grid>
      </Box>
    </ErrorBoundary>
  );
};

export default React.memo(Monitoring);
