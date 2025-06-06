import React from 'react';
import {
  Card,
  CardContent,
  Typography,
  Box,
  Chip,
  LinearProgress,
  Grid,
  useTheme,
} from '@mui/material';
import {
  CheckCircle as HealthyIcon,
  Warning as WarningIcon,
  Error as ErrorIcon,
  Help as UnknownIcon,
  Computer as CoreIcon,
  Storage as DatabaseIcon,
  Search as IndexIcon,
  Api as ApiIcon,
} from '@mui/icons-material';
import type { HealthStatus, ServiceHealth } from '../../types/monitoring';

interface HealthIndicatorsProps {
  healthStatus: HealthStatus | null;
  loading?: boolean;
}

const getStatusIcon = (status: ServiceHealth['status']) => {
  switch (status) {
    case 'healthy':
      return <HealthyIcon color="success" />;
    case 'warning':
      return <WarningIcon color="warning" />;
    case 'critical':
      return <ErrorIcon color="error" />;
    default:
      return <UnknownIcon color="disabled" />;
  }
};

const getStatusColor = (status: ServiceHealth['status']) => {
  switch (status) {
    case 'healthy':
      return 'success';
    case 'warning':
      return 'warning';
    case 'critical':
      return 'error';
    default:
      return 'default';
  }
};

const getServiceIcon = (serviceName: string) => {
  switch (serviceName) {
    case 'vexfsCore':
      return <CoreIcon />;
    case 'database':
      return <DatabaseIcon />;
    case 'vectorIndex':
      return <IndexIcon />;
    case 'api':
      return <ApiIcon />;
    default:
      return <CoreIcon />;
  }
};

const formatUptime = (seconds: number): string => {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);

  if (days > 0) {
    return `${days}d ${hours}h ${minutes}m`;
  }
  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  return `${minutes}m`;
};

const formatLastCheck = (timestamp: string): string => {
  const now = new Date();
  const checkTime = new Date(timestamp);
  const diffMs = now.getTime() - checkTime.getTime();
  const diffMinutes = Math.floor(diffMs / 60000);

  if (diffMinutes < 1) return 'Just now';
  if (diffMinutes < 60) return `${diffMinutes}m ago`;
  const diffHours = Math.floor(diffMinutes / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  const diffDays = Math.floor(diffHours / 24);
  return `${diffDays}d ago`;
};

const ServiceCard: React.FC<{
  name: string;
  displayName: string;
  service: ServiceHealth;
}> = ({ name, displayName, service }) => {
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
              backgroundColor: theme.palette.grey[100],
              mr: 2,
            }}
          >
            {getServiceIcon(name)}
          </Box>
          <Box sx={{ flexGrow: 1 }}>
            <Typography variant="h6" component="h3">
              {displayName}
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Last check: {formatLastCheck(service.lastCheck)}
            </Typography>
          </Box>
          {getStatusIcon(service.status)}
        </Box>

        <Box sx={{ mb: 2 }}>
          <Chip
            label={service.status.toUpperCase()}
            color={getStatusColor(service.status) as any}
            size="small"
            sx={{ mb: 1 }}
          />
        </Box>

        {service.responseTime && (
          <Box sx={{ mb: 1 }}>
            <Typography variant="body2" color="text.secondary">
              Response Time: {service.responseTime.toFixed(1)}ms
            </Typography>
          </Box>
        )}

        {service.errorRate !== undefined && (
          <Box sx={{ mb: 1 }}>
            <Typography variant="body2" color="text.secondary">
              Error Rate: {service.errorRate.toFixed(2)}%
            </Typography>
            <LinearProgress
              variant="determinate"
              value={Math.min(service.errorRate, 100)}
              color={
                service.errorRate > 5
                  ? 'error'
                  : service.errorRate > 2
                    ? 'warning'
                    : 'success'
              }
              sx={{ height: 4, borderRadius: 2, mt: 0.5 }}
            />
          </Box>
        )}

        {service.message && (
          <Typography
            variant="body2"
            color="text.secondary"
            sx={{ fontStyle: 'italic' }}
          >
            {service.message}
          </Typography>
        )}
      </CardContent>
    </Card>
  );
};

const HealthIndicators: React.FC<HealthIndicatorsProps> = ({
  healthStatus,
  loading = false,
}) => {
  const theme = useTheme();

  if (loading) {
    return (
      <Box>
        <Card sx={{ mb: 3 }}>
          <CardContent>
            <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
              <Box
                sx={{
                  width: 150,
                  height: 32,
                  backgroundColor: theme.palette.grey[300],
                  borderRadius: 1,
                  mr: 2,
                }}
              />
              <Box
                sx={{
                  width: 80,
                  height: 24,
                  backgroundColor: theme.palette.grey[300],
                  borderRadius: 1,
                }}
              />
            </Box>
            <LinearProgress />
          </CardContent>
        </Card>

        <Grid container spacing={3}>
          {[1, 2, 3, 4].map(i => (
            <Grid item xs={12} sm={6} md={3} key={i}>
              <Card>
                <CardContent>
                  <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
                    <Box
                      sx={{
                        width: 40,
                        height: 40,
                        backgroundColor: theme.palette.grey[300],
                        borderRadius: 1,
                        mr: 2,
                      }}
                    />
                    <Box sx={{ flexGrow: 1 }}>
                      <Box
                        sx={{
                          width: 100,
                          height: 20,
                          backgroundColor: theme.palette.grey[300],
                          borderRadius: 1,
                          mb: 1,
                        }}
                      />
                      <Box
                        sx={{
                          width: 80,
                          height: 16,
                          backgroundColor: theme.palette.grey[200],
                          borderRadius: 1,
                        }}
                      />
                    </Box>
                  </Box>
                  <LinearProgress />
                </CardContent>
              </Card>
            </Grid>
          ))}
        </Grid>
      </Box>
    );
  }

  if (!healthStatus) {
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
        <Typography color="text.secondary">No health data available</Typography>
      </Box>
    );
  }

  const serviceDisplayNames = {
    vexfsCore: 'VexFS Core',
    database: 'Database',
    vectorIndex: 'Vector Index',
    api: 'API Server',
  };

  return (
    <Box>
      {/* Overall Health Status */}
      <Card sx={{ mb: 3 }}>
        <CardContent>
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
            }}
          >
            <Box sx={{ display: 'flex', alignItems: 'center' }}>
              <Typography variant="h5" component="h2" sx={{ mr: 2 }}>
                System Health
              </Typography>
              <Chip
                label={healthStatus.overall.toUpperCase()}
                color={getStatusColor(healthStatus.overall) as any}
                icon={getStatusIcon(healthStatus.overall)}
              />
            </Box>
            <Box sx={{ textAlign: 'right' }}>
              <Typography variant="body2" color="text.secondary">
                Uptime: {formatUptime(healthStatus.uptime)}
              </Typography>
              <Typography variant="body2" color="text.secondary">
                Last check: {formatLastCheck(healthStatus.lastHealthCheck)}
              </Typography>
            </Box>
          </Box>
        </CardContent>
      </Card>

      {/* Service Health Cards */}
      <Grid container spacing={3}>
        {Object.entries(healthStatus.services).map(([serviceName, service]) => (
          <Grid item xs={12} sm={6} md={3} key={serviceName}>
            <ServiceCard
              name={serviceName}
              displayName={
                serviceDisplayNames[
                  serviceName as keyof typeof serviceDisplayNames
                ]
              }
              service={service}
            />
          </Grid>
        ))}
      </Grid>
    </Box>
  );
};

export default HealthIndicators;
