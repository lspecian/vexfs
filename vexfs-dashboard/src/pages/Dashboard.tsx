import React from 'react';
import {
  Box,
  Grid,
  Card,
  CardContent,
  Typography,
  CircularProgress,
  Alert,
} from '@mui/material';
import {
  Storage as StorageIcon,
  DataObject as DataIcon,
  Speed as SpeedIcon,
  CloudDone as ServerIcon,
} from '@mui/icons-material';
import { useDashboardStats } from '../hooks/useVexFS';
import { formatNumber } from '../utils';

const StatCard: React.FC<{
  title: string;
  value: string | number;
  icon: React.ReactNode;
  color: string;
}> = ({ title, value, icon, color }) => (
  <Card sx={{ height: '100%' }}>
    <CardContent>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
        <Box
          sx={{
            p: 1,
            borderRadius: 2,
            backgroundColor: `${color}20`,
            color: color,
            mr: 2,
          }}
        >
          {icon}
        </Box>
        <Typography variant="h6" component="div" sx={{ flexGrow: 1 }}>
          {title}
        </Typography>
      </Box>
      <Typography variant="h4" component="div" sx={{ fontWeight: 600 }}>
        {value}
      </Typography>
    </CardContent>
  </Card>
);

const Dashboard: React.FC = () => {
  const { stats, loading, error } = useDashboardStats();

  if (loading) {
    return (
      <Box
        sx={{
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          height: '50vh',
        }}
      >
        <CircularProgress />
      </Box>
    );
  }

  if (error) {
    return (
      <Box>
        <Typography variant="h4" component="h1" sx={{ mb: 3, fontWeight: 600 }}>
          Dashboard Overview
        </Typography>
        <Alert severity="error" sx={{ mb: 3 }}>
          <Typography variant="h6" sx={{ mb: 1 }}>
            🔌 VexFS Server Connection Failed
          </Typography>
          <Typography variant="body2" sx={{ mb: 2 }}>
            The dashboard cannot connect to the VexFS server. This could be
            because:
          </Typography>
          <Typography variant="body2" component="div" sx={{ ml: 2 }}>
            • The VexFS server is not running on port 8000
            <br />
            • The server is starting up (please wait a moment)
            <br />
            • Network connectivity issues
            <br />
            • Server configuration problems
          </Typography>
          <Typography variant="body2" sx={{ mt: 2, fontWeight: 500 }}>
            💡 <strong>Quick Fix:</strong> Make sure the VexFS server is running
            on <code>http://localhost:8000</code>
          </Typography>
          <Typography variant="body2" sx={{ mt: 1, color: 'text.secondary' }}>
            Technical error: {error}
          </Typography>
        </Alert>

        {/* Show offline dashboard with helpful info */}
        <Grid container spacing={3}>
          <Grid size={{ xs: 12, sm: 6, md: 3 }}>
            <StatCard
              title="Collections"
              value="—"
              icon={<StorageIcon />}
              color="#9e9e9e"
            />
          </Grid>
          <Grid size={{ xs: 12, sm: 6, md: 3 }}>
            <StatCard
              title="Total Points"
              value="—"
              icon={<DataIcon />}
              color="#9e9e9e"
            />
          </Grid>
          <Grid size={{ xs: 12, sm: 6, md: 3 }}>
            <StatCard
              title="Storage Used"
              value="—"
              icon={<SpeedIcon />}
              color="#9e9e9e"
            />
          </Grid>
          <Grid size={{ xs: 12, sm: 6, md: 3 }}>
            <StatCard
              title="Server Status"
              value="Offline"
              icon={<ServerIcon />}
              color="#d32f2f"
            />
          </Grid>

          <Grid size={{ xs: 12 }}>
            <Card>
              <CardContent>
                <Typography variant="h5" component="h2" sx={{ mb: 2 }}>
                  🚀 Getting Started with VexFS
                </Typography>
                <Typography
                  variant="body1"
                  color="text.secondary"
                  sx={{ mb: 2 }}
                >
                  VexFS is a high-performance vector filesystem designed for
                  efficient storage and retrieval of vector embeddings.
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  <strong>To get started:</strong>
                  <br />
                  1. Start the VexFS server on port 8000
                  <br />
                  2. The dashboard will automatically connect
                  <br />
                  3. Begin managing your vector collections and data
                </Typography>
              </CardContent>
            </Card>
          </Grid>
        </Grid>
      </Box>
    );
  }

  return (
    <Box>
      <Typography variant="h4" component="h1" sx={{ mb: 3, fontWeight: 600 }}>
        Dashboard Overview
      </Typography>

      <Grid container spacing={3}>
        {/* Stats Cards */}
        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <StatCard
            title="Collections"
            value={formatNumber(stats.totalCollections)}
            icon={<StorageIcon />}
            color="#1976d2"
          />
        </Grid>
        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <StatCard
            title="Total Points"
            value={formatNumber(stats.totalPoints)}
            icon={<DataIcon />}
            color="#9c27b0"
          />
        </Grid>
        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <StatCard
            title="Storage Used"
            value={stats.totalStorage}
            icon={<SpeedIcon />}
            color="#2e7d32"
          />
        </Grid>
        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <StatCard
            title="Server Status"
            value={stats.serverStatus === 'online' ? 'Online' : 'Offline'}
            icon={<ServerIcon />}
            color={stats.serverStatus === 'online' ? '#2e7d32' : '#d32f2f'}
          />
        </Grid>

        {/* Welcome Card */}
        <Grid size={{ xs: 12 }}>
          <Card>
            <CardContent>
              <Typography variant="h5" component="h2" sx={{ mb: 2 }}>
                Welcome to VexFS Dashboard
              </Typography>
              <Typography variant="body1" color="text.secondary">
                VexFS is a high-performance vector filesystem designed for
                efficient storage and retrieval of vector embeddings. Use this
                dashboard to manage your collections, perform vector searches,
                and monitor system performance.
              </Typography>
            </CardContent>
          </Card>
        </Grid>

        {/* Quick Actions */}
        <Grid size={{ xs: 12, md: 6 }}>
          <Card>
            <CardContent>
              <Typography variant="h6" component="h3" sx={{ mb: 2 }}>
                Quick Actions
              </Typography>
              <Typography variant="body2" color="text.secondary">
                • Create a new collection
                <br />
                • Upload vector data
                <br />
                • Perform vector search
                <br />• View system metrics
              </Typography>
            </CardContent>
          </Card>
        </Grid>

        {/* Recent Activity */}
        <Grid size={{ xs: 12, md: 6 }}>
          <Card>
            <CardContent>
              <Typography variant="h6" component="h3" sx={{ mb: 2 }}>
                Recent Activity
              </Typography>
              <Typography variant="body2" color="text.secondary">
                Activity monitoring will be available in the next update.
                <br />
                <br />
                This section will show:
                <br />
                • Recent searches
                <br />
                • Collection updates
                <br />• System events
              </Typography>
            </CardContent>
          </Card>
        </Grid>
      </Grid>
    </Box>
  );
};

export default Dashboard;
