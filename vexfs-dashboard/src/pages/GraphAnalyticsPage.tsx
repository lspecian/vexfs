import React from 'react';
import { Box, Container, Typography, Paper } from '@mui/material';
import { GraphAnalyticsDashboard } from '../components/Graph';
import type { NodeResponse, EdgeResponse } from '../types/graph';

// Mock data for demonstration
const mockNodes: NodeResponse[] = [
  {
    id: 'node-1',
    inode_number: 1001,
    node_type: 'File',
    properties: { name: 'String', size: 'Integer' },
    outgoing_edges: ['edge-1', 'edge-2'],
    incoming_edges: [],
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  },
  {
    id: 'node-2',
    inode_number: 1002,
    node_type: 'Directory',
    properties: { name: 'String', path: 'String' },
    outgoing_edges: ['edge-3'],
    incoming_edges: ['edge-1'],
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  },
  {
    id: 'node-3',
    inode_number: 1003,
    node_type: 'File',
    properties: { name: 'String', size: 'Integer' },
    outgoing_edges: [],
    incoming_edges: ['edge-2', 'edge-3'],
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  },
];

const mockEdges: EdgeResponse[] = [
  {
    id: 'edge-1',
    source_id: 'node-1',
    target_id: 'node-2',
    edge_type: 'Contains',
    weight: 1.0,
    properties: { relationship: 'String' },
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  },
  {
    id: 'edge-2',
    source_id: 'node-1',
    target_id: 'node-3',
    edge_type: 'References',
    weight: 0.8,
    properties: { relationship: 'String' },
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  },
  {
    id: 'edge-3',
    source_id: 'node-2',
    target_id: 'node-3',
    edge_type: 'Contains',
    weight: 1.0,
    properties: { relationship: 'String' },
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  },
];

const GraphAnalyticsPage: React.FC = () => {
  const handleExport = (data: any, format: string) => {
    console.log('Exporting analytics data:', { data, format });
    // In a real implementation, this would handle the actual export
  };

  return (
    <Container maxWidth="xl" sx={{ py: 3 }}>
      <Paper sx={{ p: 3, mb: 3 }}>
        <Typography variant="h4" gutterBottom>
          VexGraph Analytics Dashboard
        </Typography>
        <Typography variant="body1" color="text.secondary">
          Comprehensive analytics and metrics visualization for VexGraph data. 
          Monitor graph structure, performance metrics, growth patterns, and data quality.
        </Typography>
      </Paper>

      <GraphAnalyticsDashboard
        nodes={mockNodes}
        edges={mockEdges}
        autoRefresh={true}
        refreshInterval={30000}
        onExport={handleExport}
      />
    </Container>
  );
};

export default GraphAnalyticsPage;