import React, { useState, useCallback } from 'react';
import {
  Box,
  Grid,
  Paper,
  Typography,
  Card,
  CardContent,
  CardHeader,
  IconButton,
  Button,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Checkbox,
  FormControlLabel,
  FormGroup,
  Chip,
  useTheme,
  Divider,
} from '@mui/material';
import {
  Add as AddIcon,
  Remove as RemoveIcon,
  Settings as SettingsIcon,
  DragIndicator as DragIcon,
  Visibility as VisibilityIcon,
  VisibilityOff as VisibilityOffIcon,
} from '@mui/icons-material';

import { StructureMetrics } from './StructureMetrics';
import { PerformanceCharts } from './PerformanceCharts';
import { GrowthAnalytics } from './GrowthAnalytics';
import { QualityMetrics } from './QualityMetrics';

import type {
  PerformanceMetrics,
  GrowthData,
  DataQualityMetrics
} from './GraphAnalyticsDashboard';
import type { GraphStatistics } from '../../types/graph';

export interface DashboardWidget {
  id: string;
  type: 'structure' | 'performance' | 'growth' | 'quality' | 'custom';
  title: string;
  visible: boolean;
  size: 'small' | 'medium' | 'large';
  position: { x: number; y: number };
}

export interface CustomizableDashboardProps {
  statistics?: GraphStatistics | null;
  performance?: PerformanceMetrics | null;
  growth?: GrowthData | null;
  quality?: DataQualityMetrics | null;
  isLoading?: boolean;
}

const defaultWidgets: DashboardWidget[] = [
  {
    id: 'structure-overview',
    type: 'structure',
    title: 'Graph Structure Overview',
    visible: true,
    size: 'large',
    position: { x: 0, y: 0 },
  },
  {
    id: 'performance-summary',
    type: 'performance',
    title: 'Performance Summary',
    visible: true,
    size: 'medium',
    position: { x: 1, y: 0 },
  },
  {
    id: 'growth-trends',
    type: 'growth',
    title: 'Growth Trends',
    visible: true,
    size: 'medium',
    position: { x: 0, y: 1 },
  },
  {
    id: 'quality-metrics',
    type: 'quality',
    title: 'Quality Metrics',
    visible: true,
    size: 'medium',
    position: { x: 1, y: 1 },
  },
];

