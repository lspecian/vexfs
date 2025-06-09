import React, { useState, useCallback } from 'react';
import {
  Box,
  Button,
  ButtonGroup,
  Tooltip,
  Menu,
  MenuItem,
  ListItemIcon,
  ListItemText,
  Divider,
  Typography,
  Alert,
} from '@mui/material';
import {
  Add as AddIcon,
  Edit as EditIcon,
  Delete as DeleteIcon,
  AccountTree as NodeIcon,
  Timeline as EdgeIcon,
  MoreVert as MoreIcon,
} from '@mui/icons-material';

import NodeManagementDialog from './NodeManagementDialog';
import EdgeManagementDialog from './EdgeManagementDialog';
import DeleteConfirmationDialog from './DeleteConfirmationDialog';
import { vexfsApi } from '../../services/api';
import type { 
  NodeResponse, 
  EdgeResponse, 
  CreateNodeRequest, 
  UpdateNodeRequest,
  CreateEdgeRequest,
  UpdateEdgeRequest 
} from '../../types/graph';

interface NodeEdgeManagerProps {
  nodes: NodeResponse[];
  edges: EdgeResponse[];
  selectedNodes: string[];
  selectedEdges: string[];
  onDataChange: () => void; // Callback to refresh graph data
  disabled?: boolean;
}

type DialogType = 'none' | 'create-node' | 'edit-node' | 'create-edge' | 'edit-edge' | 'delete';
type DeleteTarget = { type: 'node' | 'edge'; item: NodeResponse | EdgeResponse } | null;

