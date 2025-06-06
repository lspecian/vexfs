import React, { useState } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  Button,
  Box,
  Typography,
  Alert,
  CircularProgress,
  Chip,
} from '@mui/material';
import { Warning as WarningIcon } from '@mui/icons-material';
import type { VexFSCollection } from '../../types';

interface DeleteCollectionDialogProps {
  open: boolean;
  collection: VexFSCollection | null;
  onClose: () => void;
  onConfirm: () => Promise<void>;
  loading?: boolean;
  error?: string | null;
}

const DeleteCollectionDialog: React.FC<DeleteCollectionDialogProps> = ({
  open,
  collection,
  onClose,
  onConfirm,
  loading = false,
  error = null,
}) => {
  const [confirmationText, setConfirmationText] = useState('');
  const [submitError, setSubmitError] = useState<string | null>(null);

  const isConfirmationValid =
    collection && confirmationText === collection.name;

  const handleConfirm = async () => {
    if (!isConfirmationValid) return;

    try {
      setSubmitError(null);
      await onConfirm();
      setConfirmationText('');
      onClose();
    } catch (err) {
      setSubmitError(
        err instanceof Error ? err.message : 'Failed to delete collection'
      );
    }
  };

  const handleClose = () => {
    setConfirmationText('');
    setSubmitError(null);
    onClose();
  };

  if (!collection) {
    return null;
  }

  return (
    <Dialog
      open={open}
      onClose={handleClose}
      maxWidth="sm"
      fullWidth
      PaperProps={{
        sx: { borderRadius: 2 },
      }}
    >
      <DialogTitle>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
          <WarningIcon color="error" />
          <Box>
            <Typography variant="h6" component="h2" sx={{ fontWeight: 600 }}>
              Delete Collection
            </Typography>
            <Typography variant="body2" color="text.secondary">
              This action cannot be undone
            </Typography>
          </Box>
        </Box>
      </DialogTitle>

      <DialogContent>
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
          {(error || submitError) && (
            <Alert severity="error">{error || submitError}</Alert>
          )}

          {/* Warning Alert */}
          <Alert severity="error" variant="outlined">
            <Typography variant="body2" sx={{ fontWeight: 500, mb: 1 }}>
              You are about to permanently delete this collection:
            </Typography>
            <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
              <Typography variant="body2">
                <strong>Name:</strong> {collection.name}
              </Typography>
              {collection.description && (
                <Typography variant="body2">
                  <strong>Description:</strong> {collection.description}
                </Typography>
              )}
              <Typography variant="body2">
                <strong>Vectors:</strong>{' '}
                {collection.pointsCount.toLocaleString()}
              </Typography>
              <Typography variant="body2">
                <strong>Dimensions:</strong> {collection.vectorSize}
              </Typography>
              <Typography variant="body2">
                <strong>Distance Metric:</strong>{' '}
                <Chip
                  label={collection.distance}
                  size="small"
                  color="primary"
                  variant="outlined"
                />
              </Typography>
            </Box>
          </Alert>

          {/* Data Loss Warning */}
          <Alert severity="warning">
            <Typography variant="body2">
              <strong>Data Loss Warning:</strong>
            </Typography>
            <Box component="ul" sx={{ mt: 1, mb: 0, pl: 2 }}>
              <li>
                <Typography variant="body2">
                  All {collection.pointsCount.toLocaleString()} vectors will be
                  permanently deleted
                </Typography>
              </li>
              <li>
                <Typography variant="body2">
                  All associated metadata and payloads will be lost
                </Typography>
              </li>
              <li>
                <Typography variant="body2">
                  This action cannot be undone or recovered
                </Typography>
              </li>
              <li>
                <Typography variant="body2">
                  Any applications using this collection will stop working
                </Typography>
              </li>
            </Box>
          </Alert>

          {/* Confirmation Input */}
          <Box>
            <Typography variant="body2" sx={{ mb: 2, fontWeight: 500 }}>
              To confirm deletion, type the collection name exactly as shown:
            </Typography>
            <Typography
              variant="body2"
              sx={{
                mb: 2,
                p: 1,
                bgcolor: 'grey.100',
                borderRadius: 1,
                fontFamily: 'monospace',
                border: '1px solid',
                borderColor: 'grey.300',
              }}
            >
              {collection.name}
            </Typography>
            <TextField
              fullWidth
              label="Collection Name"
              placeholder={`Type "${collection.name}" to confirm`}
              value={confirmationText}
              onChange={e => setConfirmationText(e.target.value)}
              error={confirmationText.length > 0 && !isConfirmationValid}
              helperText={
                confirmationText.length > 0 && !isConfirmationValid
                  ? 'Collection name does not match'
                  : 'Type the exact collection name to enable deletion'
              }
              autoComplete="off"
              sx={{
                '& .MuiOutlinedInput-root': {
                  fontFamily: 'monospace',
                },
              }}
            />
          </Box>

          {/* Final Warning */}
          <Alert severity="error" variant="filled">
            <Typography variant="body2" sx={{ fontWeight: 500 }}>
              ⚠️ FINAL WARNING: This will permanently delete{' '}
              {collection.pointsCount.toLocaleString()} vectors and cannot be
              undone!
            </Typography>
          </Alert>
        </Box>
      </DialogContent>

      <DialogActions sx={{ px: 3, pb: 3 }}>
        <Button onClick={handleClose} disabled={loading}>
          Cancel
        </Button>
        <Button
          onClick={handleConfirm}
          variant="contained"
          color="error"
          disabled={!isConfirmationValid || loading}
          startIcon={loading ? <CircularProgress size={16} /> : null}
          sx={{ borderRadius: 2 }}
        >
          {loading ? 'Deleting...' : 'Delete Collection'}
        </Button>
      </DialogActions>
    </Dialog>
  );
};

export default DeleteCollectionDialog;
