import React, { useEffect, useState, useCallback } from 'react';
import {
  Box,
  Typography,
  Switch,
  FormControlLabel,
  Select,
  MenuItem,
  FormControl,
  InputLabel,
  Chip,
  useTheme,
} from '@mui/material';
import {
  Refresh as RefreshIcon,
  Pause as PauseIcon,
  PlayArrow as PlayIcon,
} from '@mui/icons-material';
import type {
  SystemMetrics,
  PerformanceMetrics,
  HealthStatus,
  Alert,
  RealTimeUpdate,
} from '../../types/monitoring';

interface RealTimeUpdatesProps {
  onSystemMetricsUpdate?: (metrics: SystemMetrics) => void;
  onPerformanceMetricsUpdate?: (metrics: PerformanceMetrics) => void;
  onHealthStatusUpdate?: (status: HealthStatus) => void;
  onAlertUpdate?: (alert: Alert) => void;
  onConnectionStatusChange?: (connected: boolean) => void;
  enabled?: boolean;
  refreshInterval?: number;
}

const REFRESH_INTERVALS = [
  { label: '5 seconds', value: 5000 },
  { label: '10 seconds', value: 10000 },
  { label: '30 seconds', value: 30000 },
  { label: '1 minute', value: 60000 },
  { label: '5 minutes', value: 300000 },
];

