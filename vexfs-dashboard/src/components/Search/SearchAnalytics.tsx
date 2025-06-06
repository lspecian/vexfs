import React from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Grid,
  Paper,
  LinearProgress,
  Chip,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
} from '@mui/material';
import {
  Analytics as AnalyticsIcon,
  Speed as SpeedIcon,
  Search as SearchIcon,
  Storage as StorageIcon,
  TrendingUp as TrendingUpIcon,
} from '@mui/icons-material';
import {
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
  LineChart,
  Line,
} from 'recharts';
import { formatNumber } from '../../utils';
import type { SearchAnalytics } from '../../types';

interface SearchAnalyticsProps {
  analytics: SearchAnalytics;
  loading?: boolean;
}

const COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff7300', '#00ff00'];

const SearchAnalyticsComponent: React.FC<SearchAnalyticsProps> = ({
  analytics,
  loading = false,
}) => {
  const performanceData = [
    {
      name: 'Fast (<100ms)',
      value: analytics.performanceMetrics.fastQueries,
      color: '#4caf50',
    },
    {
      name: 'Medium (100ms-1s)',
      value: analytics.performanceMetrics.mediumQueries,
      color: '#ff9800',
    },
    {
      name: 'Slow (>1s)',
      value: analytics.performanceMetrics.slowQueries,
      color: '#f44336',
    },
  ];

  const hourlyData = analytics.searchPatterns.hourlyDistribution.map(
    (count, hour) => ({
      hour: `${hour}:00`,
      searches: count,
    })
  );

  const dailyData = analytics.searchPatterns.dailyDistribution.map(
    (count, day) => ({
      day: ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'][day],
      searches: count,
    })
  );

  if (loading) {
    return (
      <Card>
        <CardContent>
          <Typography variant="h6" gutterBottom>
            Loading Analytics...
          </Typography>
          <LinearProgress />
        </CardContent>
      </Card>
    );
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
      {/* Header */}
      <Card>
        <CardContent>
          <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
            <AnalyticsIcon sx={{ mr: 1 }} />
            <Typography variant="h6" sx={{ fontWeight: 600 }}>
              Search Analytics
            </Typography>
          </Box>
          <Typography variant="body2" color="text.secondary">
            Insights into search performance, usage patterns, and trends across
            your VexFS collections.
          </Typography>
        </CardContent>
      </Card>

      {/* Key Metrics */}
      <Grid container spacing={3}>
        <Grid item xs={12} sm={6} md={3}>
          <Paper sx={{ p: 3, textAlign: 'center' }}>
            <SearchIcon sx={{ fontSize: 40, color: 'primary.main', mb: 1 }} />
            <Typography variant="h4" sx={{ fontWeight: 600, mb: 1 }}>
              {formatNumber(analytics.totalSearches)}
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Total Searches
            </Typography>
          </Paper>
        </Grid>

        <Grid item xs={12} sm={6} md={3}>
          <Paper sx={{ p: 3, textAlign: 'center' }}>
            <SpeedIcon sx={{ fontSize: 40, color: 'success.main', mb: 1 }} />
            <Typography variant="h4" sx={{ fontWeight: 600, mb: 1 }}>
              {analytics.averageExecutionTime.toFixed(0)}ms
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Avg Response Time
            </Typography>
          </Paper>
        </Grid>

        <Grid item xs={12} sm={6} md={3}>
          <Paper sx={{ p: 3, textAlign: 'center' }}>
            <TrendingUpIcon
              sx={{ fontSize: 40, color: 'warning.main', mb: 1 }}
            />
            <Typography variant="h4" sx={{ fontWeight: 600, mb: 1 }}>
              {analytics.popularSearchTypes.length > 0
                ? analytics.popularSearchTypes[0].percentage.toFixed(1)
                : 0}
              %
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Most Popular Type
            </Typography>
          </Paper>
        </Grid>

        <Grid item xs={12} sm={6} md={3}>
          <Paper sx={{ p: 3, textAlign: 'center' }}>
            <StorageIcon sx={{ fontSize: 40, color: 'info.main', mb: 1 }} />
            <Typography variant="h4" sx={{ fontWeight: 600, mb: 1 }}>
              {analytics.topCollections.length}
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Active Collections
            </Typography>
          </Paper>
        </Grid>
      </Grid>

      {/* Charts Row */}
      <Grid container spacing={3}>
        {/* Search Types Distribution */}
        <Grid item xs={12} md={6}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom sx={{ fontWeight: 600 }}>
                Search Types Distribution
              </Typography>
              <Box sx={{ height: 300 }}>
                <ResponsiveContainer width="100%" height="100%">
                  <PieChart>
                    <Pie
                      data={analytics.popularSearchTypes}
                      cx="50%"
                      cy="50%"
                      labelLine={false}
                      label={({ type, percentage }) =>
                        `${type}: ${percentage.toFixed(1)}%`
                      }
                      outerRadius={80}
                      fill="#8884d8"
                      dataKey="count"
                    >
                      {analytics.popularSearchTypes.map((entry, index) => (
                        <Cell
                          key={`cell-${index}`}
                          fill={COLORS[index % COLORS.length]}
                        />
                      ))}
                    </Pie>
                    <Tooltip />
                  </PieChart>
                </ResponsiveContainer>
              </Box>
            </CardContent>
          </Card>
        </Grid>

        {/* Performance Distribution */}
        <Grid item xs={12} md={6}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom sx={{ fontWeight: 600 }}>
                Performance Distribution
              </Typography>
              <Box sx={{ height: 300 }}>
                <ResponsiveContainer width="100%" height="100%">
                  <BarChart data={performanceData}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="name" />
                    <YAxis />
                    <Tooltip />
                    <Bar dataKey="value" fill="#8884d8">
                      {performanceData.map((entry, index) => (
                        <Cell key={`cell-${index}`} fill={entry.color} />
                      ))}
                    </Bar>
                  </BarChart>
                </ResponsiveContainer>
              </Box>
            </CardContent>
          </Card>
        </Grid>
      </Grid>

      {/* Usage Patterns */}
      <Grid container spacing={3}>
        {/* Hourly Distribution */}
        <Grid item xs={12} md={6}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom sx={{ fontWeight: 600 }}>
                Hourly Search Pattern
              </Typography>
              <Box sx={{ height: 300 }}>
                <ResponsiveContainer width="100%" height="100%">
                  <LineChart data={hourlyData}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="hour" />
                    <YAxis />
                    <Tooltip />
                    <Line
                      type="monotone"
                      dataKey="searches"
                      stroke="#8884d8"
                      strokeWidth={2}
                    />
                  </LineChart>
                </ResponsiveContainer>
              </Box>
            </CardContent>
          </Card>
        </Grid>

        {/* Daily Distribution */}
        <Grid item xs={12} md={6}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom sx={{ fontWeight: 600 }}>
                Daily Search Pattern
              </Typography>
              <Box sx={{ height: 300 }}>
                <ResponsiveContainer width="100%" height="100%">
                  <BarChart data={dailyData}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="day" />
                    <YAxis />
                    <Tooltip />
                    <Bar dataKey="searches" fill="#82ca9d" />
                  </BarChart>
                </ResponsiveContainer>
              </Box>
            </CardContent>
          </Card>
        </Grid>
      </Grid>

      {/* Detailed Tables */}
      <Grid container spacing={3}>
        {/* Top Collections */}
        <Grid item xs={12} md={6}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom sx={{ fontWeight: 600 }}>
                Most Searched Collections
              </Typography>
              <TableContainer>
                <Table size="small">
                  <TableHead>
                    <TableRow>
                      <TableCell>Collection</TableCell>
                      <TableCell align="right">Searches</TableCell>
                      <TableCell align="right">Percentage</TableCell>
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    {analytics.topCollections.map((collection, index) => {
                      const percentage =
                        (collection.searchCount / analytics.totalSearches) *
                        100;
                      return (
                        <TableRow key={collection.collectionId}>
                          <TableCell>
                            <Box sx={{ display: 'flex', alignItems: 'center' }}>
                              <Chip
                                label={index + 1}
                                size="small"
                                sx={{ mr: 1, minWidth: 24 }}
                              />
                              {collection.collectionId}
                            </Box>
                          </TableCell>
                          <TableCell align="right">
                            {formatNumber(collection.searchCount)}
                          </TableCell>
                          <TableCell align="right">
                            {percentage.toFixed(1)}%
                          </TableCell>
                        </TableRow>
                      );
                    })}
                  </TableBody>
                </Table>
              </TableContainer>
            </CardContent>
          </Card>
        </Grid>

        {/* Search Types Breakdown */}
        <Grid item xs={12} md={6}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom sx={{ fontWeight: 600 }}>
                Search Types Breakdown
              </Typography>
              <TableContainer>
                <Table size="small">
                  <TableHead>
                    <TableRow>
                      <TableCell>Type</TableCell>
                      <TableCell align="right">Count</TableCell>
                      <TableCell align="right">Percentage</TableCell>
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    {analytics.popularSearchTypes.map((type, index) => (
                      <TableRow key={type.type}>
                        <TableCell>
                          <Box sx={{ display: 'flex', alignItems: 'center' }}>
                            <Box
                              sx={{
                                width: 12,
                                height: 12,
                                borderRadius: '50%',
                                backgroundColor: COLORS[index % COLORS.length],
                                mr: 1,
                              }}
                            />
                            {type.type}
                          </Box>
                        </TableCell>
                        <TableCell align="right">
                          {formatNumber(type.count)}
                        </TableCell>
                        <TableCell align="right">
                          {type.percentage.toFixed(1)}%
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </TableContainer>
            </CardContent>
          </Card>
        </Grid>
      </Grid>

      {/* Performance Summary */}
      <Card>
        <CardContent>
          <Typography variant="h6" gutterBottom sx={{ fontWeight: 600 }}>
            Performance Summary
          </Typography>
          <Grid container spacing={3}>
            <Grid item xs={12} md={4}>
              <Box sx={{ textAlign: 'center' }}>
                <Typography
                  variant="h3"
                  color="success.main"
                  sx={{ fontWeight: 600 }}
                >
                  {formatNumber(analytics.performanceMetrics.fastQueries)}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Fast Queries (&lt;100ms)
                </Typography>
                <LinearProgress
                  variant="determinate"
                  value={
                    (analytics.performanceMetrics.fastQueries /
                      analytics.totalSearches) *
                    100
                  }
                  color="success"
                  sx={{ mt: 1 }}
                />
              </Box>
            </Grid>
            <Grid item xs={12} md={4}>
              <Box sx={{ textAlign: 'center' }}>
                <Typography
                  variant="h3"
                  color="warning.main"
                  sx={{ fontWeight: 600 }}
                >
                  {formatNumber(analytics.performanceMetrics.mediumQueries)}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Medium Queries (100ms-1s)
                </Typography>
                <LinearProgress
                  variant="determinate"
                  value={
                    (analytics.performanceMetrics.mediumQueries /
                      analytics.totalSearches) *
                    100
                  }
                  color="warning"
                  sx={{ mt: 1 }}
                />
              </Box>
            </Grid>
            <Grid item xs={12} md={4}>
              <Box sx={{ textAlign: 'center' }}>
                <Typography
                  variant="h3"
                  color="error.main"
                  sx={{ fontWeight: 600 }}
                >
                  {formatNumber(analytics.performanceMetrics.slowQueries)}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Slow Queries (&gt;1s)
                </Typography>
                <LinearProgress
                  variant="determinate"
                  value={
                    (analytics.performanceMetrics.slowQueries /
                      analytics.totalSearches) *
                    100
                  }
                  color="error"
                  sx={{ mt: 1 }}
                />
              </Box>
            </Grid>
          </Grid>
        </CardContent>
      </Card>
    </Box>
  );
};

export default SearchAnalyticsComponent;
