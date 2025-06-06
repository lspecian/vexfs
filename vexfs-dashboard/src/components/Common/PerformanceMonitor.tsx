/**
 * Performance Monitor Component
 * Provides real-time performance monitoring and optimization suggestions
 */

import React, { useState, useEffect } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Chip,
  LinearProgress,
  Alert,
  Collapse,
  IconButton,
  List,
  ListItem,
  ListItemText,
  ListItemIcon,
} from '@mui/material';
import {
  Speed as SpeedIcon,
  Memory as MemoryIcon,
  Warning as WarningIcon,
  CheckCircle as CheckIcon,
  ExpandMore as ExpandIcon,
  ExpandLess as CollapseIcon,
} from '@mui/icons-material';

interface PerformanceMetrics {
  renderTime: number;
  memoryUsage: number;
  componentCount: number;
  slowComponents: string[];
  recommendations: string[];
}

interface PerformanceMonitorProps {
  enabled?: boolean;
  threshold?: number;
  showRecommendations?: boolean;
}

const PerformanceMonitor: React.FC<PerformanceMonitorProps> = ({
  enabled = import.meta.env.DEV,
  threshold = 16,
  showRecommendations = true,
}) => {
  const [metrics, setMetrics] = useState<PerformanceMetrics>({
    renderTime: 0,
    memoryUsage: 0,
    componentCount: 0,
    slowComponents: [],
    recommendations: [],
  });
  const [expanded, setExpanded] = useState(false);
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    if (!enabled) return;

    const updateMetrics = () => {
      // Get memory usage if available
      let memoryUsage = 0;
      if ('memory' in performance) {
        const memory = (performance as any).memory;
        memoryUsage = memory.usedJSHeapSize / 1024 / 1024; // Convert to MB
      }

      // Get performance entries
      const entries = performance.getEntriesByType('measure');
      const renderEntries = entries.filter(entry => 
        entry.name.includes('React') || entry.name.includes('render')
      );

      const avgRenderTime = renderEntries.length > 0
        ? renderEntries.reduce((sum, entry) => sum + entry.duration, 0) / renderEntries.length
        : 0;

      // Count React components (approximate)
      const componentCount = document.querySelectorAll('[data-reactroot], [data-react-component]').length;

      // Identify slow components
      const slowComponents = renderEntries
        .filter(entry => entry.duration > threshold)
        .map(entry => entry.name)
        .slice(0, 5);

      // Generate recommendations
      const recommendations = generateRecommendations(memoryUsage, avgRenderTime, slowComponents);

      setMetrics({
        renderTime: avgRenderTime,
        memoryUsage,
        componentCount,
        slowComponents,
        recommendations,
      });

      // Show monitor if there are performance issues
      setIsVisible(
        avgRenderTime > threshold ||
        memoryUsage > 100 ||
        slowComponents.length > 0
      );
    };

    // Update metrics every 5 seconds
    const interval = setInterval(updateMetrics, 5000);
    updateMetrics(); // Initial update

    return () => clearInterval(interval);
  }, [enabled, threshold]);

  const generateRecommendations = (
    memory: number,
    renderTime: number,
    slowComponents: string[]
  ): string[] => {
    const recommendations: string[] = [];

    if (memory > 100) {
      recommendations.push('High memory usage detected. Consider implementing virtualization for large lists.');
    }

    if (renderTime > threshold) {
      recommendations.push('Slow render times detected. Consider using React.memo for expensive components.');
    }

    if (slowComponents.length > 0) {
      recommendations.push('Multiple slow components found. Consider code splitting and lazy loading.');
    }

    if (memory > 200) {
      recommendations.push('Critical memory usage. Check for memory leaks and unnecessary re-renders.');
    }

    if (renderTime > 50) {
      recommendations.push('Very slow renders. Consider moving heavy computations to Web Workers.');
    }

    return recommendations;
  };

  const getPerformanceStatus = () => {
    if (metrics.renderTime > threshold * 2 || metrics.memoryUsage > 200) {
      return { color: 'error', label: 'Poor' };
    }
    if (metrics.renderTime > threshold || metrics.memoryUsage > 100) {
      return { color: 'warning', label: 'Fair' };
    }
    return { color: 'success', label: 'Good' };
  };

  if (!enabled || !isVisible) {
    return null;
  }

  const status = getPerformanceStatus();

  return (
    <Box
      sx={{
        position: 'fixed',
        bottom: 16,
        right: 16,
        zIndex: 9999,
        maxWidth: 400,
      }}
    >
      <Card elevation={8}>
        <CardContent sx={{ pb: 1 }}>
          <Box display="flex" alignItems="center" justifyContent="space-between">
            <Box display="flex" alignItems="center" gap={1}>
              <SpeedIcon color="primary" />
              <Typography variant="subtitle2" fontWeight="bold">
                Performance Monitor
              </Typography>
              <Chip
                label={status.label}
                color={status.color as any}
                size="small"
                variant="outlined"
              />
            </Box>
            <IconButton
              size="small"
              onClick={() => setExpanded(!expanded)}
            >
              {expanded ? <CollapseIcon /> : <ExpandIcon />}
            </IconButton>
          </Box>

          <Collapse in={expanded}>
            <Box mt={2}>
              {/* Render Time */}
              <Box mb={2}>
                <Box display="flex" alignItems="center" gap={1} mb={1}>
                  <SpeedIcon fontSize="small" />
                  <Typography variant="body2">
                    Avg Render Time: {metrics.renderTime.toFixed(2)}ms
                  </Typography>
                </Box>
                <LinearProgress
                  variant="determinate"
                  value={Math.min((metrics.renderTime / (threshold * 2)) * 100, 100)}
                  color={metrics.renderTime > threshold ? 'warning' : 'success'}
                />
              </Box>

              {/* Memory Usage */}
              <Box mb={2}>
                <Box display="flex" alignItems="center" gap={1} mb={1}>
                  <MemoryIcon fontSize="small" />
                  <Typography variant="body2">
                    Memory Usage: {metrics.memoryUsage.toFixed(1)}MB
                  </Typography>
                </Box>
                <LinearProgress
                  variant="determinate"
                  value={Math.min((metrics.memoryUsage / 200) * 100, 100)}
                  color={metrics.memoryUsage > 100 ? 'warning' : 'success'}
                />
              </Box>

              {/* Component Count */}
              <Typography variant="body2" color="text.secondary" mb={2}>
                Active Components: {metrics.componentCount}
              </Typography>

              {/* Slow Components */}
              {metrics.slowComponents.length > 0 && (
                <Alert severity="warning" sx={{ mb: 2 }}>
                  <Typography variant="body2" fontWeight="bold" gutterBottom>
                    Slow Components:
                  </Typography>
                  {metrics.slowComponents.map((component, index) => (
                    <Typography key={index} variant="caption" display="block">
                      • {component}
                    </Typography>
                  ))}
                </Alert>
              )}

              {/* Recommendations */}
              {showRecommendations && metrics.recommendations.length > 0 && (
                <Box>
                  <Typography variant="body2" fontWeight="bold" gutterBottom>
                    Recommendations:
                  </Typography>
                  <List dense>
                    {metrics.recommendations.map((recommendation, index) => (
                      <ListItem key={index} sx={{ py: 0.5 }}>
                        <ListItemIcon sx={{ minWidth: 32 }}>
                          {recommendation.includes('Critical') ? (
                            <WarningIcon color="error" fontSize="small" />
                          ) : (
                            <CheckIcon color="primary" fontSize="small" />
                          )}
                        </ListItemIcon>
                        <ListItemText
                          primary={recommendation}
                          primaryTypographyProps={{ variant: 'caption' }}
                        />
                      </ListItem>
                    ))}
                  </List>
                </Box>
              )}
            </Box>
          </Collapse>
        </CardContent>
      </Card>
    </Box>
  );
};

export default PerformanceMonitor;