const RealTimeUpdates: React.FC<RealTimeUpdatesProps> = ({
  onSystemMetricsUpdate,
  onPerformanceMetricsUpdate,
  onHealthStatusUpdate,
  onAlertUpdate,
  onConnectionStatusChange,
  enabled = true,
  refreshInterval = 10000,
}) => {
  const theme = useTheme();
  const [isEnabled, setIsEnabled] = useState(enabled);
  const [currentInterval, setCurrentInterval] = useState(refreshInterval);
  const [isConnected, setIsConnected] = useState(false);
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null);
  const [updateCount, setUpdateCount] = useState(0);

  // Mock data generators for development
  const generateMockSystemMetrics = useCallback(
    (): SystemMetrics => ({
      cpu: {
        usage: Math.random() * 100,
        cores: 8,
        loadAverage: [Math.random() * 2, Math.random() * 2, Math.random() * 2],
      },
      memory: {
        used: 8589934592 * Math.random(),
        total: 17179869184,
        available: 8589934592 * (1 - Math.random()),
        percentage: Math.random() * 100,
      },
      disk: {
        used: 107374182400 * Math.random(),
        total: 214748364800,
        available: 107374182400 * (1 - Math.random()),
        percentage: Math.random() * 100,
      },
      network: {
        bytesIn: Math.floor(Math.random() * 1000000000),
        bytesOut: Math.floor(Math.random() * 1000000000),
        packetsIn: Math.floor(Math.random() * 1000000),
        packetsOut: Math.floor(Math.random() * 1000000),
      },
    }),
    []
  );

  const generateMockPerformanceMetrics = useCallback(
    (): PerformanceMetrics => ({
      queryPerformance: {
        averageResponseTime: Math.random() * 100 + 10,
        p95ResponseTime: Math.random() * 200 + 50,
        p99ResponseTime: Math.random() * 500 + 100,
        throughput: Math.random() * 1000 + 100,
        totalQueries: Math.floor(Math.random() * 1000000),
      },
      vectorOperations: {
        indexingRate: Math.random() * 1000 + 50,
        searchRate: Math.random() * 500 + 25,
        totalIndexed: Math.floor(Math.random() * 10000000),
        totalSearches: Math.floor(Math.random() * 1000000),
      },
      storage: {
        readThroughput: Math.random() * 500 + 50,
        writeThroughput: Math.random() * 300 + 30,
        iops: Math.random() * 10000 + 1000,
      },
    }),
    []
  );

  const generateMockHealthStatus = useCallback((): HealthStatus => {
    const statuses: Array<'healthy' | 'warning' | 'critical'> = [
      'healthy',
      'healthy',
      'healthy',
      'warning',
    ];
    const randomStatus = () =>
      statuses[Math.floor(Math.random() * statuses.length)];

    return {
      overall: 'healthy',
      services: {
        vexfsCore: {
          status: randomStatus(),
          responseTime: Math.random() * 50 + 5,
          errorRate: Math.random() * 5,
          lastCheck: new Date().toISOString(),
        },
        database: {
          status: randomStatus(),
          responseTime: Math.random() * 30 + 2,
          errorRate: Math.random() * 2,
          lastCheck: new Date().toISOString(),
        },
        vectorIndex: {
          status: randomStatus(),
          responseTime: Math.random() * 100 + 10,
          errorRate: Math.random() * 3,
          lastCheck: new Date().toISOString(),
        },
        api: {
          status: 'healthy',
          responseTime: Math.random() * 20 + 1,
          errorRate: 0,
          lastCheck: new Date().toISOString(),
        },
      },
      uptime: Math.floor(Math.random() * 86400 * 30),
      lastHealthCheck: new Date().toISOString(),
    };
  }, []);

  // Simulate real-time updates
  useEffect(() => {
    if (!isEnabled) {
      setIsConnected(false);
      onConnectionStatusChange?.(false);
      return;
    }

    setIsConnected(true);
    onConnectionStatusChange?.(true);

    const interval = setInterval(() => {
      // Simulate occasional connection issues
      if (Math.random() < 0.05) {
        setIsConnected(false);
        onConnectionStatusChange?.(false);
        return;
      }

      setIsConnected(true);
      onConnectionStatusChange?.(true);
      setLastUpdate(new Date());
      setUpdateCount(prev => prev + 1);

      // Generate and send mock updates
      if (onSystemMetricsUpdate) {
        onSystemMetricsUpdate(generateMockSystemMetrics());
      }

      if (onPerformanceMetricsUpdate) {
        onPerformanceMetricsUpdate(generateMockPerformanceMetrics());
      }

      if (onHealthStatusUpdate) {
        onHealthStatusUpdate(generateMockHealthStatus());
      }

      // Occasionally generate alerts
      if (Math.random() < 0.1 && onAlertUpdate) {
        const alertTypes: Alert['type'][] = ['info', 'warning', 'error'];
        const randomType =
          alertTypes[Math.floor(Math.random() * alertTypes.length)];

        onAlertUpdate({
          id: `alert-${Date.now()}`,
          type: randomType,
          title: `${randomType.charAt(0).toUpperCase() + randomType.slice(1)} Alert`,
          message: `This is a mock ${randomType} alert generated at ${new Date().toLocaleTimeString()}`,
          timestamp: new Date().toISOString(),
          acknowledged: false,
          source: 'real-time-monitor',
        });
      }
    }, currentInterval);

    return () => {
      clearInterval(interval);
      setIsConnected(false);
      onConnectionStatusChange?.(false);
    };
  }, [
    isEnabled,
    currentInterval,
    onSystemMetricsUpdate,
    onPerformanceMetricsUpdate,
    onHealthStatusUpdate,
    onAlertUpdate,
    onConnectionStatusChange,
    generateMockSystemMetrics,
    generateMockPerformanceMetrics,
    generateMockHealthStatus,
  ]);

  const handleToggle = (event: React.ChangeEvent<HTMLInputElement>) => {
    setIsEnabled(event.target.checked);
  };

  const handleIntervalChange = (event: any) => {
    setCurrentInterval(event.target.value);
  };

  const formatLastUpdate = (date: Date | null): string => {
    if (!date) return 'Never';
    return date.toLocaleTimeString();
  };

  const getConnectionStatus = () => {
    if (!isEnabled) return { label: 'Disabled', color: 'default' as const };
    if (isConnected) return { label: 'Connected', color: 'success' as const };
    return { label: 'Disconnected', color: 'error' as const };
  };

  const status = getConnectionStatus();

  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        gap: 2,
        p: 2,
        backgroundColor: theme.palette.background.paper,
        borderRadius: 1,
        border: `1px solid ${theme.palette.divider}`,
      }}
    >
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
        {isEnabled && isConnected ? (
          <RefreshIcon color="success" />
        ) : isEnabled ? (
          <PauseIcon color="warning" />
        ) : (
          <PlayIcon color="disabled" />
        )}
        <Typography variant="body2" fontWeight="medium">
          Real-time Updates
        </Typography>
      </Box>

      <FormControlLabel
        control={
          <Switch checked={isEnabled} onChange={handleToggle} size="small" />
        }
        label="Enable"
        sx={{ margin: 0 }}
      />

      <FormControl size="small" sx={{ minWidth: 120 }}>
        <InputLabel>Interval</InputLabel>
        <Select
          value={currentInterval}
          onChange={handleIntervalChange}
          disabled={!isEnabled}
          label="Interval"
        >
          {REFRESH_INTERVALS.map(interval => (
            <MenuItem key={interval.value} value={interval.value}>
              {interval.label}
            </MenuItem>
          ))}
        </Select>
      </FormControl>

      <Chip
        label={status.label}
        color={status.color}
        size="small"
        variant={isConnected ? 'filled' : 'outlined'}
      />

      {lastUpdate && (
        <Typography variant="caption" color="text.secondary">
          Last update: {formatLastUpdate(lastUpdate)}
        </Typography>
      )}

      {isEnabled && (
        <Typography variant="caption" color="text.secondary">
          Updates: {updateCount}
        </Typography>
      )}
    </Box>
  );
};

export default RealTimeUpdates;
