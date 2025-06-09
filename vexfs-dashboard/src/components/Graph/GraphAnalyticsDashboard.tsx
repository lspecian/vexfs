import React, { useState, useEffect, useCallback } from 'react';
import {
  Box,
  Grid,
  Paper,
  Typography,
  Card,
  CardContent,
  CardHeader,
  IconButton,
  Tooltip,
  Switch,
  FormControlLabel,
  Button,
  ButtonGroup,
  Chip,
  Alert,
  CircularProgress,
  useTheme,
  Divider,
} from '@mui/material';
import {
  Refresh as RefreshIcon,
  Settings as SettingsIcon,
  Download as DownloadIcon,
  Dashboard as DashboardIcon,
  Timeline as TimelineIcon,
  Assessment as AssessmentIcon,
  Speed as SpeedIcon,
  TrendingUp as TrendingUpIcon,
  VerifiedUser as QualityIcon,
} from '@mui/icons-material';

import { StructureMetrics } from './StructureMetrics';
import { PerformanceCharts } from './PerformanceCharts';
import { GrowthAnalytics } from './GrowthAnalytics';
import { QualityMetrics } from './QualityMetrics';
import { CustomizableDashboard } from './CustomizableDashboard';
import { AnalyticsExport } from './AnalyticsExport';

import type { 
  GraphStatistics, 
  GraphAnalytics,
  NodeResponse,
  EdgeResponse 
} from '../../types/graph';

// Analytics API types
export interface PerformanceMetrics {
  query_performance: {
    average_query_time_ms: number;
    queries_per_second: number;
    success_rate: number;
    timeout_rate: number;
  };
  index_efficiency: {
    hit_rate: number;
    update_time_ms: number;
    storage_overhead_mb: number;
  };
  memory_usage: {
    ram_utilization_mb: number;
    cache_hit_rate: number;
    gc_frequency_per_hour: number;
  };
  storage_metrics: {
    disk_usage_mb: number;
    compression_ratio: number;
    io_operations_per_second: number;
  };
}

export interface GrowthData {
  historical_trends: {
    timestamp: string;
    node_count: number;
    edge_count: number;
  }[];
  activity_patterns: {
    creation_rate_per_hour: number;
    modification_rate_per_hour: number;
    deletion_rate_per_hour: number;
  };
  usage_analytics: {
    most_accessed_nodes: { node_id: string; access_count: number }[];
    popular_query_patterns: { pattern: string; frequency: number }[];
  };
}

export interface DataQualityMetrics {
  completeness_scores: {
    nodes_with_complete_metadata: number;
    edges_with_complete_metadata: number;
    overall_completeness_percentage: number;
  };
  consistency_metrics: {
    schema_compliance_rate: number;
    referential_integrity_score: number;
    duplicate_detection_count: number;
  };
  semantic_coherence: {
    embedding_quality_score: number;
    relationship_accuracy_score: number;
    semantic_consistency_score: number;
  };
}

export interface GraphAnalyticsDashboardProps {
  nodes?: NodeResponse[];
  edges?: EdgeResponse[];
  autoRefresh?: boolean;
  refreshInterval?: number;
  onExport?: (data: any, format: string) => void;
  className?: string;
}

