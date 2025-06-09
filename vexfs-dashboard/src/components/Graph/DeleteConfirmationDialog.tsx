import React from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  Typography,
  Box,
  Alert,
  CircularProgress,
  Chip,
} from '@mui/material';
import {
  Warning as WarningIcon,
  Delete as DeleteIcon,
} from '@mui/icons-material';
import type { NodeResponse, EdgeResponse } from '../../types/graph';

interface DeleteConfirmationDialogProps {
  open: boolean;
  onClose: () => void;
  onConfirm: () => Promise<void>;
  item: NodeResponse | EdgeResponse | null;
  itemType: 'node' | 'edge';
  loading?: boolean;
  error?: string | null;
}

const DeleteConfirmationDialog: React.FC<DeleteConfirmationDialogProps> = ({
  open,
  onClose,
  onConfirm,
  item,
  itemType,
  loading = false,
  error = null,
}) => {
  if (!item) return null;

  const handleConfirm = async () => {
    try {
      await onConfirm();
      onClose();
    } catch (err) {
      // Error handling is done by parent component
      console.error(`Failed to delete ${itemType}:`, err);
    }
  };

  const getItemDisplayName = () => {
    if (itemType === 'node') {
      const node = item as NodeResponse;
      return node.properties?.name || node.properties?.path || `Node ${node.id}`;
    } else {
      const edge = item as EdgeResponse;
      return `Edge ${edge.id} (${edge.edge_type})`;
    }
  };

  const getItemDetails = () => {
    if (itemType === 'node') {
      const node = item as NodeResponse;
      return {
        type: node.node_type,
        id: node.id,
        inode: node.inode_number,
        connections: node.incoming_edges.length + node.outgoing_edges.length,
        properties: Object.keys(node.properties).length,
      };
    } else {
      const edge = item as EdgeResponse;
      return {
        type: edge.edge_type,
        id: edge.id,
        source: edge.source_id,
        target: edge.target_id,
        weight: edge.weight,
        properties: Object.keys(edge.properties).length,
      };
    }
  };

  const details = getItemDetails();

  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth="sm"
      fullWidth
      PaperProps={{
        sx: { borderRadius: 2 },
      }}
    >
      <DialogTitle>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <WarningIcon color="warning" />
          <Typography variant="h6" component="h2" sx={{ fontWeight: 600 }}>
            Delete {itemType === 'node' ? 'Node' : 'Edge'}
          </Typography>
        </Box>
        <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
          This action cannot be undone. Please confirm you want to delete this {itemType}.
        </Typography>
      </DialogTitle>

      <DialogContent>
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
          {error && (
            <Alert severity="error">{error}</Alert>
          )}

          {/* Item Information */}
          <Box>
            <Typography variant="h6" gutterBottom>
              {getItemDisplayName()}
            </Typography>
            
            <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1, mb: 2 }}>
              <Chip 
                label={`Type: ${details.type}`} 
                size="small" 
                variant="outlined" 
              />
              <Chip 
                label={`ID: ${details.id}`} 
                size="small" 
                variant="outlined" 
              />
              {itemType === 'node' && (
                <>
                  <Chip 
                    label={`Inode: ${details.inode}`} 
                    size="small" 
                    variant="outlined" 
                  />
                  <Chip
                    label={`${details.connections} connections`}
                    size="small"
                    variant="outlined"
                    color={(details.connections && details.connections > 0) ? 'warning' : 'default'}
                  />
                </>
              )}
              {itemType === 'edge' && (
                <>
                  <Chip 
                    label={`${details.source} → ${details.target}`} 
                    size="small" 
                    variant="outlined" 
                  />
                  <Chip 
                    label={`Weight: ${details.weight}`} 
                    size="small" 
                    variant="outlined" 
                  />
                </>
              )}
              <Chip 
                label={`${details.properties} properties`} 
                size="small" 
                variant="outlined" 
              />
            </Box>
          </Box>

          {/* Warning Messages */}
          {itemType === 'node' && details.connections && details.connections > 0 && (
            <Alert severity="warning">
              <Typography variant="body2">
                <strong>Warning:</strong> This node has {details.connections} connected edge(s).
                Deleting this node will also remove all connected edges.
              </Typography>
            </Alert>
          )}

          <Alert severity="error">
            <Typography variant="body2">
              <strong>Permanent Action:</strong> This {itemType} and all its data will be 
              permanently removed from the VexGraph. This action cannot be undone.
            </Typography>
          </Alert>

          {/* Confirmation Text */}
          <Box sx={{ p: 2, backgroundColor: 'grey.50', borderRadius: 1 }}>
            <Typography variant="body2" color="text.secondary">
              Are you sure you want to delete <strong>{getItemDisplayName()}</strong>?
            </Typography>
          </Box>
        </Box>
      </DialogContent>

      <DialogActions sx={{ px: 3, pb: 3 }}>
        <Button 
          onClick={onClose} 
          disabled={loading}
          variant="outlined"
        >
          Cancel
        </Button>
        <Button
          onClick={handleConfirm}
          disabled={loading}
          variant="contained"
          color="error"
          startIcon={loading ? <CircularProgress size={16} /> : <DeleteIcon />}
          sx={{ borderRadius: 2 }}
        >
          {loading ? 'Deleting...' : `Delete ${itemType === 'node' ? 'Node' : 'Edge'}`}
        </Button>
      </DialogActions>
    </Dialog>
  );
};

export default DeleteConfirmationDialog;