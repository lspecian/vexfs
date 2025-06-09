import React, { useState } from 'react';
import {
  Box,
  Grid,
  Paper,
  Typography,
  Card,
  CardContent,
  CardHeader,
  Chip,
  LinearProgress,
  useTheme,
  ToggleButton,
  ToggleButtonGroup,
  Alert,
} from '@mui/material';
import {
  LineChart,
  Line,
  AreaChart,
  Area,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
  RadialBarChart,
  RadialBar,
  Legend,
} from 'recharts';
import {
  Speed as SpeedIcon,
  Memory as MemoryIcon,
  Storage as StorageIcon,
  TrendingUp as TrendingUpIcon,
} from '@mui/icons-material';

import type { PerformanceMetrics } from './GraphAnalyticsDashboard';

export interface PerformanceChartsProps {
  metrics: PerformanceMetrics;
  isLoading?: boolean;
}

export const PerformanceCharts: React.FC<PerformanceChartsProps> = ({
  metrics,
  isLoading = false,
}) => {
  const theme = useTheme();
  const [timeRange, setTimeRange] = useState<'1h' | '6h' | '24h' | '7d'>('1h');

  // Generate mock time series data for performance metrics
  const generateTimeSeriesData = (baseValue: number, variance: number, points: number = 20) => {
    const now = new Date();
    return Array.from({ length: points }, (_, i) => {
      const timestamp = new Date(now.getTime() - (points - 1 - i) * (60000 * (timeRange === '1h' ? 3 : timeRange === '6h' ? 18 : timeRange === '24h' ? 72 : 504)));
      const value = baseValue + (Math.random() - 0.5) * variance;
      return {
        timestamp: timestamp.toLocaleTimeString(),
        value: Math.max(0, value),
      };
    });
  };

  // Performance data
  const queryPerformanceData = generateTimeSeriesData(metrics.query_performance.average_query_time_ms, 20);
  const throughputData = generateTimeSeriesData(metrics.query_performance.queries_per_second, 100);
  const memoryUsageData = generateTimeSeriesData(metrics.memory_usage.ram_utilization_mb, 50);
  const cacheHitData = generateTimeSeriesData(metrics.memory_usage.cache_hit_rate * 100, 5);

  // Performance summary data
  const performanceSummary = [
    {
      category: 'Query Performance',
      metrics: [
        { name: 'Avg Query Time', value: `${metrics.query_performance.average_query_time_ms.toFixed(1)}ms`, status: 'good' },
        { name: 'Queries/Second', value: metrics.query_performance.queries_per_second.toFixed(0), status: 'good' },
        { name: 'Success Rate', value: `${(metrics.query_performance.success_rate * 100).toFixed(1)}%`, status: 'excellent' },
        { name: 'Timeout Rate', value: `${(metrics.query_performance.timeout_rate * 100).toFixed(2)}%`, status: 'warning' },
      ],
    },
    {
      category: 'Index Efficiency',
      metrics: [
        { name: 'Hit Rate', value: `${(metrics.index_efficiency.hit_rate * 100).toFixed(1)}%`, status: 'good' },
        { name: 'Update Time', value: `${metrics.index_efficiency.update_time_ms.toFixed(1)}ms`, status: 'good' },
        { name: 'Storage Overhead', value: `${metrics.index_efficiency.storage_overhead_mb.toFixed(1)}MB`, status: 'warning' },
      ],
    },
    {
      category: 'Memory Usage',
      metrics: [
        { name: 'RAM Utilization', value: `${metrics.memory_usage.ram_utilization_mb.toFixed(0)}MB`, status: 'good' },
        { name: 'Cache Hit Rate', value: `${(metrics.memory_usage.cache_hit_rate * 100).toFixed(1)}%`, status: 'excellent' },
        { name: 'GC Frequency', value: `${metrics.memory_usage.gc_frequency_per_hour.toFixed(1)}/hr`, status: 'good' },
      ],
    },
    {
      category: 'Storage',
      metrics: [
        { name: 'Disk Usage', value: `${(metrics.storage_metrics.disk_usage_mb / 1024).toFixed(1)}GB`, status: 'good' },
        { name: 'Compression Ratio', value: `${metrics.storage_metrics.compression_ratio.toFixed(1)}:1`, status: 'excellent' },
        { name: 'I/O Operations', value: `${metrics.storage_metrics.io_operations_per_second.toFixed(0)}/s`, status: 'good' },
      ],
    },
  ];

  // Status colors
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'excellent': return theme.palette.success.main;
      case 'good': return theme.palette.info.main;
      case 'warning': return theme.palette.warning.main;
      case 'critical': return theme.palette.error.main;
      default: return theme.palette.text.secondary;
    }
  };

  // Resource utilization data for radial chart
  const resourceUtilization = [
    {
      name: 'CPU',
      value: Math.random() * 40 + 30, // Mock CPU usage
      fill: theme.palette.primary.main,
    },
    {
      name: 'Memory',
      value: (metrics.memory_usage.ram_utilization_mb / 2048) * 100, // Assuming 2GB max
      fill: theme.palette.secondary.main,
    },
    {
      name: 'Storage',
      value: (metrics.storage_metrics.disk_usage_mb / 10240) * 100, // Assuming 10GB max
      fill: theme.palette.success.main,
    },
    {
      name: 'Network',
      value: Math.random() * 30 + 20, // Mock network usage
      fill: theme.palette.warning.main,
    },
  ];

  if (isLoading) {
    return (
      <Box>
        <LinearProgress sx={{ mb: 2 }} />
        <Typography>Loading performance metrics...</Typography>
      </Box>
    );
  }

  return (
    <Grid container spacing={3}>
      {/* Time Range Selector */}
      <Grid item xs={12}>
        <Paper sx={{ p: 2 }}>
          <Box display="flex" justifyContent="space-between" alignItems="center">
            <Typography variant="h6">Performance Metrics</Typography>
            <ToggleButtonGroup
              value={timeRange}
              exclusive
              onChange={(_, newValue) => newValue && setTimeRange(newValue)}
              size="small"
            >
              <ToggleButton value="1h">1H</ToggleButton>
              <ToggleButton value="6h">6H</ToggleButton>
              <ToggleButton value="24h">24H</ToggleButton>
              <ToggleButton value="7d">7D</ToggleButton>
            </ToggleButtonGroup>
          </Box>
        </Paper>
      </Grid>

      {/* Performance Summary Cards */}
      {performanceSummary.map((category, categoryIndex) => (
        <Grid item xs={12} md={6} lg={3} key={categoryIndex}>
          <Card>
            <CardHeader 
              title={category.category}
              titleTypographyProps={{ variant: 'subtitle1' }}
            />
            <CardContent>
              {category.metrics.map((metric, metricIndex) => (
                <Box key={metricIndex} display="flex" justifyContent="space-between" alignItems="center" mb={1}>
                  <Typography variant="body2" color="text.secondary">
                    {metric.name}
                  </Typography>
                  <Chip
                    label={metric.value}
                    size="small"
                    sx={{ 
                      backgroundColor: getStatusColor(metric.status),
                      color: 'white',
                      fontWeight: 'bold',
                    }}
                  />
                </Box>
              ))}
            </CardContent>
          </Card>
        </Grid>
      ))}

      {/* Query Performance Chart */}
      <Grid item xs={12} md={6}>
        <Card>
          <CardHeader title="Query Response Time" />
          <CardContent>
            <Box height={300}>
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={queryPerformanceData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="timestamp" />
                  <YAxis label={{ value: 'Time (ms)', angle: -90, position: 'insideLeft' }} />
                  <Tooltip formatter={(value) => [`${Number(value).toFixed(1)}ms`, 'Response Time']} />
                  <Line 
                    type="monotone" 
                    dataKey="value" 
                    stroke={theme.palette.primary.main}
                    strokeWidth={2}
                    dot={false}
                  />
                </LineChart>
              </ResponsiveContainer>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      {/* Throughput Chart */}
      <Grid item xs={12} md={6}>
        <Card>
          <CardHeader title="Query Throughput" />
          <CardContent>
            <Box height={300}>
              <ResponsiveContainer width="100%" height="100%">
                <AreaChart data={throughputData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="timestamp" />
                  <YAxis label={{ value: 'QPS', angle: -90, position: 'insideLeft' }} />
                  <Tooltip formatter={(value) => [`${Number(value).toFixed(0)}`, 'Queries/Second']} />
                  <Area 
                    type="monotone" 
                    dataKey="value" 
                    stroke={theme.palette.secondary.main}
                    fill={theme.palette.secondary.light}
                    fillOpacity={0.6}
                  />
                </AreaChart>
              </ResponsiveContainer>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      {/* Memory Usage Chart */}
      <Grid item xs={12} md={6}>
        <Card>
          <CardHeader title="Memory Utilization" />
          <CardContent>
            <Box height={300}>
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={memoryUsageData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="timestamp" />
                  <YAxis label={{ value: 'Memory (MB)', angle: -90, position: 'insideLeft' }} />
                  <Tooltip formatter={(value) => [`${Number(value).toFixed(0)}MB`, 'Memory Usage']} />
                  <Line 
                    type="monotone" 
                    dataKey="value" 
                    stroke={theme.palette.success.main}
                    strokeWidth={2}
                    dot={false}
                  />
                </LineChart>
              </ResponsiveContainer>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      {/* Cache Hit Rate Chart */}
      <Grid item xs={12} md={6}>
        <Card>
          <CardHeader title="Cache Hit Rate" />
          <CardContent>
            <Box height={300}>
              <ResponsiveContainer width="100%" height="100%">
                <AreaChart data={cacheHitData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="timestamp" />
                  <YAxis 
                    domain={[0, 100]}
                    label={{ value: 'Hit Rate (%)', angle: -90, position: 'insideLeft' }} 
                  />
                  <Tooltip formatter={(value) => [`${Number(value).toFixed(1)}%`, 'Cache Hit Rate']} />
                  <Area 
                    type="monotone" 
                    dataKey="value" 
                    stroke={theme.palette.warning.main}
                    fill={theme.palette.warning.light}
                    fillOpacity={0.6}
                  />
                </AreaChart>
              </ResponsiveContainer>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      {/* Resource Utilization Radial Chart */}
      <Grid item xs={12} md={6}>
        <Card>
          <CardHeader title="Resource Utilization" />
          <CardContent>
            <Box height={300}>
              <ResponsiveContainer width="100%" height="100%">
                <RadialBarChart cx="50%" cy="50%" innerRadius="20%" outerRadius="80%" data={resourceUtilization}>
                  <RadialBar dataKey="value" cornerRadius={10} fill="#8884d8" />
                  <Legend />
                  <Tooltip formatter={(value) => [`${Number(value).toFixed(1)}%`, 'Utilization']} />
                </RadialBarChart>
              </ResponsiveContainer>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      {/* Performance Alerts */}
      <Grid item xs={12} md={6}>
        <Card>
          <CardHeader title="Performance Alerts" />
          <CardContent>
            <Box>
              {metrics.query_performance.timeout_rate > 0.05 && (
                <Alert severity="warning" sx={{ mb: 1 }}>
                  High timeout rate detected: {(metrics.query_performance.timeout_rate * 100).toFixed(2)}%
                </Alert>
              )}
              {metrics.memory_usage.cache_hit_rate < 0.8 && (
                <Alert severity="info" sx={{ mb: 1 }}>
                  Cache hit rate below optimal: {(metrics.memory_usage.cache_hit_rate * 100).toFixed(1)}%
                </Alert>
              )}
              {metrics.query_performance.average_query_time_ms > 100 && (
                <Alert severity="warning" sx={{ mb: 1 }}>
                  Average query time above threshold: {metrics.query_performance.average_query_time_ms.toFixed(1)}ms
                </Alert>
              )}
              {metrics.storage_metrics.compression_ratio < 2 && (
                <Alert severity="info" sx={{ mb: 1 }}>
                  Low compression ratio: {metrics.storage_metrics.compression_ratio.toFixed(1)}:1
                </Alert>
              )}
              {/* Show success message if no alerts */}
              {metrics.query_performance.timeout_rate <= 0.05 && 
               metrics.memory_usage.cache_hit_rate >= 0.8 && 
               metrics.query_performance.average_query_time_ms <= 100 && 
               metrics.storage_metrics.compression_ratio >= 2 && (
                <Alert severity="success">
                  All performance metrics are within optimal ranges
                </Alert>
              )}
            </Box>
          </CardContent>
        </Card>
      </Grid>
    </Grid>
  );
};

export default PerformanceCharts;