const NodeEdgeManager: React.FC<NodeEdgeManagerProps> = ({
  nodes,
  edges,
  selectedNodes,
  selectedEdges,
  onDataChange,
  disabled = false,
}) => {
  const [dialogType, setDialogType] = useState<DialogType>('none');
  const [editingNode, setEditingNode] = useState<NodeResponse | null>(null);
  const [editingEdge, setEditingEdge] = useState<EdgeResponse | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<DeleteTarget>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [menuAnchor, setMenuAnchor] = useState<null | HTMLElement>(null);

  // Menu handlers
  const handleMenuOpen = (event: React.MouseEvent<HTMLElement>) => {
    setMenuAnchor(event.currentTarget);
  };

  const handleMenuClose = () => {
    setMenuAnchor(null);
  };

  // Dialog handlers
  const handleCreateNode = () => {
    setEditingNode(null);
    setDialogType('create-node');
    handleMenuClose();
  };

  const handleEditNode = () => {
    if (selectedNodes.length === 1) {
      const node = nodes.find(n => n.id === selectedNodes[0]);
      if (node) {
        setEditingNode(node);
        setDialogType('edit-node');
      }
    }
    handleMenuClose();
  };

  const handleCreateEdge = () => {
    setEditingEdge(null);
    setDialogType('create-edge');
    handleMenuClose();
  };

  const handleEditEdge = () => {
    if (selectedEdges.length === 1) {
      const edge = edges.find(e => e.id === selectedEdges[0]);
      if (edge) {
        setEditingEdge(edge);
        setDialogType('edit-edge');
      }
    }
    handleMenuClose();
  };

  const handleDeleteSelected = () => {
    if (selectedNodes.length === 1) {
      const node = nodes.find(n => n.id === selectedNodes[0]);
      if (node) {
        setDeleteTarget({ type: 'node', item: node });
        setDialogType('delete');
      }
    } else if (selectedEdges.length === 1) {
      const edge = edges.find(e => e.id === selectedEdges[0]);
      if (edge) {
        setDeleteTarget({ type: 'edge', item: edge });
        setDialogType('delete');
      }
    }
    handleMenuClose();
  };

  const handleCloseDialog = () => {
    setDialogType('none');
    setEditingNode(null);
    setEditingEdge(null);
    setDeleteTarget(null);
    setError(null);
  };

  // API handlers
  const handleNodeSubmit = useCallback(async (data: CreateNodeRequest | UpdateNodeRequest) => {
    try {
      setLoading(true);
      setError(null);

      if (editingNode) {
        // Update existing node
        await vexfsApi.updateNode(editingNode.id, data as UpdateNodeRequest);
      } else {
        // Create new node
        await vexfsApi.createNode(data as CreateNodeRequest);
      }

      onDataChange(); // Refresh graph data
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Operation failed';
      setError(errorMessage);
      throw err; // Re-throw to let dialog handle it
    } finally {
      setLoading(false);
    }
  }, [editingNode, onDataChange]);

  const handleEdgeSubmit = useCallback(async (data: CreateEdgeRequest | UpdateEdgeRequest) => {
    try {
      setLoading(true);
      setError(null);

      if (editingEdge) {
        // Update existing edge
        await vexfsApi.updateEdge(editingEdge.id, data as UpdateEdgeRequest);
      } else {
        // Create new edge
        await vexfsApi.createEdge(data as CreateEdgeRequest);
      }

      onDataChange(); // Refresh graph data
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Operation failed';
      setError(errorMessage);
      throw err; // Re-throw to let dialog handle it
    } finally {
      setLoading(false);
    }
  }, [editingEdge, onDataChange]);

  const handleDeleteConfirm = useCallback(async () => {
    if (!deleteTarget) return;

    try {
      setLoading(true);
      setError(null);

      if (deleteTarget.type === 'node') {
        await vexfsApi.deleteNode(deleteTarget.item.id);
      } else {
        await vexfsApi.deleteEdge(deleteTarget.item.id);
      }

      onDataChange(); // Refresh graph data
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Delete operation failed';
      setError(errorMessage);
      throw err; // Re-throw to let dialog handle it
    } finally {
      setLoading(false);
    }
  }, [deleteTarget, onDataChange]);

  // Check what actions are available
  const canEditNode = selectedNodes.length === 1;
  const canEditEdge = selectedEdges.length === 1;
  const canDelete = selectedNodes.length === 1 || selectedEdges.length === 1;
  const hasSelection = selectedNodes.length > 0 || selectedEdges.length > 0;

  return (
    <Box>
      {/* Action Buttons */}
      <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
        {/* Quick Actions */}
        <ButtonGroup variant="outlined" disabled={disabled}>
          <Tooltip title="Create Node">
            <Button
              startIcon={<NodeIcon />}
              onClick={handleCreateNode}
              size="small"
            >
              Node
            </Button>
          </Tooltip>
          <Tooltip title="Create Edge">
            <Button
              startIcon={<EdgeIcon />}
              onClick={handleCreateEdge}
              size="small"
            >
              Edge
            </Button>
          </Tooltip>
        </ButtonGroup>

        {/* More Actions Menu */}
        <Tooltip title="More Actions">
          <Button
            variant="outlined"
            size="small"
            onClick={handleMenuOpen}
            disabled={disabled}
            startIcon={<MoreIcon />}
          >
            Actions
          </Button>
        </Tooltip>

        <Menu
          anchorEl={menuAnchor}
          open={Boolean(menuAnchor)}
          onClose={handleMenuClose}
          transformOrigin={{ horizontal: 'right', vertical: 'top' }}
          anchorOrigin={{ horizontal: 'right', vertical: 'bottom' }}
        >
          {/* Create Actions */}
          <MenuItem onClick={handleCreateNode}>
            <ListItemIcon>
              <AddIcon fontSize="small" />
            </ListItemIcon>
            <ListItemText>Create Node</ListItemText>
          </MenuItem>
          <MenuItem onClick={handleCreateEdge}>
            <ListItemIcon>
              <AddIcon fontSize="small" />
            </ListItemIcon>
            <ListItemText>Create Edge</ListItemText>
          </MenuItem>

          <Divider />

          {/* Edit Actions */}
          <MenuItem onClick={handleEditNode} disabled={!canEditNode}>
            <ListItemIcon>
              <EditIcon fontSize="small" />
            </ListItemIcon>
            <ListItemText>
              Edit Node
              {!canEditNode && selectedNodes.length > 1 && ' (select one)'}
              {!canEditNode && selectedNodes.length === 0 && ' (select a node)'}
            </ListItemText>
          </MenuItem>
          <MenuItem onClick={handleEditEdge} disabled={!canEditEdge}>
            <ListItemIcon>
              <EditIcon fontSize="small" />
            </ListItemIcon>
            <ListItemText>
              Edit Edge
              {!canEditEdge && selectedEdges.length > 1 && ' (select one)'}
              {!canEditEdge && selectedEdges.length === 0 && ' (select an edge)'}
            </ListItemText>
          </MenuItem>

          <Divider />

          {/* Delete Actions */}
          <MenuItem onClick={handleDeleteSelected} disabled={!canDelete}>
            <ListItemIcon>
              <DeleteIcon fontSize="small" color={canDelete ? 'error' : 'disabled'} />
            </ListItemIcon>
            <ListItemText>
              Delete Selected
              {!canDelete && ' (select one item)'}
            </ListItemText>
          </MenuItem>
        </Menu>

        {/* Selection Info */}
        {hasSelection && (
          <Typography variant="body2" color="text.secondary" sx={{ ml: 2 }}>
            Selected: {selectedNodes.length} node(s), {selectedEdges.length} edge(s)
          </Typography>
        )}
      </Box>

      {/* Error Display */}
      {error && (
        <Alert severity="error" sx={{ mt: 2 }} onClose={() => setError(null)}>
          {error}
        </Alert>
      )}

      {/* Node Management Dialog */}
      <NodeManagementDialog
        open={dialogType === 'create-node' || dialogType === 'edit-node'}
        onClose={handleCloseDialog}
        onSubmit={handleNodeSubmit}
        node={editingNode}
        loading={loading}
        error={error}
      />

      {/* Edge Management Dialog */}
      <EdgeManagementDialog
        open={dialogType === 'create-edge' || dialogType === 'edit-edge'}
        onClose={handleCloseDialog}
        onSubmit={handleEdgeSubmit}
        edge={editingEdge}
        nodes={nodes}
        loading={loading}
        error={error}
      />

      {/* Delete Confirmation Dialog */}
      <DeleteConfirmationDialog
        open={dialogType === 'delete'}
        onClose={handleCloseDialog}
        onConfirm={handleDeleteConfirm}
        item={deleteTarget?.item || null}
        itemType={deleteTarget?.type || 'node'}
        loading={loading}
        error={error}
      />
    </Box>
  );
};

export default NodeEdgeManager;