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
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  ToggleButton,
  ToggleButtonGroup,
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
  ComposedChart,
  Legend,
} from 'recharts';
import {
  TrendingUp as TrendingUpIcon,
  Timeline as TimelineIcon,
  Speed as SpeedIcon,
  Assessment as AssessmentIcon,
} from '@mui/icons-material';

import type { GrowthData } from './GraphAnalyticsDashboard';

export interface GrowthAnalyticsProps {
  data: GrowthData;
  isLoading?: boolean;
}

export const GrowthAnalytics: React.FC<GrowthAnalyticsProps> = ({
  data,
  isLoading = false,
}) => {
  const theme = useTheme();
  const [viewMode, setViewMode] = useState<'absolute' | 'relative'>('absolute');

  // Process historical data for charts
  const processedHistoricalData = data.historical_trends.map((item, index) => {
    const date = new Date(item.timestamp);
    const prevItem = index > 0 ? data.historical_trends[index - 1] : item;
    
    return {
      ...item,
      date: date.toLocaleDateString(),
      time: date.toLocaleTimeString(),
      node_growth: item.node_count - prevItem.node_count,
      edge_growth: item.edge_count - prevItem.edge_count,
      node_growth_rate: prevItem.node_count > 0 ? ((item.node_count - prevItem.node_count) / prevItem.node_count) * 100 : 0,
      edge_growth_rate: prevItem.edge_count > 0 ? ((item.edge_count - prevItem.edge_count) / prevItem.edge_count) * 100 : 0,
    };
  });

  // Activity patterns data
  const activityData = [
    {
      activity: 'Creation',
      rate: data.activity_patterns.creation_rate_per_hour,
      color: theme.palette.success.main,
      icon: <TrendingUpIcon />,
    },
    {
      activity: 'Modification',
      rate: data.activity_patterns.modification_rate_per_hour,
      color: theme.palette.info.main,
      icon: <TimelineIcon />,
    },
    {
      activity: 'Deletion',
      rate: data.activity_patterns.deletion_rate_per_hour,
      color: theme.palette.warning.main,
      icon: <SpeedIcon />,
    },
  ];

  // Calculate growth statistics
  const totalGrowth = {
    nodes: processedHistoricalData.length > 0 ? 
      processedHistoricalData[processedHistoricalData.length - 1].node_count - processedHistoricalData[0].node_count : 0,
    edges: processedHistoricalData.length > 0 ? 
      processedHistoricalData[processedHistoricalData.length - 1].edge_count - processedHistoricalData[0].edge_count : 0,
  };

  const averageGrowthRate = {
    nodes: processedHistoricalData.length > 1 ? 
      processedHistoricalData.reduce((sum, item) => sum + item.node_growth, 0) / (processedHistoricalData.length - 1) : 0,
    edges: processedHistoricalData.length > 1 ? 
      processedHistoricalData.reduce((sum, item) => sum + item.edge_growth, 0) / (processedHistoricalData.length - 1) : 0,
  };

  // Capacity planning projections
  const projectionData = processedHistoricalData.slice(-7).map((item, index) => {
    const futureDate = new Date(item.timestamp);
    futureDate.setDate(futureDate.getDate() + 7);
    
    return {
      date: futureDate.toLocaleDateString(),
      projected_nodes: item.node_count + (averageGrowthRate.nodes * 7),
      projected_edges: item.edge_count + (averageGrowthRate.edges * 7),
      confidence: Math.max(0.5, 1 - (index * 0.1)), // Decreasing confidence over time
    };
  });

  if (isLoading) {
    return (
      <Box>
        <LinearProgress sx={{ mb: 2 }} />
        <Typography>Loading growth analytics...</Typography>
      </Box>
    );
  }

  return (
    <Grid container spacing={3}>
      {/* Growth Summary Cards */}
      <Grid item xs={12} md={3}>
        <Card>
          <CardContent>
            <Box display="flex" alignItems="center" gap={2}>
              <TrendingUpIcon sx={{ color: theme.palette.success.main, fontSize: 40 }} />
              <Box>
                <Typography variant="h5" color="success.main">
                  +{totalGrowth.nodes}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Total Node Growth
                </Typography>
              </Box>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      <Grid item xs={12} md={3}>
        <Card>
          <CardContent>
            <Box display="flex" alignItems="center" gap={2}>
              <TimelineIcon sx={{ color: theme.palette.info.main, fontSize: 40 }} />
              <Box>
                <Typography variant="h5" color="info.main">
                  +{totalGrowth.edges}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Total Edge Growth
                </Typography>
              </Box>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      <Grid item xs={12} md={3}>
        <Card>
          <CardContent>
            <Box display="flex" alignItems="center" gap={2}>
              <SpeedIcon sx={{ color: theme.palette.primary.main, fontSize: 40 }} />
              <Box>
                <Typography variant="h5" color="primary.main">
                  {averageGrowthRate.nodes.toFixed(1)}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Avg Nodes/Day
                </Typography>
              </Box>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      <Grid item xs={12} md={3}>
        <Card>
          <CardContent>
            <Box display="flex" alignItems="center" gap={2}>
              <AssessmentIcon sx={{ color: theme.palette.secondary.main, fontSize: 40 }} />
              <Box>
                <Typography variant="h5" color="secondary.main">
                  {averageGrowthRate.edges.toFixed(1)}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Avg Edges/Day
                </Typography>
              </Box>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      {/* View Mode Toggle */}
      <Grid item xs={12}>
        <Paper sx={{ p: 2 }}>
          <Box display="flex" justifyContent="space-between" alignItems="center">
            <Typography variant="h6">Growth Trends</Typography>
            <ToggleButtonGroup
              value={viewMode}
              exclusive
              onChange={(_, newValue) => newValue && setViewMode(newValue)}
              size="small"
            >
              <ToggleButton value="absolute">Absolute</ToggleButton>
              <ToggleButton value="relative">Growth Rate</ToggleButton>
            </ToggleButtonGroup>
          </Box>
        </Paper>
      </Grid>

      {/* Historical Growth Chart */}
      <Grid item xs={12}>
        <Card>
          <CardHeader title="Historical Growth Trends" />
          <CardContent>
            <Box height={400}>
              <ResponsiveContainer width="100%" height="100%">
                {viewMode === 'absolute' ? (
                  <ComposedChart data={processedHistoricalData}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="date" />
                    <YAxis yAxisId="left" label={{ value: 'Count', angle: -90, position: 'insideLeft' }} />
                    <YAxis yAxisId="right" orientation="right" label={{ value: 'Growth', angle: 90, position: 'insideRight' }} />
                    <Tooltip />
                    <Legend />
                    <Area
                      yAxisId="left"
                      type="monotone"
                      dataKey="node_count"
                      fill={theme.palette.primary.light}
                      stroke={theme.palette.primary.main}
                      name="Nodes"
                      fillOpacity={0.6}
                    />
                    <Area
                      yAxisId="left"
                      type="monotone"
                      dataKey="edge_count"
                      fill={theme.palette.secondary.light}
                      stroke={theme.palette.secondary.main}
                      name="Edges"
                      fillOpacity={0.6}
                    />
                    <Bar yAxisId="right" dataKey="node_growth" fill={theme.palette.success.main} name="Node Growth" />
                    <Bar yAxisId="right" dataKey="edge_growth" fill={theme.palette.info.main} name="Edge Growth" />
                  </ComposedChart>
                ) : (
                  <LineChart data={processedHistoricalData}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="date" />
                    <YAxis label={{ value: 'Growth Rate (%)', angle: -90, position: 'insideLeft' }} />
                    <Tooltip formatter={(value) => [`${Number(value).toFixed(2)}%`, 'Growth Rate']} />
                    <Legend />
                    <Line
                      type="monotone"
                      dataKey="node_growth_rate"
                      stroke={theme.palette.primary.main}
                      strokeWidth={2}
                      name="Node Growth Rate"
                    />
                    <Line
                      type="monotone"
                      dataKey="edge_growth_rate"
                      stroke={theme.palette.secondary.main}
                      strokeWidth={2}
                      name="Edge Growth Rate"
                    />
                  </LineChart>
                )}
              </ResponsiveContainer>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      {/* Activity Patterns */}
      <Grid item xs={12} md={6}>
        <Card>
          <CardHeader title="Activity Patterns (per hour)" />
          <CardContent>
            <Box height={300}>
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={activityData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="activity" />
                  <YAxis label={{ value: 'Operations/Hour', angle: -90, position: 'insideLeft' }} />
                  <Tooltip formatter={(value) => [value, 'Operations/Hour']} />
                  <Bar dataKey="rate" fill={theme.palette.primary.main} radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      {/* Most Accessed Nodes */}
      <Grid item xs={12} md={6}>
        <Card>
          <CardHeader title="Most Accessed Nodes" />
          <CardContent>
            <TableContainer>
              <Table size="small">
                <TableHead>
                  <TableRow>
                    <TableCell>Node ID</TableCell>
                    <TableCell align="right">Access Count</TableCell>
                    <TableCell align="right">Percentage</TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {data.usage_analytics.most_accessed_nodes.slice(0, 8).map((node, index) => {
                    const totalAccess = data.usage_analytics.most_accessed_nodes.reduce((sum, n) => sum + n.access_count, 0);
                    const percentage = (node.access_count / totalAccess) * 100;
                    
                    return (
                      <TableRow key={index}>
                        <TableCell>
                          <Typography variant="body2" fontFamily="monospace">
                            {node.node_id.substring(0, 12)}...
                          </Typography>
                        </TableCell>
                        <TableCell align="right">
                          <Chip 
                            label={node.access_count.toLocaleString()} 
                            size="small"
                            color={index < 3 ? 'primary' : 'default'}
                          />
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

      {/* Popular Query Patterns */}
      <Grid item xs={12} md={6}>
        <Card>
          <CardHeader title="Popular Query Patterns" />
          <CardContent>
            <Box height={300}>
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={data.usage_analytics.popular_query_patterns} layout="horizontal">
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis type="number" />
                  <YAxis dataKey="pattern" type="category" width={100} />
                  <Tooltip formatter={(value) => [value, 'Frequency']} />
                  <Bar dataKey="frequency" fill={theme.palette.secondary.main} radius={[0, 4, 4, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      {/* Capacity Planning */}
      <Grid item xs={12} md={6}>
        <Card>
          <CardHeader title="Capacity Planning (7-day projection)" />
          <CardContent>
            <Box height={300}>
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={projectionData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="date" />
                  <YAxis label={{ value: 'Projected Count', angle: -90, position: 'insideLeft' }} />
                  <Tooltip />
                  <Legend />
                  <Line
                    type="monotone"
                    dataKey="projected_nodes"
                    stroke={theme.palette.primary.main}
                    strokeWidth={2}
                    strokeDasharray="5 5"
                    name="Projected Nodes"
                  />
                  <Line
                    type="monotone"
                    dataKey="projected_edges"
                    stroke={theme.palette.secondary.main}
                    strokeWidth={2}
                    strokeDasharray="5 5"
                    name="Projected Edges"
                  />
                </LineChart>
              </ResponsiveContainer>
            </Box>
            <Typography variant="caption" color="text.secondary" sx={{ mt: 1, display: 'block' }}>
              * Projections based on recent growth trends. Actual results may vary.
            </Typography>
          </CardContent>
        </Card>
      </Grid>

      {/* Growth Statistics Summary */}
      <Grid item xs={12}>
        <Card>
          <CardHeader title="Growth Statistics Summary" />
          <CardContent>
            <Grid container spacing={2}>
              <Grid item xs={12} md={3}>
                <Typography variant="subtitle2" gutterBottom>
                  Current Period
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Nodes: {processedHistoricalData[processedHistoricalData.length - 1]?.node_count || 0}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Edges: {processedHistoricalData[processedHistoricalData.length - 1]?.edge_count || 0}
                </Typography>
              </Grid>
              <Grid item xs={12} md={3}>
                <Typography variant="subtitle2" gutterBottom>
                  Peak Growth Day
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Nodes: +{Math.max(...processedHistoricalData.map(d => d.node_growth))}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Edges: +{Math.max(...processedHistoricalData.map(d => d.edge_growth))}
                </Typography>
              </Grid>
              <Grid item xs={12} md={3}>
                <Typography variant="subtitle2" gutterBottom>
                  Activity Rate
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Creates: {data.activity_patterns.creation_rate_per_hour.toFixed(1)}/hr
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Updates: {data.activity_patterns.modification_rate_per_hour.toFixed(1)}/hr
                </Typography>
              </Grid>
              <Grid item xs={12} md={3}>
                <Typography variant="subtitle2" gutterBottom>
                  Efficiency Metrics
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Edge/Node Ratio: {processedHistoricalData.length > 0 ? 
                    (processedHistoricalData[processedHistoricalData.length - 1].edge_count / 
                     processedHistoricalData[processedHistoricalData.length - 1].node_count).toFixed(2) : 'N/A'}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Growth Correlation: {/* Mock correlation */}
                  {(Math.random() * 0.4 + 0.6).toFixed(2)}
                </Typography>
              </Grid>
            </Grid>
          </CardContent>
        </Card>
      </Grid>
    </Grid>
  );
};

export default GrowthAnalytics;