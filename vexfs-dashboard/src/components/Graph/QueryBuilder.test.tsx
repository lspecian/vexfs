import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryBuilder } from './QueryBuilder';
import type { NodeResponse, EdgeResponse } from '../../types/graph';

// Mock the API service
jest.mock('../../services/api', () => ({
  vexfsApi: {
    executeTraversal: jest.fn(),
    breadthFirstSearch: jest.fn(),
    depthFirstSearch: jest.fn(),
    findShortestPath: jest.fn(),
  },
}));

const mockNodes: NodeResponse[] = [
  {
    id: 'node1',
    inode_number: 1,
    node_type: 'File',
    properties: { name: 'test.txt' },
    outgoing_edges: ['edge1'],
    incoming_edges: [],
    created_at: '2023-01-01T00:00:00Z',
    updated_at: '2023-01-01T00:00:00Z',
  },
  {
    id: 'node2',
    inode_number: 2,
    node_type: 'Directory',
    properties: { name: 'testdir' },
    outgoing_edges: [],
    incoming_edges: ['edge1'],
    created_at: '2023-01-01T00:00:00Z',
    updated_at: '2023-01-01T00:00:00Z',
  },
];

const mockEdges: EdgeResponse[] = [
  {
    id: 'edge1',
    source_id: 'node1',
    target_id: 'node2',
    edge_type: 'Contains',
    weight: 1.0,
    properties: {},
    created_at: '2023-01-01T00:00:00Z',
    updated_at: '2023-01-01T00:00:00Z',
  },
];

describe('QueryBuilder', () => {
  const defaultProps = {
    nodes: mockNodes,
    edges: mockEdges,
    onQueryExecute: jest.fn(),
    onResultsHighlight: jest.fn(),
    disabled: false,
  };

  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders the query builder interface', () => {
    render(<QueryBuilder {...defaultProps} />);
    
    expect(screen.getByText('Graph Traversal Query Builder')).toBeInTheDocument();
    expect(screen.getByText('Build Query')).toBeInTheDocument();
    expect(screen.getByText('Templates')).toBeInTheDocument();
    expect(screen.getByText('History')).toBeInTheDocument();
    expect(screen.getByText('Saved Queries')).toBeInTheDocument();
  });

  it('shows validation error when no starting node is selected', () => {
    render(<QueryBuilder {...defaultProps} />);
    
    const executeButton = screen.getByRole('button', { name: /execute query/i });
    fireEvent.click(executeButton);
    
    expect(screen.getByText('Please select a starting node')).toBeInTheDocument();
  });

  it('allows switching between tabs', () => {
    render(<QueryBuilder {...defaultProps} />);
    
    const templatesTab = screen.getByText('Templates');
    fireEvent.click(templatesTab);
    
    expect(screen.getByText('Query Templates')).toBeInTheDocument();
    expect(screen.getByText('Pre-built query templates for common graph traversal patterns')).toBeInTheDocument();
  });

  it('displays query templates with different categories', () => {
    render(<QueryBuilder {...defaultProps} />);
    
    const templatesTab = screen.getByText('Templates');
    fireEvent.click(templatesTab);
    
    expect(screen.getByText('Explore Neighbors')).toBeInTheDocument();
    expect(screen.getByText('Shortest Path')).toBeInTheDocument();
    expect(screen.getByText('Deep Exploration')).toBeInTheDocument();
  });

  it('shows save dialog when save button is clicked', () => {
    render(<QueryBuilder {...defaultProps} />);
    
    const saveButton = screen.getByLabelText('Save Query');
    fireEvent.click(saveButton);
    
    expect(screen.getByText('Save Query')).toBeInTheDocument();
    expect(screen.getByLabelText('Query Name')).toBeInTheDocument();
  });

  it('handles template selection', () => {
    render(<QueryBuilder {...defaultProps} />);
    
    const templatesTab = screen.getByText('Templates');
    fireEvent.click(templatesTab);
    
    const useTemplateButton = screen.getAllByText('Use Template')[0];
    fireEvent.click(useTemplateButton);
    
    // Switch back to Build Query tab to see the loaded template
    const buildTab = screen.getByText('Build Query');
    fireEvent.click(buildTab);
    
    // The template should have loaded some default values
    expect(screen.getByDisplayValue('Explore Neighbors')).toBeInTheDocument();
  });

  it('enables preview and execute buttons when query is valid', async () => {
    render(<QueryBuilder {...defaultProps} />);
    
    // Initially buttons should be disabled due to no starting node
    const executeButton = screen.getByRole('button', { name: /execute query/i });
    const previewButton = screen.getByLabelText('Preview Query');
    
    expect(executeButton).toBeDisabled();
    expect(previewButton).toBeDisabled();
  });

  it('displays query history when available', () => {
    // Mock localStorage to have some history
    const mockHistory = [
      {
        id: 'history-1',
        name: 'Test Query',
        algorithm: 'BreadthFirstSearch',
        createdAt: '2023-01-01T00:00:00Z',
        updatedAt: '2023-01-01T00:00:00Z',
        nodeFilters: {},
        edgeFilters: {},
      },
    ];
    
    Storage.prototype.getItem = jest.fn(() => JSON.stringify(mockHistory));
    
    render(<QueryBuilder {...defaultProps} />);
    
    const historyTab = screen.getByText('History');
    fireEvent.click(historyTab);
    
    expect(screen.getByText('Query History')).toBeInTheDocument();
  });

  it('handles export functionality', () => {
    // Mock URL.createObjectURL and related functions
    global.URL.createObjectURL = jest.fn(() => 'mock-url');
    global.URL.revokeObjectURL = jest.fn();
    
    const mockLink = {
      href: '',
      download: '',
      click: jest.fn(),
    };
    
    document.createElement = jest.fn().mockImplementation((tagName) => {
      if (tagName === 'a') {
        return mockLink;
      }
      return document.createElement(tagName);
    });
    
    document.body.appendChild = jest.fn();
    document.body.removeChild = jest.fn();
    
    render(<QueryBuilder {...defaultProps} />);
    
    const exportButton = screen.getByLabelText('Export Query');
    fireEvent.click(exportButton);
    
    expect(global.URL.createObjectURL).toHaveBeenCalled();
    expect(mockLink.click).toHaveBeenCalled();
  });
});