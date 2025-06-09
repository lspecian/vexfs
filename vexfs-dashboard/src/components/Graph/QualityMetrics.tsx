import React from 'react';
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
  Alert,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  CircularProgress,
} from '@mui/material';
import {
  PieChart,
  Pie,
  Cell,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  RadialBarChart,
  RadialBar,
  Legend,
  LineChart,
  Line,
} from 'recharts';
import {
  CheckCircle as CheckCircleIcon,
  Warning as WarningIcon,
  Error as ErrorIcon,
  VerifiedUser as VerifiedIcon,
  Assessment as AssessmentIcon,
  Security as SecurityIcon,
} from '@mui/icons-material';

import type { DataQualityMetrics } from './GraphAnalyticsDashboard';

export interface QualityMetricsProps {
  metrics: DataQualityMetrics;
  isLoading?: boolean;
}

export const QualityMetrics: React.FC<QualityMetricsProps> = ({
  metrics,
  isLoading = false,
}) => {
  const theme = useTheme();

  // Quality score calculation
  const overallQualityScore = (
    (metrics.completeness_scores.overall_completeness_percentage +
     metrics.consistency_metrics.schema_compliance_rate +
     metrics.consistency_metrics.referential_integrity_score +
     metrics.semantic_coherence.embedding_quality_score +
     metrics.semantic_coherence.relationship_accuracy_score +
     metrics.semantic_coherence.semantic_consistency_score) / 6
  ) * 100;

  // Quality categories
  const qualityCategories = [
    {
      name: 'Completeness',
      score: metrics.completeness_scores.overall_completeness_percentage * 100,
      color: theme.palette.primary.main,
      icon: <CheckCircleIcon />,
      description: 'Data completeness across nodes and edges',
    },
    {
      name: 'Consistency',
      score: ((metrics.consistency_metrics.schema_compliance_rate + metrics.consistency_metrics.referential_integrity_score) / 2) * 100,
      color: theme.palette.secondary.main,
      icon: <VerifiedIcon />,
      description: 'Schema compliance and referential integrity',
    },
    {
      name: 'Semantic Coherence',
      score: ((metrics.semantic_coherence.embedding_quality_score + metrics.semantic_coherence.relationship_accuracy_score + metrics.semantic_coherence.semantic_consistency_score) / 3) * 100,
      color: theme.palette.success.main,
      icon: <AssessmentIcon />,
      description: 'Semantic relationship quality and accuracy',
    },
  ];

  // Completeness breakdown data
  const completenessData = [
    {
      category: 'Complete Nodes',
      count: metrics.completeness_scores.nodes_with_complete_metadata,
      total: metrics.completeness_scores.nodes_with_complete_metadata + Math.floor(metrics.completeness_scores.nodes_with_complete_metadata * 0.2),
      percentage: (metrics.completeness_scores.nodes_with_complete_metadata / (metrics.completeness_scores.nodes_with_complete_metadata + Math.floor(metrics.completeness_scores.nodes_with_complete_metadata * 0.2))) * 100,
    },
    {
      category: 'Complete Edges',
      count: metrics.completeness_scores.edges_with_complete_metadata,
      total: metrics.completeness_scores.edges_with_complete_metadata + Math.floor(metrics.completeness_scores.edges_with_complete_metadata * 0.15),
      percentage: (metrics.completeness_scores.edges_with_complete_metadata / (metrics.completeness_scores.edges_with_complete_metadata + Math.floor(metrics.completeness_scores.edges_with_complete_metadata * 0.15))) * 100,
    },
  ];

  // Quality issues data
  const qualityIssues = [
    {
      type: 'Duplicate Nodes',
      count: metrics.consistency_metrics.duplicate_detection_count,
      severity: metrics.consistency_metrics.duplicate_detection_count > 10 ? 'high' : metrics.consistency_metrics.duplicate_detection_count > 5 ? 'medium' : 'low',
      impact: 'Data redundancy and storage inefficiency',
    },
    {
      type: 'Schema Violations',
      count: Math.floor((1 - metrics.consistency_metrics.schema_compliance_rate) * 100),
      severity: metrics.consistency_metrics.schema_compliance_rate < 0.9 ? 'high' : metrics.consistency_metrics.schema_compliance_rate < 0.95 ? 'medium' : 'low',
      impact: 'Inconsistent data structure',
    },
    {
      type: 'Broken References',
      count: Math.floor((1 - metrics.consistency_metrics.referential_integrity_score) * 50),
      severity: metrics.consistency_metrics.referential_integrity_score < 0.95 ? 'high' : metrics.consistency_metrics.referential_integrity_score < 0.98 ? 'medium' : 'low',
      impact: 'Invalid relationships between entities',
    },
  ];

  // Semantic quality breakdown
  const semanticQualityData = [
    {
      metric: 'Embedding Quality',
      score: metrics.semantic_coherence.embedding_quality_score * 100,
      description: 'Quality of vector embeddings',
    },
    {
      metric: 'Relationship Accuracy',
      score: metrics.semantic_coherence.relationship_accuracy_score * 100,
      description: 'Accuracy of semantic relationships',
    },
    {
      metric: 'Semantic Consistency',
      score: metrics.semantic_coherence.semantic_consistency_score * 100,
      description: 'Consistency across semantic mappings',
    },
  ];

  // Quality trend data (mock historical data)
  const qualityTrendData = Array.from({ length: 30 }, (_, i) => {
    const date = new Date();
    date.setDate(date.getDate() - (29 - i));
    return {
      date: date.toLocaleDateString(),
      completeness: Math.max(60, metrics.completeness_scores.overall_completeness_percentage * 100 + (Math.random() - 0.5) * 10),
      consistency: Math.max(70, ((metrics.consistency_metrics.schema_compliance_rate + metrics.consistency_metrics.referential_integrity_score) / 2) * 100 + (Math.random() - 0.5) * 8),
      semantic: Math.max(65, ((metrics.semantic_coherence.embedding_quality_score + metrics.semantic_coherence.relationship_accuracy_score + metrics.semantic_coherence.semantic_consistency_score) / 3) * 100 + (Math.random() - 0.5) * 12),
    };
  });

  const getQualityColor = (score: number) => {
    if (score >= 90) return theme.palette.success.main;
    if (score >= 75) return theme.palette.info.main;
    if (score >= 60) return theme.palette.warning.main;
    return theme.palette.error.main;
  };

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'high': return theme.palette.error.main;
      case 'medium': return theme.palette.warning.main;
      case 'low': return theme.palette.success.main;
      default: return theme.palette.text.secondary;
    }
  };

  const getSeverityIcon = (severity: string) => {
    switch (severity) {
      case 'high': return <ErrorIcon />;
      case 'medium': return <WarningIcon />;
      case 'low': return <CheckCircleIcon />;
      default: return <AssessmentIcon />;
    }
  };

  if (isLoading) {
    return (
      <Box>
        <LinearProgress sx={{ mb: 2 }} />
        <Typography>Loading quality metrics...</Typography>
      </Box>
    );
  }

  return (
    <Grid container spacing={3}>
      {/* Overall Quality Score */}
      <Grid item xs={12} md={4}>
        <Card>
          <CardHeader title="Overall Quality Score" />
          <CardContent>
            <Box display="flex" flexDirection="column" alignItems="center">
              <Box position="relative" display="inline-flex" mb={2}>
                <CircularProgress
                  variant="determinate"
                  value={overallQualityScore}
                  size={120}
                  thickness={6}
                  sx={{ color: getQualityColor(overallQualityScore) }}
                />
                <Box
                  position="absolute"
                  top={0}
                  left={0}
                  bottom={0}
                  right={0}
                  display="flex"
                  alignItems="center"
                  justifyContent="center"
                >
                  <Typography variant="h4" component="div" color="text.secondary">
                    {overallQualityScore.toFixed(0)}%
                  </Typography>
                </Box>
              </Box>
              <Typography variant="body2" color="text.secondary" textAlign="center">
                {overallQualityScore >= 90 ? 'Excellent' : 
                 overallQualityScore >= 75 ? 'Good' : 
                 overallQualityScore >= 60 ? 'Fair' : 'Needs Improvement'}
              </Typography>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      {/* Quality Categories */}
      <Grid item xs={12} md={8}>
        <Card>
          <CardHeader title="Quality Categories" />
          <CardContent>
            {qualityCategories.map((category, index) => (
              <Box key={index} mb={2}>
                <Box display="flex" justifyContent="space-between" alignItems="center" mb={1}>
                  <Box display="flex" alignItems="center" gap={1}>
                    <Box sx={{ color: category.color }}>
                      {category.icon}
                    </Box>
                    <Typography variant="subtitle2">
                      {category.name}
                    </Typography>
                  </Box>
                  <Chip
                    label={`${category.score.toFixed(1)}%`}
                    size="small"
                    sx={{ 
                      backgroundColor: getQualityColor(category.score),
                      color: 'white',
                      fontWeight: 'bold',
                    }}
                  />
                </Box>
                <LinearProgress
                  variant="determinate"
                  value={category.score}
                  sx={{ 
                    height: 8, 
                    borderRadius: 4,
                    backgroundColor: theme.palette.grey[200],
                    '& .MuiLinearProgress-bar': {
                      backgroundColor: getQualityColor(category.score),
                    },
                  }}
                />
                <Typography variant="caption" color="text.secondary">
                  {category.description}
                </Typography>
              </Box>
            ))}
          </CardContent>
        </Card>
      </Grid>

      {/* Completeness Breakdown */}
      <Grid item xs={12} md={6}>
        <Card>
          <CardHeader title="Data Completeness" />
          <CardContent>
            <Box height={300}>
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={completenessData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="category" />
                  <YAxis label={{ value: 'Count', angle: -90, position: 'insideLeft' }} />
                  <Tooltip 
                    formatter={(value, name) => [
                      name === 'count' ? `${value} complete` : `${value} total`,
                      name === 'count' ? 'Complete' : 'Total'
                    ]}
                  />
                  <Bar dataKey="total" fill={theme.palette.grey[300]} name="Total" />
                  <Bar dataKey="count" fill={theme.palette.primary.main} name="Complete" />
                </BarChart>
              </ResponsiveContainer>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      {/* Semantic Quality */}
      <Grid item xs={12} md={6}>
        <Card>
          <CardHeader title="Semantic Quality Metrics" />
          <CardContent>
            <Box height={300}>
              <ResponsiveContainer width="100%" height="100%">
                <RadialBarChart cx="50%" cy="50%" innerRadius="20%" outerRadius="80%" data={semanticQualityData}>
                  <RadialBar dataKey="score" cornerRadius={10} fill="#8884d8" />
                  <Legend />
                  <Tooltip formatter={(value) => [`${Number(value).toFixed(1)}%`, 'Score']} />
                </RadialBarChart>
              </ResponsiveContainer>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      {/* Quality Issues */}
      <Grid item xs={12}>
        <Card>
          <CardHeader title="Quality Issues" />
          <CardContent>
            <TableContainer>
              <Table>
                <TableHead>
                  <TableRow>
                    <TableCell>Issue Type</TableCell>
                    <TableCell align="center">Severity</TableCell>
                    <TableCell align="right">Count</TableCell>
                    <TableCell>Impact</TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {qualityIssues.map((issue, index) => (
                    <TableRow key={index}>
                      <TableCell>
                        <Typography variant="body2" fontWeight="medium">
                          {issue.type}
                        </Typography>
                      </TableCell>
                      <TableCell align="center">
                        <Box display="flex" alignItems="center" justifyContent="center" gap={1}>
                          <Box sx={{ color: getSeverityColor(issue.severity) }}>
                            {getSeverityIcon(issue.severity)}
                          </Box>
                          <Chip
                            label={issue.severity.toUpperCase()}
                            size="small"
                            sx={{ 
                              backgroundColor: getSeverityColor(issue.severity),
                              color: 'white',
                              fontWeight: 'bold',
                            }}
                          />
                        </Box>
                      </TableCell>
                      <TableCell align="right">
                        <Typography variant="body2" fontWeight="bold">
                          {issue.count}
                        </Typography>
                      </TableCell>
                      <TableCell>
                        <Typography variant="body2" color="text.secondary">
                          {issue.impact}
                        </Typography>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </TableContainer>
          </CardContent>
        </Card>
      </Grid>

      {/* Quality Trends */}
      <Grid item xs={12}>
        <Card>
          <CardHeader title="Quality Trends (30 days)" />
          <CardContent>
            <Box height={400}>
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={qualityTrendData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="date" />
                  <YAxis 
                    domain={[0, 100]}
                    label={{ value: 'Quality Score (%)', angle: -90, position: 'insideLeft' }} 
                  />
                  <Tooltip formatter={(value) => [`${Number(value).toFixed(1)}%`, 'Quality Score']} />
                  <Legend />
                  <Line
                    type="monotone"
                    dataKey="completeness"
                    stroke={theme.palette.primary.main}
                    strokeWidth={2}
                    name="Completeness"
                  />
                  <Line
                    type="monotone"
                    dataKey="consistency"
                    stroke={theme.palette.secondary.main}
                    strokeWidth={2}
                    name="Consistency"
                  />
                  <Line
                    type="monotone"
                    dataKey="semantic"
                    stroke={theme.palette.success.main}
                    strokeWidth={2}
                    name="Semantic Quality"
                  />
                </LineChart>
              </ResponsiveContainer>
            </Box>
          </CardContent>
        </Card>
      </Grid>

      {/* Quality Alerts */}
      <Grid item xs={12}>
        <Card>
          <CardHeader title="Quality Alerts & Recommendations" />
          <CardContent>
            <Box>
              {metrics.completeness_scores.overall_completeness_percentage < 0.8 && (
                <Alert severity="warning" sx={{ mb: 1 }}>
                  <Typography variant="subtitle2">Low Data Completeness</Typography>
                  <Typography variant="body2">
                    Only {(metrics.completeness_scores.overall_completeness_percentage * 100).toFixed(1)}% of data is complete. 
                    Consider implementing data validation rules.
                  </Typography>
                </Alert>
              )}
              
              {metrics.consistency_metrics.duplicate_detection_count > 10 && (
                <Alert severity="error" sx={{ mb: 1 }}>
                  <Typography variant="subtitle2">High Duplicate Count</Typography>
                  <Typography variant="body2">
                    {metrics.consistency_metrics.duplicate_detection_count} duplicate nodes detected. 
                    Run deduplication process to improve data quality.
                  </Typography>
                </Alert>
              )}
              
              {metrics.consistency_metrics.schema_compliance_rate < 0.9 && (
                <Alert severity="warning" sx={{ mb: 1 }}>
                  <Typography variant="subtitle2">Schema Compliance Issues</Typography>
                  <Typography variant="body2">
                    {((1 - metrics.consistency_metrics.schema_compliance_rate) * 100).toFixed(1)}% of data doesn't comply with schema. 
                    Review and update data validation rules.
                  </Typography>
                </Alert>
              )}
              
              {metrics.semantic_coherence.embedding_quality_score < 0.7 && (
                <Alert severity="info" sx={{ mb: 1 }}>
                  <Typography variant="subtitle2">Semantic Quality Improvement</Typography>
                  <Typography variant="body2">
                    Embedding quality score is {(metrics.semantic_coherence.embedding_quality_score * 100).toFixed(1)}%. 
                    Consider retraining embeddings with more diverse data.
                  </Typography>
                </Alert>
              )}
              
              {/* Show success message if quality is good */}
              {metrics.completeness_scores.overall_completeness_percentage >= 0.8 && 
               metrics.consistency_metrics.duplicate_detection_count <= 10 && 
               metrics.consistency_metrics.schema_compliance_rate >= 0.9 && 
               metrics.semantic_coherence.embedding_quality_score >= 0.7 && (
                <Alert severity="success">
                  <Typography variant="subtitle2">Excellent Data Quality</Typography>
                  <Typography variant="body2">
                    All quality metrics are within acceptable ranges. Your graph data maintains high standards.
                  </Typography>
                </Alert>
              )}
            </Box>
          </CardContent>
        </Card>
      </Grid>
    </Grid>
  );
};

export default QualityMetrics;