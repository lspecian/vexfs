import React, { useState } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  FormControl,
  FormLabel,
  RadioGroup,
  FormControlLabel,
  Radio,
  Checkbox,
  FormGroup,
  Typography,
  Box,
  Chip,
  Alert,
  LinearProgress,
  Divider,
} from '@mui/material';
import {
  Download as DownloadIcon,
  PictureAsPdf as PdfIcon,
  TableChart as CsvIcon,
  Code as JsonIcon,
  Image as ImageIcon,
} from '@mui/icons-material';

import type { 
  PerformanceMetrics,
  GrowthData,
  DataQualityMetrics 
} from './GraphAnalyticsDashboard';
import type { GraphStatistics } from '../../types/graph';

export interface AnalyticsExportProps {
  open: boolean;
  onClose: () => void;
  onExport: (format: string) => void;
  data: {
    statistics?: GraphStatistics | null;
    performance?: PerformanceMetrics | null;
    growth?: GrowthData | null;
    quality?: DataQualityMetrics | null;
  };
}

export const AnalyticsExport: React.FC<AnalyticsExportProps> = ({
  open,
  onClose,
  onExport,
  data,
}) => {
  const [exportFormat, setExportFormat] = useState<'json' | 'csv' | 'pdf' | 'png'>('json');
  const [selectedSections, setSelectedSections] = useState({
    statistics: true,
    performance: true,
    growth: true,
    quality: true,
  });
  const [isExporting, setIsExporting] = useState(false);

  // Export format options
  const formatOptions = [
    {
      value: 'json',
      label: 'JSON',
      description: 'Raw data in JSON format',
      icon: <JsonIcon />,
      supported: ['statistics', 'performance', 'growth', 'quality'],
    },
    {
      value: 'csv',
      label: 'CSV',
      description: 'Tabular data for spreadsheets',
      icon: <CsvIcon />,
      supported: ['statistics', 'performance', 'growth'],
    },
    {
      value: 'pdf',
      label: 'PDF Report',
      description: 'Formatted analytics report',
      icon: <PdfIcon />,
      supported: ['statistics', 'performance', 'growth', 'quality'],
    },
    {
      value: 'png',
      label: 'PNG Images',
      description: 'Chart images for presentations',
      icon: <ImageIcon />,
      supported: ['performance', 'growth', 'quality'],
    },
  ];

  // Data sections
  const dataSections = [
    {
      key: 'statistics' as keyof typeof selectedSections,
      label: 'Graph Statistics',
      description: 'Node/edge counts, topology metrics',
      available: !!data.statistics,
      size: data.statistics ? Object.keys(data.statistics).length : 0,
    },
    {
      key: 'performance' as keyof typeof selectedSections,
      label: 'Performance Metrics',
      description: 'Query performance, resource usage',
      available: !!data.performance,
      size: data.performance ? Object.keys(data.performance).length : 0,
    },
    {
      key: 'growth' as keyof typeof selectedSections,
      label: 'Growth Analytics',
      description: 'Historical trends, activity patterns',
      available: !!data.growth,
      size: data.growth ? Object.keys(data.growth).length : 0,
    },
    {
      key: 'quality' as keyof typeof selectedSections,
      label: 'Quality Metrics',
      description: 'Data completeness, consistency scores',
      available: !!data.quality,
      size: data.quality ? Object.keys(data.quality).length : 0,
    },
  ];

  // Handle section toggle
  const handleSectionToggle = (section: keyof typeof selectedSections) => {
    setSelectedSections(prev => ({
      ...prev,
      [section]: !prev[section],
    }));
  };

  // Handle export
  const handleExport = async () => {
    setIsExporting(true);
    
    try {
      // Simulate export process
      await new Promise(resolve => setTimeout(resolve, 2000));
      
      // Prepare export data
      const exportData: any = {
        timestamp: new Date().toISOString(),
        format: exportFormat,
        sections: {},
      };

      // Include selected sections
      if (selectedSections.statistics && data.statistics) {
        exportData.sections.statistics = data.statistics;
      }
      if (selectedSections.performance && data.performance) {
        exportData.sections.performance = data.performance;
      }
      if (selectedSections.growth && data.growth) {
        exportData.sections.growth = data.growth;
      }
      if (selectedSections.quality && data.quality) {
        exportData.sections.quality = data.quality;
      }

      // Mock file download based on format
      switch (exportFormat) {
        case 'json':
          downloadFile(
            JSON.stringify(exportData, null, 2),
            'vexgraph-analytics.json',
            'application/json'
          );
          break;
        case 'csv':
          downloadFile(
            convertToCSV(exportData),
            'vexgraph-analytics.csv',
            'text/csv'
          );
          break;
        case 'pdf':
          // In a real implementation, you'd generate a PDF
          alert('PDF export would be generated here');
          break;
        case 'png':
          // In a real implementation, you'd capture chart images
          alert('PNG export would be generated here');
          break;
      }

      onExport(exportFormat);
      onClose();
    } catch (error) {
      console.error('Export failed:', error);
    } finally {
      setIsExporting(false);
    }
  };

  // Helper function to download file
  const downloadFile = (content: string, filename: string, mimeType: string) => {
    const blob = new Blob([content], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  };

  // Helper function to convert data to CSV
  const convertToCSV = (data: any): string => {
    const rows: string[] = [];
    
    // Add header
    rows.push('Section,Metric,Value,Description');
    
    // Process each section
    Object.entries(data.sections).forEach(([sectionName, sectionData]: [string, any]) => {
      if (sectionName === 'statistics' && sectionData) {
        rows.push(`Statistics,Node Count,${sectionData.node_count},Total number of nodes`);
        rows.push(`Statistics,Edge Count,${sectionData.edge_count},Total number of edges`);
        rows.push(`Statistics,Average Degree,${sectionData.average_degree.toFixed(2)},Average connections per node`);
        rows.push(`Statistics,Density,${sectionData.density.toFixed(4)},Graph density ratio`);
      }
      
      if (sectionName === 'performance' && sectionData) {
        rows.push(`Performance,Avg Query Time,${sectionData.query_performance.average_query_time_ms.toFixed(1)}ms,Average query response time`);
        rows.push(`Performance,Queries Per Second,${sectionData.query_performance.queries_per_second.toFixed(0)},Query throughput`);
        rows.push(`Performance,Success Rate,${(sectionData.query_performance.success_rate * 100).toFixed(1)}%,Query success percentage`);
      }
      
      if (sectionName === 'growth' && sectionData) {
        rows.push(`Growth,Creation Rate,${sectionData.activity_patterns.creation_rate_per_hour.toFixed(1)}/hr,Node creation rate`);
        rows.push(`Growth,Modification Rate,${sectionData.activity_patterns.modification_rate_per_hour.toFixed(1)}/hr,Node modification rate`);
        rows.push(`Growth,Deletion Rate,${sectionData.activity_patterns.deletion_rate_per_hour.toFixed(1)}/hr,Node deletion rate`);
      }
    });
    
    return rows.join('\n');
  };

  // Calculate total data size
  const totalDataSize = dataSections
    .filter(section => selectedSections[section.key] && section.available)
    .reduce((sum, section) => sum + section.size, 0);

  // Check if any sections are selected
  const hasSelectedSections = Object.values(selectedSections).some(Boolean);

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>
        <Box display="flex" alignItems="center" gap={1}>
          <DownloadIcon />
          Export Analytics Data
        </Box>
      </DialogTitle>
      
      <DialogContent>
        {isExporting && (
          <Box mb={2}>
            <LinearProgress />
            <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
              Preparing export...
            </Typography>
          </Box>
        )}

        {/* Export Format Selection */}
        <Box mb={3}>
          <FormControl component="fieldset">
            <FormLabel component="legend">Export Format</FormLabel>
            <RadioGroup
              value={exportFormat}
              onChange={(e) => setExportFormat(e.target.value as typeof exportFormat)}
            >
              {formatOptions.map((option) => (
                <FormControlLabel
                  key={option.value}
                  value={option.value}
                  control={<Radio />}
                  label={
                    <Box display="flex" alignItems="center" gap={1}>
                      {option.icon}
                      <Box>
                        <Typography variant="body2" fontWeight="medium">
                          {option.label}
                        </Typography>
                        <Typography variant="caption" color="text.secondary">
                          {option.description}
                        </Typography>
                      </Box>
                    </Box>
                  }
                />
              ))}
            </RadioGroup>
          </FormControl>
        </Box>

        <Divider sx={{ my: 2 }} />

        {/* Data Section Selection */}
        <Box mb={3}>
          <FormLabel component="legend">Data Sections</FormLabel>
          <FormGroup>
            {dataSections.map((section) => (
              <FormControlLabel
                key={section.key}
                control={
                  <Checkbox
                    checked={selectedSections[section.key]}
                    onChange={() => handleSectionToggle(section.key)}
                    disabled={!section.available}
                  />
                }
                label={
                  <Box display="flex" alignItems="center" gap={1}>
                    <Box>
                      <Typography 
                        variant="body2" 
                        color={section.available ? 'text.primary' : 'text.disabled'}
                      >
                        {section.label}
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        {section.description}
                      </Typography>
                    </Box>
                    {section.available ? (
                      <Chip label={`${section.size} fields`} size="small" />
                    ) : (
                      <Chip label="No data" size="small" color="default" />
                    )}
                  </Box>
                }
              />
            ))}
          </FormGroup>
        </Box>

        {/* Export Summary */}
        <Box mb={2}>
          <Typography variant="subtitle2" gutterBottom>
            Export Summary
          </Typography>
          <Box display="flex" gap={1} flexWrap="wrap">
            <Chip 
              label={`Format: ${exportFormat.toUpperCase()}`} 
              color="primary" 
              size="small" 
            />
            <Chip 
              label={`Sections: ${Object.values(selectedSections).filter(Boolean).length}`} 
              color="secondary" 
              size="small" 
            />
            <Chip 
              label={`Fields: ${totalDataSize}`} 
              color="info" 
              size="small" 
            />
          </Box>
        </Box>

        {/* Warnings */}
        {!hasSelectedSections && (
          <Alert severity="warning" sx={{ mb: 2 }}>
            Please select at least one data section to export.
          </Alert>
        )}

        {exportFormat === 'csv' && selectedSections.quality && (
          <Alert severity="info" sx={{ mb: 2 }}>
            Quality metrics may not be fully represented in CSV format. Consider JSON for complete data.
          </Alert>
        )}
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose} disabled={isExporting}>
          Cancel
        </Button>
        <Button
          variant="contained"
          onClick={handleExport}
          disabled={!hasSelectedSections || isExporting}
          startIcon={<DownloadIcon />}
        >
          {isExporting ? 'Exporting...' : 'Export'}
        </Button>
      </DialogActions>
    </Dialog>
  );
};

export default AnalyticsExport;