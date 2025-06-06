import React from 'react';
import {
  Card,
  CardContent,
  Typography,
  Grid,
  Box,
  LinearProgress,
  Chip,
  useTheme,
} from '@mui/material';
import {
  Memory as MemoryIcon,
  Storage as StorageIcon,
  Speed as CpuIcon,
  NetworkCheck as NetworkIcon,
} from '@mui/icons-material';
import type { SystemMetrics as SystemMetricsType } from '../../types/monitoring';

interface SystemMetricsProps {
  metrics: SystemMetricsType | null;
  loading?: boolean;
}

const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
};

const formatNumber = (num: number): string => {
  if (num >= 1000000) {
    return `${(num / 1000000).toFixed(1)}M`;
  }
  if (num >= 1000) {
    return `${(num / 1000).toFixed(1)}K`;
  }
  return num.toString();
};

const getProgressColor = (
  percentage: number
): 'primary' | 'warning' | 'error' => {
  if (percentage < 70) return 'primary';
  if (percentage < 90) return 'warning';
  return 'error';
};

const MetricCard: React.FC<{
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}> = ({ title, icon, children }) => {
  const theme = useTheme();

  return (
    <Card
      sx={{
        height: '100%',
        transition: 'all 0.2s ease-in-out',
        '&:hover': {
          transform: 'translateY(-2px)',
          boxShadow: theme.shadows[4],
        },
      }}
    >
      <CardContent>
        <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              width: 40,
              height: 40,
              borderRadius: 1,
              backgroundColor: theme.palette.primary.main,
              color: theme.palette.primary.contrastText,
              mr: 2,
            }}
          >
            {icon}
          </Box>
          <Typography variant="h6" component="h3">
            {title}
          </Typography>
        </Box>
        {children}
      </CardContent>
    </Card>
  );
};

