import React from 'react';
import {
  Box,
  Grid,
  Paper,
  Typography,
  Card,
  CardContent,
  CardHeader,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Chip,
  LinearProgress,
  useTheme,
} from '@mui/material';
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
  ScatterChart,
  Scatter,
} from 'recharts';

import type { GraphStatistics, GraphAnalytics } from '../../types/graph';

export interface StructureMetricsProps {
  statistics: GraphStatistics;
  analytics?: GraphAnalytics | null;
  isLoading?: boolean;
}

export const StructureMetrics: React.FC<StructureMetricsProps> = ({
  statistics,
  analytics,
  isLoading = false,
}) => {
  const theme = useTheme();

  // Prepare data for charts
  const nodeTypeData = Object.entries(statistics.node_types).map(([type, count]) => ({
    type,
    count,
    percentage: (count / statistics.node_count) * 100,
  }));

  const edgeTypeData = Object.entries(statistics.edge_types).map(([type, count]) => ({
    type,
    count,
    percentage: (count / statistics.edge_count) * 100,
  }));

  // Mock degree distribution data if analytics not available
  const degreeDistribution = analytics?.degree_distribution || [
    { degree: 0, count: Math.floor(statistics.node_count * 0.1) },
    { degree: 1, count: Math.floor(statistics.node_count * 0.2) },
    { degree: 2, count: Math.floor(statistics.node_count * 0.3) },
    { degree: 3, count: Math.floor(statistics.node_count * 0.2) },
    { degree: 4, count: Math.floor(statistics.node_count * 0.1) },
    { degree: 5, count: Math.floor(statistics.node_count * 0.05) },
    { degree: 6, count: Math.floor(statistics.node_count * 0.03) },
    { degree: 7, count: Math.floor(statistics.node_count * 0.02) },
  ];

  // Color schemes
  const nodeTypeColors = [
    theme.palette.primary.main,
    theme.palette.secondary.main,
    theme.palette.success.main,
    theme.palette.warning.main,
    theme.palette.error.main,
    theme.palette.info.main,
  ];

  const edgeTypeColors = [
    theme.palette.primary.light,
    theme.palette.secondary.light,
    theme.palette.success.light,
    theme.palette.warning.light,
    theme.palette.error.light,
  ];

  // Centrality measures data (mock if not available)
  const centralityData = analytics?.centrality_measures ? 
    Object.entries(analytics.centrality_measures.betweenness)
      .slice(0, 10)
      .map(([nodeId, betweenness]) => ({
        nodeId: nodeId.substring(0, 8) + '...',
        betweenness,
        closeness: analytics.centrality_measures.closeness[nodeId] || 0,
        eigenvector: analytics.centrality_measures.eigenvector[nodeId] || 0,
        pagerank: analytics.centrality_measures.pagerank[nodeId] || 0,
      })) : [];

  // Connectivity metrics
  const connectivityMetrics = [
    {
      metric: 'Graph Density',
      value: statistics.density,
      description: 'Ratio of actual edges to possible edges',
      color: theme.palette.primary.main,
    },
    {
      metric: 'Average Degree',
      value: statistics.average_degree,
      description: 'Average number of connections per node',
      color: theme.palette.secondary.main,
    },
    {
      metric: 'Clustering Coefficient',
      value: statistics.clustering_coefficient || 0,
      description: 'Measure of local clustering in the graph',
      color: theme.palette.success.main,
    },
    {
      metric: 'Connected Components',
      value: statistics.connected_components,
      description: 'Number of disconnected subgraphs',
      color: theme.palette.warning.main,
    },
  ];

  if (isLoading) {
    return (
      <Box>
        <LinearProgress sx={{ mb: 2 }} />
        <Typography>Loading structure metrics...</Typography>
      </Box>
    );
  }

  return (
    <Grid container spacing={3}>
      {/* Basic Statistics */}
      <Grid item xs={12} md={6}>
        <Card>
          <CardHeader title="Graph Overview" />
          <CardContent>
            <Table size="small">
              <TableBody>
                <TableRow>
                  <TableCell>Total Nodes</TableCell>
                  <TableCell align="right">
                    <Chip label={statistics.node_count.toLocaleString()} color="primary" />
                  </TableCell>
                </TableRow>
                <TableRow>
                  <TableCell>Total Edges</TableCell>
                  <TableCell align="right">
                    <Chip label={statistics.edge_count.toLocaleString()} color="secondary" />
                  </TableCell>
                </TableRow>
                <TableRow>
                  <TableCell>Largest Component</TableCell>
                  <TableCell align="right">
                    <Chip 
                      label={`${statistics.largest_component_size} nodes`} 
                      color="success" 
                    />
                  </TableCell>
                </TableRow>
                <TableRow>
                  <TableCell>Graph Diameter</TableCell>
                  <TableCell align="right">
                    <Chip 
                      label={statistics.diameter || 'N/A'} 
                      color="info" 
                    />
                  </TableCell>
                </TableRow>
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      </Grid>

      {/* Connectivity Metrics */}
      <Grid item xs={12} md={6}>
        <Card>
          <CardHeader title="Connectivity Metrics" />
          <CardContent>
            {connectivityMetrics.map((metric, index) => (
              <Box key={index} sx={{ mb: 2 }}>
                <Box display="flex" justifyContent="space-between" alignItems="center">
                  <Typography variant="body2" fontWeight="medium">
                    {metric.metric}
                  </Typography>
                  <Typography variant="body2" color={metric.color}>
                    {typeof metric.value === 'number' ? metric.value.toFixed(3) : metric.value}
                  </Typography>
                </Box>
                <Typography variant="caption" color="text.secondary">
                  {metric.description}
                </Typography>
                {typeof metric.value === 'number' && (
                  <LinearProgress
                    variant="determinate"
                    value={Math.min(metric.value * 100, 100)}
                    sx={{ mt: 0.5, height: 4, borderRadius: 2 }}
                  />
                )}
              </Box>
            ))}
          </CardContent>
        </Card>
      </Grid>

      {/* Node Type Distribution */}
      <Grid item xs={12} md={6}>
        <Card>
          <CardHeader title="Node Type Distribution" />
          <CardContent>
            <Box height={300}>
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={nodeTypeData}
                    cx="50%"
                    cy="50%"
                    labelLine={false}
                    label={({ type, percentage }) => `${type} (${percentage.toFixed(1)}%)`}
                    outerRadius={80}
                    fill="#8884d8"
                    dataKey="count"
                  >
                    {nodeTypeData.map((entry, index) => (
                      <Cell 
                        key={`cell-${index}`} 
                        fill={nodeTypeColors[index % nodeTypeColors.length]} 
                      />
                    ))}
                  </Pie>
                  <Tooltip formatter={(value) => [value, 'Count']} />
                </PieChart>
              </ResponsiveContainer>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      {/* Edge Type Distribution */}
      <Grid item xs={12} md={6}>
        <Card>
          <CardHeader title="Edge Type Distribution" />
          <CardContent>
            <Box height={300}>
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={edgeTypeData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="type" />
                  <YAxis />
                  <Tooltip />
                  <Bar dataKey="count" fill={theme.palette.primary.main} />
                </BarChart>
              </ResponsiveContainer>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      {/* Degree Distribution */}
      <Grid item xs={12}>
        <Card>
          <CardHeader title="Degree Distribution" />
          <CardContent>
            <Box height={400}>
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={degreeDistribution}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis 
                    dataKey="degree" 
                    label={{ value: 'Node Degree', position: 'insideBottom', offset: -5 }}
                  />
                  <YAxis 
                    label={{ value: 'Number of Nodes', angle: -90, position: 'insideLeft' }}
                  />
                  <Tooltip 
                    formatter={(value) => [value, 'Node Count']}
                    labelFormatter={(label) => `Degree: ${label}`}
                  />
                  <Bar 
                    dataKey="count" 
                    fill={theme.palette.secondary.main}
                    radius={[2, 2, 0, 0]}
                  />
                </BarChart>
              </ResponsiveContainer>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      {/* Centrality Measures */}
      {centralityData.length > 0 && (
        <Grid item xs={12}>
          <Card>
            <CardHeader title="Top Nodes by Centrality Measures" />
            <CardContent>
              <TableContainer>
                <Table size="small">
                  <TableHead>
                    <TableRow>
                      <TableCell>Node ID</TableCell>
                      <TableCell align="right">Betweenness</TableCell>
                      <TableCell align="right">Closeness</TableCell>
                      <TableCell align="right">Eigenvector</TableCell>
                      <TableCell align="right">PageRank</TableCell>
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    {centralityData.map((row, index) => (
                      <TableRow key={index}>
                        <TableCell component="th" scope="row">
                          <Typography variant="body2" fontFamily="monospace">
                            {row.nodeId}
                          </Typography>
                        </TableCell>
                        <TableCell align="right">{row.betweenness.toFixed(4)}</TableCell>
                        <TableCell align="right">{row.closeness.toFixed(4)}</TableCell>
                        <TableCell align="right">{row.eigenvector.toFixed(4)}</TableCell>
                        <TableCell align="right">{row.pagerank.toFixed(4)}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </TableContainer>
            </CardContent>
          </Card>
        </Grid>
      )}

      {/* Community Structure */}
      {analytics?.community_detection && (
        <Grid item xs={12} md={6}>
          <Card>
            <CardHeader title="Community Structure" />
            <CardContent>
              <Box>
                <Typography variant="body2" gutterBottom>
                  <strong>Communities Detected:</strong> {analytics.community_detection.communities.length}
                </Typography>
                <Typography variant="body2" gutterBottom>
                  <strong>Modularity Score:</strong> {analytics.community_detection.modularity.toFixed(3)}
                </Typography>
                
                <Box mt={2}>
                  <Typography variant="subtitle2" gutterBottom>
                    Community Sizes:
                  </Typography>
                  {analytics.community_detection.communities.slice(0, 5).map((community, index) => (
                    <Box key={index} display="flex" justifyContent="space-between" alignItems="center" mb={1}>
                      <Typography variant="body2">
                        Community {index + 1}
                      </Typography>
                      <Chip 
                        size="small" 
                        label={`${community.length} nodes`}
                        color={index < 3 ? 'primary' : 'default'}
                      />
                    </Box>
                  ))}
                </Box>
              </Box>
            </CardContent>
          </Card>
        </Grid>
      )}

      {/* Path Statistics */}
      {analytics?.shortest_paths_stats && (
        <Grid item xs={12} md={6}>
          <Card>
            <CardHeader title="Path Statistics" />
            <CardContent>
              <Table size="small">
                <TableBody>
                  <TableRow>
                    <TableCell>Average Path Length</TableCell>
                    <TableCell align="right">
                      {analytics.shortest_paths_stats.average_path_length.toFixed(2)}
                    </TableCell>
                  </TableRow>
                  <TableRow>
                    <TableCell>Graph Diameter</TableCell>
                    <TableCell align="right">
                      {analytics.shortest_paths_stats.diameter}
                    </TableCell>
                  </TableRow>
                  <TableRow>
                    <TableCell>Graph Radius</TableCell>
                    <TableCell align="right">
                      {analytics.shortest_paths_stats.radius}
                    </TableCell>
                  </TableRow>
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        </Grid>
      )}
    </Grid>
  );
};

export default StructureMetrics;