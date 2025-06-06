import React from 'react';
import { Card, CardContent, Typography, Box, useTheme } from '@mui/material';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  AreaChart,
  Area,
  BarChart,
  Bar,
} from 'recharts';
import type {
  PerformanceMetrics,
  TimeSeriesData,
} from '../../types/monitoring';

interface PerformanceChartsProps {
  metrics: PerformanceMetrics | null;
  historicalData?: {
    responseTime: TimeSeriesData[];
    throughput: TimeSeriesData[];
    errorRate: TimeSeriesData[];
  };
  loading?: boolean;
}

const formatTime = (timestamp: string): string => {
  return new Date(timestamp).toLocaleTimeString('en-US', {
    hour: '2-digit',
    minute: '2-digit',
  });
};

const formatDuration = (ms: number): string => {
  if (ms < 1000) return `${ms.toFixed(1)}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
};

const formatNumber = (num: number): string => {
  if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
  if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
  return num.toString();
};

const ChartCard: React.FC<{
  title: string;
  children: React.ReactNode;
}> = ({ title, children }) => (
  <Card sx={{ height: '100%' }}>
    <CardContent>
      <Typography variant="h6" component="h3" sx={{ mb: 2 }}>
        {title}
      </Typography>
      <Box sx={{ height: 300 }}>{children}</Box>
    </CardContent>
  </Card>
);

const PerformanceCharts: React.FC<PerformanceChartsProps> = ({
  metrics,
  historicalData,
  loading = false,
}) => {
  const theme = useTheme();

  if (loading) {
    return (
      <Box
        sx={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(400px, 1fr))',
          gap: 3,
        }}
      >
        {[1, 2, 3, 4].map(i => (
          <Card key={i} sx={{ height: 400 }}>
            <CardContent>
              <Box
                sx={{
                  width: '60%',
                  height: 24,
                  backgroundColor: theme.palette.grey[300],
                  borderRadius: 1,
                  mb: 2,
                }}
              />
              <Box
                sx={{
                  width: '100%',
                  height: 300,
                  backgroundColor: theme.palette.grey[100],
                  borderRadius: 1,
                }}
              />
            </CardContent>
          </Card>
        ))}
      </Box>
    );
  }

  if (!metrics) {
    return (
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: 200,
          backgroundColor: theme.palette.grey[50],
          borderRadius: 1,
        }}
      >
        <Typography color="text.secondary">
          No performance data available
        </Typography>
      </Box>
    );
  }

  // Generate mock historical data if not provided
  const defaultHistoricalData = {
    responseTime: Array.from({ length: 20 }, (_, i) => ({
      timestamp: new Date(Date.now() - (20 - i) * 60000).toISOString(),
      value: Math.random() * 100 + 20,
      label: '',
    })),
    throughput: Array.from({ length: 20 }, (_, i) => ({
      timestamp: new Date(Date.now() - (20 - i) * 60000).toISOString(),
      value: Math.random() * 1000 + 100,
      label: '',
    })),
    errorRate: Array.from({ length: 20 }, (_, i) => ({
      timestamp: new Date(Date.now() - (20 - i) * 60000).toISOString(),
      value: Math.random() * 5,
      label: '',
    })),
  };

  const chartData = historicalData || defaultHistoricalData;

  // Response time distribution data
  const responseTimeDistribution = [
    { range: '0-10ms', count: Math.floor(Math.random() * 1000) },
    { range: '10-50ms', count: Math.floor(Math.random() * 2000) },
    { range: '50-100ms', count: Math.floor(Math.random() * 1500) },
    { range: '100-500ms', count: Math.floor(Math.random() * 800) },
    { range: '500ms+', count: Math.floor(Math.random() * 200) },
  ];

  return (
    <Box
      sx={{
        display: 'grid',
        gridTemplateColumns: 'repeat(auto-fit, minmax(400px, 1fr))',
        gap: 3,
      }}
    >
      {/* Response Time Trend */}
      <ChartCard title="Response Time Trend">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={chartData.responseTime}>
            <CartesianGrid
              strokeDasharray="3 3"
              stroke={theme.palette.divider}
            />
            <XAxis
              dataKey="timestamp"
              tickFormatter={formatTime}
              stroke={theme.palette.text.secondary}
            />
            <YAxis
              tickFormatter={formatDuration}
              stroke={theme.palette.text.secondary}
            />
            <Tooltip
              labelFormatter={value => formatTime(value as string)}
              formatter={(value: number) => [
                formatDuration(value),
                'Response Time',
              ]}
              contentStyle={{
                backgroundColor: theme.palette.background.paper,
                border: `1px solid ${theme.palette.divider}`,
                borderRadius: theme.shape.borderRadius,
              }}
            />
            <Line
              type="monotone"
              dataKey="value"
              stroke={theme.palette.primary.main}
              strokeWidth={2}
              dot={false}
              activeDot={{ r: 4, fill: theme.palette.primary.main }}
            />
          </LineChart>
        </ResponsiveContainer>
      </ChartCard>

      {/* Throughput Trend */}
      <ChartCard title="Query Throughput">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={chartData.throughput}>
            <CartesianGrid
              strokeDasharray="3 3"
              stroke={theme.palette.divider}
            />
            <XAxis
              dataKey="timestamp"
              tickFormatter={formatTime}
              stroke={theme.palette.text.secondary}
            />
            <YAxis
              tickFormatter={formatNumber}
              stroke={theme.palette.text.secondary}
            />
            <Tooltip
              labelFormatter={value => formatTime(value as string)}
              formatter={(value: number) => [
                `${formatNumber(value)} qps`,
                'Throughput',
              ]}
              contentStyle={{
                backgroundColor: theme.palette.background.paper,
                border: `1px solid ${theme.palette.divider}`,
                borderRadius: theme.shape.borderRadius,
              }}
            />
            <Area
              type="monotone"
              dataKey="value"
              stroke={theme.palette.secondary.main}
              fill={theme.palette.secondary.main}
              fillOpacity={0.3}
            />
          </AreaChart>
        </ResponsiveContainer>
      </ChartCard>

      {/* Error Rate */}
      <ChartCard title="Error Rate">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={chartData.errorRate}>
            <CartesianGrid
              strokeDasharray="3 3"
              stroke={theme.palette.divider}
            />
            <XAxis
              dataKey="timestamp"
              tickFormatter={formatTime}
              stroke={theme.palette.text.secondary}
            />
            <YAxis
              tickFormatter={value => `${value}%`}
              stroke={theme.palette.text.secondary}
            />
            <Tooltip
              labelFormatter={value => formatTime(value as string)}
              formatter={(value: number) => [
                `${value.toFixed(2)}%`,
                'Error Rate',
              ]}
              contentStyle={{
                backgroundColor: theme.palette.background.paper,
                border: `1px solid ${theme.palette.divider}`,
                borderRadius: theme.shape.borderRadius,
              }}
            />
            <Line
              type="monotone"
              dataKey="value"
              stroke={theme.palette.error.main}
              strokeWidth={2}
              dot={false}
              activeDot={{ r: 4, fill: theme.palette.error.main }}
            />
          </LineChart>
        </ResponsiveContainer>
      </ChartCard>

      {/* Response Time Distribution */}
      <ChartCard title="Response Time Distribution">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={responseTimeDistribution}>
            <CartesianGrid
              strokeDasharray="3 3"
              stroke={theme.palette.divider}
            />
            <XAxis dataKey="range" stroke={theme.palette.text.secondary} />
            <YAxis
              tickFormatter={formatNumber}
              stroke={theme.palette.text.secondary}
            />
            <Tooltip
              formatter={(value: number) => [formatNumber(value), 'Requests']}
              contentStyle={{
                backgroundColor: theme.palette.background.paper,
                border: `1px solid ${theme.palette.divider}`,
                borderRadius: theme.shape.borderRadius,
              }}
            />
            <Bar
              dataKey="count"
              fill={theme.palette.info.main}
              radius={[4, 4, 0, 0]}
            />
          </BarChart>
        </ResponsiveContainer>
      </ChartCard>
    </Box>
  );
};

export default PerformanceCharts;