const SystemMetrics: React.FC<SystemMetricsProps> = ({
  metrics,
  loading = false,
}) => {
  const theme = useTheme();

  if (loading) {
    return (
      <Grid container spacing={3}>
        {[1, 2, 3, 4].map(i => (
          <Grid item xs={12} sm={6} lg={3} key={i}>
            <Card>
              <CardContent>
                <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
                  <Box
                    sx={{
                      width: 40,
                      height: 40,
                      borderRadius: 1,
                      backgroundColor: theme.palette.grey[300],
                      mr: 2,
                    }}
                  />
                  <Box
                    sx={{
                      width: 100,
                      height: 24,
                      backgroundColor: theme.palette.grey[300],
                      borderRadius: 1,
                    }}
                  />
                </Box>
                <LinearProgress />
              </CardContent>
            </Card>
          </Grid>
        ))}
      </Grid>
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
          No metrics data available
        </Typography>
      </Box>
    );
  }

  return (
    <Grid container spacing={3}>
      {/* CPU Metrics */}
      <Grid item xs={12} sm={6} lg={3}>
        <MetricCard title="CPU" icon={<CpuIcon />}>
          <Box sx={{ mb: 2 }}>
            <Box
              sx={{ display: 'flex', justifyContent: 'space-between', mb: 1 }}
            >
              <Typography variant="body2" color="text.secondary">
                Usage
              </Typography>
              <Typography variant="body2" fontWeight="medium">
                {metrics.cpu.usage.toFixed(1)}%
              </Typography>
            </Box>
            <LinearProgress
              variant="determinate"
              value={metrics.cpu.usage}
              color={getProgressColor(metrics.cpu.usage)}
              sx={{ height: 8, borderRadius: 4 }}
            />
          </Box>
          <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 1 }}>
            <Typography variant="body2" color="text.secondary">
              Cores
            </Typography>
            <Chip label={metrics.cpu.cores} size="small" />
          </Box>
          <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
            <Typography variant="body2" color="text.secondary">
              Load Avg
            </Typography>
            <Typography variant="body2" fontWeight="medium">
              {metrics.cpu.loadAverage.map(load => load.toFixed(2)).join(', ')}
            </Typography>
          </Box>
        </MetricCard>
      </Grid>

      {/* Memory Metrics */}
      <Grid item xs={12} sm={6} lg={3}>
        <MetricCard title="Memory" icon={<MemoryIcon />}>
          <Box sx={{ mb: 2 }}>
            <Box
              sx={{ display: 'flex', justifyContent: 'space-between', mb: 1 }}
            >
              <Typography variant="body2" color="text.secondary">
                Usage
              </Typography>
              <Typography variant="body2" fontWeight="medium">
                {metrics.memory.percentage.toFixed(1)}%
              </Typography>
            </Box>
            <LinearProgress
              variant="determinate"
              value={metrics.memory.percentage}
              color={getProgressColor(metrics.memory.percentage)}
              sx={{ height: 8, borderRadius: 4 }}
            />
          </Box>
          <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 1 }}>
            <Typography variant="body2" color="text.secondary">
              Used
            </Typography>
            <Typography variant="body2" fontWeight="medium">
              {formatBytes(metrics.memory.used)}
            </Typography>
          </Box>
          <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
            <Typography variant="body2" color="text.secondary">
              Total
            </Typography>
            <Typography variant="body2" fontWeight="medium">
              {formatBytes(metrics.memory.total)}
            </Typography>
          </Box>
        </MetricCard>
      </Grid>

      {/* Disk Metrics */}
      <Grid item xs={12} sm={6} lg={3}>
        <MetricCard title="Storage" icon={<StorageIcon />}>
          <Box sx={{ mb: 2 }}>
            <Box
              sx={{ display: 'flex', justifyContent: 'space-between', mb: 1 }}
            >
              <Typography variant="body2" color="text.secondary">
                Usage
              </Typography>
              <Typography variant="body2" fontWeight="medium">
                {metrics.disk.percentage.toFixed(1)}%
              </Typography>
            </Box>
            <LinearProgress
              variant="determinate"
              value={metrics.disk.percentage}
              color={getProgressColor(metrics.disk.percentage)}
              sx={{ height: 8, borderRadius: 4 }}
            />
          </Box>
          <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 1 }}>
            <Typography variant="body2" color="text.secondary">
              Used
            </Typography>
            <Typography variant="body2" fontWeight="medium">
              {formatBytes(metrics.disk.used)}
            </Typography>
          </Box>
          <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
            <Typography variant="body2" color="text.secondary">
              Available
            </Typography>
            <Typography variant="body2" fontWeight="medium">
              {formatBytes(metrics.disk.available)}
            </Typography>
          </Box>
        </MetricCard>
      </Grid>

      {/* Network Metrics */}
      <Grid item xs={12} sm={6} lg={3}>
        <MetricCard title="Network" icon={<NetworkIcon />}>
          <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 1 }}>
            <Typography variant="body2" color="text.secondary">
              Bytes In
            </Typography>
            <Typography variant="body2" fontWeight="medium">
              {formatBytes(metrics.network.bytesIn)}
            </Typography>
          </Box>
          <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 1 }}>
            <Typography variant="body2" color="text.secondary">
              Bytes Out
            </Typography>
            <Typography variant="body2" fontWeight="medium">
              {formatBytes(metrics.network.bytesOut)}
            </Typography>
          </Box>
          <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 1 }}>
            <Typography variant="body2" color="text.secondary">
              Packets In
            </Typography>
            <Typography variant="body2" fontWeight="medium">
              {formatNumber(metrics.network.packetsIn)}
            </Typography>
          </Box>
          <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
            <Typography variant="body2" color="text.secondary">
              Packets Out
            </Typography>
            <Typography variant="body2" fontWeight="medium">
              {formatNumber(metrics.network.packetsOut)}
            </Typography>
          </Box>
        </MetricCard>
      </Grid>
    </Grid>
  );
};

export default SystemMetrics;