export const CustomizableDashboard: React.FC<CustomizableDashboardProps> = ({
  statistics,
  performance,
  growth,
  quality,
  isLoading = false,
}) => {
  const theme = useTheme();
  const [widgets, setWidgets] = useState<DashboardWidget[]>(defaultWidgets);
  const [configDialogOpen, setConfigDialogOpen] = useState(false);
  const [selectedWidget, setSelectedWidget] = useState<string | null>(null);

  // Widget configuration handlers
  const handleToggleWidget = useCallback((widgetId: string) => {
    setWidgets(prev => prev.map(widget => 
      widget.id === widgetId 
        ? { ...widget, visible: !widget.visible }
        : widget
    ));
  }, []);

  const handleWidgetSizeChange = useCallback((widgetId: string, size: DashboardWidget['size']) => {
    setWidgets(prev => prev.map(widget => 
      widget.id === widgetId 
        ? { ...widget, size }
        : widget
    ));
  }, []);

  const handleAddWidget = useCallback((type: DashboardWidget['type']) => {
    const newWidget: DashboardWidget = {
      id: `${type}-${Date.now()}`,
      type,
      title: `${type.charAt(0).toUpperCase() + type.slice(1)} Widget`,
      visible: true,
      size: 'medium',
      position: { x: 0, y: widgets.length },
    };
    setWidgets(prev => [...prev, newWidget]);
  }, [widgets.length]);

  const handleRemoveWidget = useCallback((widgetId: string) => {
    setWidgets(prev => prev.filter(widget => widget.id !== widgetId));
  }, []);

  // Get grid size based on widget size
  const getGridSize = (size: DashboardWidget['size']) => {
    switch (size) {
      case 'small': return { xs: 12, md: 6, lg: 4 };
      case 'medium': return { xs: 12, md: 6, lg: 6 };
      case 'large': return { xs: 12, md: 12, lg: 12 };
      default: return { xs: 12, md: 6, lg: 6 };
    }
  };

  // Render widget content based on type
  const renderWidgetContent = (widget: DashboardWidget) => {
    switch (widget.type) {
      case 'structure':
        return statistics ? (
          <StructureMetrics statistics={statistics} isLoading={isLoading} />
        ) : (
          <Typography color="text.secondary">No structure data available</Typography>
        );
      
      case 'performance':
        return performance ? (
          <PerformanceCharts metrics={performance} isLoading={isLoading} />
        ) : (
          <Typography color="text.secondary">No performance data available</Typography>
        );
      
      case 'growth':
        return growth ? (
          <GrowthAnalytics data={growth} isLoading={isLoading} />
        ) : (
          <Typography color="text.secondary">No growth data available</Typography>
        );
      
      case 'quality':
        return quality ? (
          <QualityMetrics metrics={quality} isLoading={isLoading} />
        ) : (
          <Typography color="text.secondary">No quality data available</Typography>
        );
      
      default:
        return (
          <Typography color="text.secondary">
            Custom widget content for {widget.type}
          </Typography>
        );
    }
  };

  // Configuration dialog
  const ConfigurationDialog = () => (
    <Dialog open={configDialogOpen} onClose={() => setConfigDialogOpen(false)} maxWidth="md" fullWidth>
      <DialogTitle>Dashboard Configuration</DialogTitle>
      <DialogContent>
        <Box mb={3}>
          <Typography variant="h6" gutterBottom>
            Widget Visibility
          </Typography>
          <FormGroup>
            {widgets.map(widget => (
              <FormControlLabel
                key={widget.id}
                control={
                  <Checkbox
                    checked={widget.visible}
                    onChange={() => handleToggleWidget(widget.id)}
                  />
                }
                label={
                  <Box display="flex" alignItems="center" gap={1}>
                    <Typography>{widget.title}</Typography>
                    <Chip label={widget.type} size="small" />
                    <Chip label={widget.size} size="small" variant="outlined" />
                  </Box>
                }
              />
            ))}
          </FormGroup>
        </Box>

        <Divider sx={{ my: 2 }} />

        <Box mb={3}>
          <Typography variant="h6" gutterBottom>
            Add New Widget
          </Typography>
          <Box display="flex" gap={1} flexWrap="wrap">
            <Button
              variant="outlined"
              startIcon={<AddIcon />}
              onClick={() => handleAddWidget('structure')}
            >
              Structure
            </Button>
            <Button
              variant="outlined"
              startIcon={<AddIcon />}
              onClick={() => handleAddWidget('performance')}
            >
              Performance
            </Button>
            <Button
              variant="outlined"
              startIcon={<AddIcon />}
              onClick={() => handleAddWidget('growth')}
            >
              Growth
            </Button>
            <Button
              variant="outlined"
              startIcon={<AddIcon />}
              onClick={() => handleAddWidget('quality')}
            >
              Quality
            </Button>
          </Box>
        </Box>

        <Divider sx={{ my: 2 }} />

        <Box>
          <Typography variant="h6" gutterBottom>
            Widget Settings
          </Typography>
          {selectedWidget && (
            <Box>
              <FormControl fullWidth sx={{ mb: 2 }}>
                <InputLabel>Widget Size</InputLabel>
                <Select
                  value={widgets.find(w => w.id === selectedWidget)?.size || 'medium'}
                  onChange={(e) => handleWidgetSizeChange(selectedWidget, e.target.value as DashboardWidget['size'])}
                >
                  <MenuItem value="small">Small</MenuItem>
                  <MenuItem value="medium">Medium</MenuItem>
                  <MenuItem value="large">Large</MenuItem>
                </Select>
              </FormControl>
            </Box>
          )}
        </Box>
      </DialogContent>
      <DialogActions>
        <Button onClick={() => setConfigDialogOpen(false)}>Close</Button>
        <Button variant="contained" onClick={() => setConfigDialogOpen(false)}>
          Save Configuration
        </Button>
      </DialogActions>
    </Dialog>
  );

  return (
    <Box>
      {/* Dashboard Header */}
      <Paper sx={{ p: 2, mb: 2 }}>
        <Box display="flex" justifyContent="space-between" alignItems="center">
          <Typography variant="h6">
            Custom Dashboard
          </Typography>
          <Box display="flex" gap={1}>
            <Button
              variant="outlined"
              startIcon={<SettingsIcon />}
              onClick={() => setConfigDialogOpen(true)}
            >
              Configure
            </Button>
          </Box>
        </Box>
      </Paper>

      {/* Widget Grid */}
      <Grid container spacing={3}>
        {widgets
          .filter(widget => widget.visible)
          .map(widget => {
            const gridSize = getGridSize(widget.size);
            return (
              <Grid item {...gridSize} key={widget.id}>
                <Card sx={{ height: '100%' }}>
                  <CardHeader
                    title={widget.title}
                    action={
                      <Box display="flex" alignItems="center">
                        <IconButton
                          size="small"
                          onClick={() => setSelectedWidget(widget.id)}
                          color={selectedWidget === widget.id ? 'primary' : 'default'}
                        >
                          <SettingsIcon fontSize="small" />
                        </IconButton>
                        <IconButton
                          size="small"
                          onClick={() => handleToggleWidget(widget.id)}
                        >
                          {widget.visible ? <VisibilityIcon fontSize="small" /> : <VisibilityOffIcon fontSize="small" />}
                        </IconButton>
                        <IconButton
                          size="small"
                          onClick={() => handleRemoveWidget(widget.id)}
                          color="error"
                        >
                          <RemoveIcon fontSize="small" />
                        </IconButton>
                        <IconButton size="small" sx={{ cursor: 'grab' }}>
                          <DragIcon fontSize="small" />
                        </IconButton>
                      </Box>
                    }
                    titleTypographyProps={{ variant: 'subtitle1' }}
                  />
                  <CardContent>
                    {renderWidgetContent(widget)}
                  </CardContent>
                </Card>
              </Grid>
            );
          })}
      </Grid>

      {/* Empty State */}
      {widgets.filter(w => w.visible).length === 0 && (
        <Paper sx={{ p: 4, textAlign: 'center' }}>
          <Typography variant="h6" color="text.secondary" gutterBottom>
            No widgets configured
          </Typography>
          <Typography variant="body2" color="text.secondary" mb={2}>
            Add widgets to customize your dashboard view
          </Typography>
          <Button
            variant="contained"
            startIcon={<AddIcon />}
            onClick={() => setConfigDialogOpen(true)}
          >
            Configure Dashboard
          </Button>
        </Paper>
      )}

      {/* Configuration Dialog */}
      <ConfigurationDialog />
    </Box>
  );
};

export default CustomizableDashboard;