export const GraphAnalyticsDashboard: React.FC<GraphAnalyticsDashboardProps> = ({
  nodes = [],
  edges = [],
  autoRefresh = true,
  refreshInterval = 30000, // 30 seconds
  onExport,
  className,
}) => {
  const theme = useTheme();
  
  // State management
  const [activeTab, setActiveTab] = useState<'overview' | 'structure' | 'performance' | 'growth' | 'quality' | 'custom'>('overview');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date>(new Date());
  
  // Analytics data state
  const [graphStats, setGraphStats] = useState<GraphStatistics | null>(null);
  const [graphAnalytics, setGraphAnalytics] = useState<GraphAnalytics | null>(null);
  const [performanceMetrics, setPerformanceMetrics] = useState<PerformanceMetrics | null>(null);
  const [growthData, setGrowthData] = useState<GrowthData | null>(null);
  const [qualityMetrics, setQualityMetrics] = useState<DataQualityMetrics | null>(null);
  
  // Settings state
  const [realTimeUpdates, setRealTimeUpdates] = useState(autoRefresh);
  const [showExportDialog, setShowExportDialog] = useState(false);

  // Mock API functions (replace with actual API calls)
  const fetchGraphStatistics = useCallback(async (): Promise<GraphStatistics> => {
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 500));
    
    const nodeTypeCount = nodes.reduce((acc, node) => {
      acc[node.node_type] = (acc[node.node_type] || 0) + 1;
      return acc;
    }, {} as Record<string, number>);
    
    const edgeTypeCount = edges.reduce((acc, edge) => {
      acc[edge.edge_type] = (acc[edge.edge_type] || 0) + 1;
      return acc;
    }, {} as Record<string, number>);
    
    return {
      node_count: nodes.length,
      edge_count: edges.length,
      node_types: nodeTypeCount,
      edge_types: edgeTypeCount,
      average_degree: nodes.length > 0 ? (edges.length * 2) / nodes.length : 0,
      density: nodes.length > 1 ? edges.length / (nodes.length * (nodes.length - 1) / 2) : 0,
      connected_components: Math.max(1, Math.floor(nodes.length / 10)),
      largest_component_size: Math.floor(nodes.length * 0.8),
      clustering_coefficient: Math.random() * 0.5 + 0.3,
      diameter: Math.floor(Math.log(nodes.length + 1) * 3),
    };
  }, [nodes, edges]);

  const fetchPerformanceMetrics = useCallback(async (): Promise<PerformanceMetrics> => {
    await new Promise(resolve => setTimeout(resolve, 300));
    return {
      query_performance: {
        average_query_time_ms: Math.random() * 100 + 50,
        queries_per_second: Math.random() * 1000 + 500,
        success_rate: Math.random() * 0.1 + 0.9,
        timeout_rate: Math.random() * 0.05,
      },
      index_efficiency: {
        hit_rate: Math.random() * 0.2 + 0.8,
        update_time_ms: Math.random() * 50 + 10,
        storage_overhead_mb: Math.random() * 100 + 50,
      },
      memory_usage: {
        ram_utilization_mb: Math.random() * 1000 + 500,
        cache_hit_rate: Math.random() * 0.3 + 0.7,
        gc_frequency_per_hour: Math.random() * 10 + 5,
      },
      storage_metrics: {
        disk_usage_mb: Math.random() * 5000 + 1000,
        compression_ratio: Math.random() * 2 + 2,
        io_operations_per_second: Math.random() * 10000 + 5000,
      },
    };
  }, []);

  const fetchGrowthData = useCallback(async (): Promise<GrowthData> => {
    await new Promise(resolve => setTimeout(resolve, 400));
    
    const now = new Date();
    const historical_trends = Array.from({ length: 30 }, (_, i) => {
      const date = new Date(now.getTime() - (29 - i) * 24 * 60 * 60 * 1000);
      return {
        timestamp: date.toISOString(),
        node_count: Math.floor(nodes.length * (0.5 + i * 0.5 / 29)),
        edge_count: Math.floor(edges.length * (0.5 + i * 0.5 / 29)),
      };
    });
    
    return {
      historical_trends,
      activity_patterns: {
        creation_rate_per_hour: Math.random() * 100 + 50,
        modification_rate_per_hour: Math.random() * 200 + 100,
        deletion_rate_per_hour: Math.random() * 20 + 5,
      },
      usage_analytics: {
        most_accessed_nodes: nodes.slice(0, 10).map((node, i) => ({
          node_id: node.id,
          access_count: Math.floor(Math.random() * 1000) + 100 - i * 50,
        })),
        popular_query_patterns: [
          { pattern: 'semantic_search', frequency: Math.floor(Math.random() * 500) + 200 },
          { pattern: 'traversal_bfs', frequency: Math.floor(Math.random() * 300) + 100 },
          { pattern: 'property_filter', frequency: Math.floor(Math.random() * 400) + 150 },
        ],
      },
    };
  }, [nodes, edges]);

  const fetchQualityMetrics = useCallback(async (): Promise<DataQualityMetrics> => {
    await new Promise(resolve => setTimeout(resolve, 350));
    return {
      completeness_scores: {
        nodes_with_complete_metadata: Math.floor(nodes.length * (Math.random() * 0.3 + 0.7)),
        edges_with_complete_metadata: Math.floor(edges.length * (Math.random() * 0.3 + 0.7)),
        overall_completeness_percentage: Math.random() * 0.3 + 0.7,
      },
      consistency_metrics: {
        schema_compliance_rate: Math.random() * 0.2 + 0.8,
        referential_integrity_score: Math.random() * 0.1 + 0.9,
        duplicate_detection_count: Math.floor(Math.random() * 20),
      },
      semantic_coherence: {
        embedding_quality_score: Math.random() * 0.3 + 0.7,
        relationship_accuracy_score: Math.random() * 0.2 + 0.8,
        semantic_consistency_score: Math.random() * 0.25 + 0.75,
      },
    };
  }, [nodes, edges]);

  // Data fetching
  const refreshData = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    
    try {
      const [stats, performance, growth, quality] = await Promise.all([
        fetchGraphStatistics(),
        fetchPerformanceMetrics(),
        fetchGrowthData(),
        fetchQualityMetrics(),
      ]);
      
      setGraphStats(stats);
      setPerformanceMetrics(performance);
      setGrowthData(growth);
      setQualityMetrics(quality);
      setLastUpdated(new Date());
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch analytics data');
    } finally {
      setIsLoading(false);
    }
  }, [fetchGraphStatistics, fetchPerformanceMetrics, fetchGrowthData, fetchQualityMetrics]);

  // Auto-refresh effect
  useEffect(() => {
    refreshData();
  }, [refreshData]);

  useEffect(() => {
    if (!realTimeUpdates) return;
    
    const interval = setInterval(refreshData, refreshInterval);
    return () => clearInterval(interval);
  }, [realTimeUpdates, refreshInterval, refreshData]);

  // Event handlers
  const handleTabChange = (tab: typeof activeTab) => {
    setActiveTab(tab);
  };

  const handleExport = () => {
    setShowExportDialog(true);
  };

  const handleExportData = (format: string) => {
    const data = {
      statistics: graphStats,
      analytics: graphAnalytics,
      performance: performanceMetrics,
      growth: growthData,
      quality: qualityMetrics,
      timestamp: lastUpdated.toISOString(),
    };
    
    onExport?.(data, format);
    setShowExportDialog(false);
  };

  // Overview metrics cards
  const overviewCards = [
    {
      title: 'Graph Structure',
      value: graphStats ? `${graphStats.node_count} nodes, ${graphStats.edge_count} edges` : 'Loading...',
      icon: <DashboardIcon />,
      color: theme.palette.primary.main,
    },
    {
      title: 'Performance',
      value: performanceMetrics ? `${performanceMetrics.query_performance.queries_per_second.toFixed(0)} QPS` : 'Loading...',
      icon: <SpeedIcon />,
      color: theme.palette.success.main,
    },
    {
      title: 'Growth Rate',
      value: growthData ? `${growthData.activity_patterns.creation_rate_per_hour.toFixed(0)}/hr` : 'Loading...',
      icon: <TrendingUpIcon />,
      color: theme.palette.info.main,
    },
    {
      title: 'Data Quality',
      value: qualityMetrics ? `${(qualityMetrics.completeness_scores.overall_completeness_percentage * 100).toFixed(1)}%` : 'Loading...',
      icon: <QualityIcon />,
      color: theme.palette.warning.main,
    },
  ];

  if (error) {
    return (
      <Paper sx={{ p: 3 }}>
        <Alert severity="error">
          <Typography variant="h6">Analytics Error</Typography>
          <Typography variant="body2">{error}</Typography>
          <Button onClick={refreshData} sx={{ mt: 1 }}>
            Retry
          </Button>
        </Alert>
      </Paper>
    );
  }

  return (
    <Box className={className}>
      {/* Header */}
      <Paper sx={{ p: 2, mb: 2 }}>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Box>
            <Typography variant="h5" gutterBottom>
              Graph Analytics Dashboard
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Last updated: {lastUpdated.toLocaleTimeString()}
            </Typography>
          </Box>
          
          <Box display="flex" alignItems="center" gap={2}>
            <FormControlLabel
              control={
                <Switch
                  checked={realTimeUpdates}
                  onChange={(e) => setRealTimeUpdates(e.target.checked)}
                />
              }
              label="Real-time updates"
            />
            
            <ButtonGroup variant="outlined" size="small">
              <Tooltip title="Refresh Data">
                <IconButton onClick={refreshData} disabled={isLoading}>
                  <RefreshIcon />
                </IconButton>
              </Tooltip>
              <Tooltip title="Export Data">
                <IconButton onClick={handleExport}>
                  <DownloadIcon />
                </IconButton>
              </Tooltip>
              <Tooltip title="Settings">
                <IconButton>
                  <SettingsIcon />
                </IconButton>
              </Tooltip>
            </ButtonGroup>
          </Box>
        </Box>
      </Paper>

      {/* Navigation Tabs */}
      <Paper sx={{ p: 1, mb: 2 }}>
        <ButtonGroup variant="outlined" fullWidth>
          <Button
            variant={activeTab === 'overview' ? 'contained' : 'outlined'}
            onClick={() => handleTabChange('overview')}
            startIcon={<AssessmentIcon />}
          >
            Overview
          </Button>
          <Button
            variant={activeTab === 'structure' ? 'contained' : 'outlined'}
            onClick={() => handleTabChange('structure')}
            startIcon={<DashboardIcon />}
          >
            Structure
          </Button>
          <Button
            variant={activeTab === 'performance' ? 'contained' : 'outlined'}
            onClick={() => handleTabChange('performance')}
            startIcon={<SpeedIcon />}
          >
            Performance
          </Button>
          <Button
            variant={activeTab === 'growth' ? 'contained' : 'outlined'}
            onClick={() => handleTabChange('growth')}
            startIcon={<TimelineIcon />}
          >
            Growth
          </Button>
          <Button
            variant={activeTab === 'quality' ? 'contained' : 'outlined'}
            onClick={() => handleTabChange('quality')}
            startIcon={<QualityIcon />}
          >
            Quality
          </Button>
          <Button
            variant={activeTab === 'custom' ? 'contained' : 'outlined'}
            onClick={() => handleTabChange('custom')}
            startIcon={<SettingsIcon />}
          >
            Custom
          </Button>
        </ButtonGroup>
      </Paper>

      {/* Loading Indicator */}
      {isLoading && (
        <Box display="flex" justifyContent="center" py={2}>
          <CircularProgress />
        </Box>
      )}

      {/* Content */}
      {activeTab === 'overview' && (
        <Grid container spacing={3}>
          {/* Overview Cards */}
          {overviewCards.map((card, index) => (
            <Grid item xs={12} sm={6} md={3} key={index}>
              <Card>
                <CardContent>
                  <Box display="flex" alignItems="center" gap={2}>
                    <Box sx={{ color: card.color }}>
                      {card.icon}
                    </Box>
                    <Box>
                      <Typography variant="h6" component="div">
                        {card.value}
                      </Typography>
                      <Typography variant="body2" color="text.secondary">
                        {card.title}
                      </Typography>
                    </Box>
                  </Box>
                </CardContent>
              </Card>
            </Grid>
          ))}
          
          {/* Quick Stats */}
          <Grid item xs={12}>
            <Card>
              <CardHeader title="Quick Statistics" />
              <CardContent>
                <Grid container spacing={2}>
                  <Grid item xs={12} md={6}>
                    <Typography variant="subtitle2" gutterBottom>
                      Graph Topology
                    </Typography>
                    <Box display="flex" flexWrap="wrap" gap={1}>
                      <Chip label={`Density: ${graphStats?.density.toFixed(3) || 'N/A'}`} size="small" />
                      <Chip label={`Avg Degree: ${graphStats?.average_degree.toFixed(1) || 'N/A'}`} size="small" />
                      <Chip label={`Components: ${graphStats?.connected_components || 'N/A'}`} size="small" />
                      <Chip label={`Diameter: ${graphStats?.diameter || 'N/A'}`} size="small" />
                    </Box>
                  </Grid>
                  <Grid item xs={12} md={6}>
                    <Typography variant="subtitle2" gutterBottom>
                      Performance Indicators
                    </Typography>
                    <Box display="flex" flexWrap="wrap" gap={1}>
                      <Chip 
                        label={`Query Time: ${performanceMetrics?.query_performance.average_query_time_ms.toFixed(1) || 'N/A'}ms`} 
                        size="small" 
                        color="success"
                      />
                      <Chip 
                        label={`Success Rate: ${((performanceMetrics?.query_performance.success_rate || 0) * 100).toFixed(1)}%`} 
                        size="small" 
                        color="info"
                      />
                      <Chip 
                        label={`Cache Hit: ${((performanceMetrics?.memory_usage.cache_hit_rate || 0) * 100).toFixed(1)}%`} 
                        size="small" 
                        color="primary"
                      />
                    </Box>
                  </Grid>
                </Grid>
              </CardContent>
            </Card>
          </Grid>
        </Grid>
      )}

      {activeTab === 'structure' && graphStats && (
        <StructureMetrics 
          statistics={graphStats}
          analytics={graphAnalytics}
          isLoading={isLoading}
        />
      )}

      {activeTab === 'performance' && performanceMetrics && (
        <PerformanceCharts 
          metrics={performanceMetrics}
          isLoading={isLoading}
        />
      )}

      {activeTab === 'growth' && growthData && (
        <GrowthAnalytics 
          data={growthData}
          isLoading={isLoading}
        />
      )}

      {activeTab === 'quality' && qualityMetrics && (
        <QualityMetrics 
          metrics={qualityMetrics}
          isLoading={isLoading}
        />
      )}

      {activeTab === 'custom' && (
        <CustomizableDashboard 
          statistics={graphStats}
          performance={performanceMetrics}
          growth={growthData}
          quality={qualityMetrics}
          isLoading={isLoading}
        />
      )}

      {/* Export Dialog */}
      {showExportDialog && (
        <AnalyticsExport
          open={showExportDialog}
          onClose={() => setShowExportDialog(false)}
          onExport={handleExportData}
          data={{
            statistics: graphStats,
            performance: performanceMetrics,
            growth: growthData,
            quality: qualityMetrics,
          }}
        />
      )}
    </Box>
  );
};

export default GraphAnalyticsDashboard;