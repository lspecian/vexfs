import React, { useState, useMemo } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Switch,
  FormControlLabel,
  Button,
  Slider,
} from '@mui/material';
import {
  BarChart,
  Bar,
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
  Cell,
} from 'recharts';
import { Download as DownloadIcon } from '@mui/icons-material';
import type { VectorVisualizationConfig } from '../../types';

interface VectorVisualizationProps {
  vector: number[];
  vectorId?: string | number;
  title?: string;
  config?: Partial<VectorVisualizationConfig>;
  onConfigChange?: (config: VectorVisualizationConfig) => void;
}

const VectorVisualization: React.FC<VectorVisualizationProps> = ({
  vector,
  vectorId,
  title = 'Vector Visualization',
  config = {},
  onConfigChange,
}) => {
  const [localConfig, setLocalConfig] = useState<VectorVisualizationConfig>({
    type: 'bar',
    maxDimensions: 50,
    showLabels: true,
    colorScheme: 'blue',
    ...config,
  });

  const updateConfig = (updates: Partial<VectorVisualizationConfig>) => {
    const newConfig = { ...localConfig, ...updates };
    setLocalConfig(newConfig);
    onConfigChange?.(newConfig);
  };

  const chartData = useMemo(() => {
    const maxDims = Math.min(localConfig.maxDimensions || 50, vector.length);
    return vector.slice(0, maxDims).map((value, index) => ({
      dimension: index,
      value: value,
      label: `D${index}`,
    }));
  }, [vector, localConfig.maxDimensions]);

  const getBarColor = (value: number, index: number) => {
    const intensity = Math.abs(value);
    const maxIntensity = Math.max(...vector.map(Math.abs));
    const normalizedIntensity = intensity / maxIntensity;

    switch (localConfig.colorScheme) {
      case 'blue':
        return `hsl(210, 100%, ${100 - normalizedIntensity * 50}%)`;
      case 'red':
        return `hsl(0, 100%, ${100 - normalizedIntensity * 50}%)`;
      case 'green':
        return `hsl(120, 100%, ${100 - normalizedIntensity * 50}%)`;
      case 'gradient':
        return `hsl(${(index / chartData.length) * 360}, 70%, 50%)`;
      default:
        return value >= 0 ? '#8884d8' : '#ff7c7c';
    }
  };

  const exportChart = () => {
    const dataStr = JSON.stringify(chartData, null, 2);
    const dataBlob = new Blob([dataStr], { type: 'application/json' });
    const url = URL.createObjectURL(dataBlob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `vector_${vectorId || 'data'}_visualization.json`;
    link.click();
    URL.revokeObjectURL(url);
  };

  const renderChart = () => {
    const commonProps = {
      data: chartData,
      margin: { top: 20, right: 30, left: 20, bottom: 5 },
    };

    switch (localConfig.type) {
      case 'line':
        return (
          <LineChart {...commonProps}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis
              dataKey="dimension"
              tick={{ fontSize: 12 }}
              interval="preserveStartEnd"
            />
            <YAxis tick={{ fontSize: 12 }} />
            <RechartsTooltip
              formatter={(value: number, _name: string) => [
                value.toFixed(4),
                'Value',
              ]}
              labelFormatter={(label: number) => `Dimension ${label}`}
            />
            <Line
              type="monotone"
              dataKey="value"
              stroke="#8884d8"
              strokeWidth={2}
              dot={{ r: 2 }}
              activeDot={{ r: 4 }}
            />
          </LineChart>
        );

      case 'bar':
      default:
        return (
          <BarChart {...commonProps}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis
              dataKey="dimension"
              tick={{ fontSize: 12 }}
              interval="preserveStartEnd"
            />
            <YAxis tick={{ fontSize: 12 }} />
            <RechartsTooltip
              formatter={(value: number, _name: string) => [
                value.toFixed(4),
                'Value',
              ]}
              labelFormatter={(label: number) => `Dimension ${label}`}
            />
            <Bar dataKey="value">
              {chartData.map((entry, index) => (
                <Cell
                  key={`cell-${index}`}
                  fill={getBarColor(entry.value, index)}
                />
              ))}
            </Bar>
          </BarChart>
        );
    }
  };

  const vectorStats = useMemo(() => {
    const values = vector;
    const magnitude = Math.sqrt(
      values.reduce((sum, val) => sum + val * val, 0)
    );
    const mean = values.reduce((sum, val) => sum + val, 0) / values.length;
    const variance =
      values.reduce((sum, val) => sum + Math.pow(val - mean, 2), 0) /
      values.length;
    const stdDev = Math.sqrt(variance);
    const min = Math.min(...values);
    const max = Math.max(...values);

    return { magnitude, mean, stdDev, min, max };
  }, [vector]);

  return (
    <Card>
      <CardContent>
        <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 2 }}>
          <Typography variant="h6" sx={{ fontWeight: 600 }}>
            {title}
          </Typography>
          <Button
            size="small"
            startIcon={<DownloadIcon />}
            onClick={exportChart}
            variant="outlined"
          >
            Export
          </Button>
        </Box>

        {/* Configuration Controls */}
        <Box
          sx={{
            display: 'flex',
            flexWrap: 'wrap',
            gap: 2,
            mb: 3,
            alignItems: 'center',
          }}
        >
          <FormControl size="small" sx={{ minWidth: 120 }}>
            <InputLabel>Chart Type</InputLabel>
            <Select
              value={localConfig.type}
              label="Chart Type"
              onChange={e =>
                updateConfig({
                  type: e.target.value as 'bar' | 'line' | 'heatmap',
                })
              }
            >
              <MenuItem value="bar">Bar Chart</MenuItem>
              <MenuItem value="line">Line Chart</MenuItem>
            </Select>
          </FormControl>

          <FormControl size="small" sx={{ minWidth: 120 }}>
            <InputLabel>Color Scheme</InputLabel>
            <Select
              value={localConfig.colorScheme}
              label="Color Scheme"
              onChange={e => updateConfig({ colorScheme: e.target.value })}
            >
              <MenuItem value="blue">Blue</MenuItem>
              <MenuItem value="red">Red</MenuItem>
              <MenuItem value="green">Green</MenuItem>
              <MenuItem value="gradient">Gradient</MenuItem>
              <MenuItem value="signed">Signed (Pos/Neg)</MenuItem>
            </Select>
          </FormControl>

          <Box sx={{ minWidth: 200 }}>
            <Typography variant="caption" color="text.secondary">
              Max Dimensions: {localConfig.maxDimensions}
            </Typography>
            <Slider
              value={localConfig.maxDimensions || 50}
              onChange={(_, value) =>
                updateConfig({ maxDimensions: value as number })
              }
              min={10}
              max={Math.min(vector.length, 200)}
              step={10}
              size="small"
            />
          </Box>

          <FormControlLabel
            control={
              <Switch
                checked={localConfig.showLabels}
                onChange={e => updateConfig({ showLabels: e.target.checked })}
                size="small"
              />
            }
            label="Show Labels"
          />
        </Box>

        {/* Vector Statistics */}
        <Box
          sx={{
            display: 'flex',
            flexWrap: 'wrap',
            gap: 3,
            mb: 3,
            justifyContent: 'space-around',
          }}
        >
          <Box sx={{ textAlign: 'center' }}>
            <Typography variant="h6" color="primary">
              {vectorStats.magnitude.toFixed(3)}
            </Typography>
            <Typography variant="caption" color="text.secondary">
              Magnitude
            </Typography>
          </Box>
          <Box sx={{ textAlign: 'center' }}>
            <Typography variant="h6" color="secondary">
              {vectorStats.mean.toFixed(3)}
            </Typography>
            <Typography variant="caption" color="text.secondary">
              Mean
            </Typography>
          </Box>
          <Box sx={{ textAlign: 'center' }}>
            <Typography variant="h6">
              {vectorStats.stdDev.toFixed(3)}
            </Typography>
            <Typography variant="caption" color="text.secondary">
              Std Dev
            </Typography>
          </Box>
          <Box sx={{ textAlign: 'center' }}>
            <Typography variant="h6" color="error">
              {vectorStats.min.toFixed(3)}
            </Typography>
            <Typography variant="caption" color="text.secondary">
              Min
            </Typography>
          </Box>
          <Box sx={{ textAlign: 'center' }}>
            <Typography variant="h6" color="success">
              {vectorStats.max.toFixed(3)}
            </Typography>
            <Typography variant="caption" color="text.secondary">
              Max
            </Typography>
          </Box>
          <Box sx={{ textAlign: 'center' }}>
            <Typography variant="h6">
              {vector.length.toLocaleString()}
            </Typography>
            <Typography variant="caption" color="text.secondary">
              Dimensions
            </Typography>
          </Box>
        </Box>

        {/* Chart */}
        <Box sx={{ height: 400, width: '100%' }}>
          <ResponsiveContainer width="100%" height="100%">
            {renderChart()}
          </ResponsiveContainer>
        </Box>

        {/* Additional Info */}
        <Box sx={{ mt: 2, p: 2, bgcolor: 'grey.50', borderRadius: 1 }}>
          <Typography variant="caption" color="text.secondary">
            Showing {chartData.length} of {vector.length} dimensions
            {vectorId && ` • Vector ID: ${vectorId}`}
          </Typography>
        </Box>
      </CardContent>
    </Card>
  );
};

export default VectorVisualization;
