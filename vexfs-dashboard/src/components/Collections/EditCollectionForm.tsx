import React, { useState, useEffect } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  Button,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Box,
  Typography,
  Alert,
  CircularProgress,
  Chip,
} from '@mui/material';
import { useForm, Controller, type FieldValues } from 'react-hook-form';
import type { VexFSCollection } from '../../types';

interface EditCollectionFormProps {
  open: boolean;
  collection: VexFSCollection | null;
  onClose: () => void;
  onSubmit: (data: EditCollectionData) => Promise<void>;
  loading?: boolean;
  error?: string | null;
}

interface EditCollectionData {
  name: string;
  description?: string;
  vectorSize: number;
  distance: 'cosine' | 'euclidean' | 'dot';
}

const EditCollectionForm: React.FC<EditCollectionFormProps> = ({
  open,
  collection,
  onClose,
  onSubmit,
  loading = false,
  error = null,
}) => {
  const [submitError, setSubmitError] = useState<string | null>(null);

  const {
    control,
    handleSubmit,
    reset,
    formState: { errors, isValid, isDirty },
  } = useForm<EditCollectionData>({
    defaultValues: {
      name: '',
      description: '',
      vectorSize: 384,
      distance: 'cosine',
    },
    mode: 'onChange',
  });

  // Update form when collection changes
  useEffect(() => {
    if (collection) {
      reset({
        name: collection.name,
        description: collection.description || '',
        vectorSize: collection.vectorSize,
        distance: collection.distance,
      });
    }
  }, [collection, reset]);

  const handleFormSubmit = async (data: EditCollectionData) => {
    try {
      setSubmitError(null);
      await onSubmit(data);
      onClose();
    } catch (err) {
      setSubmitError(
        err instanceof Error ? err.message : 'Failed to update collection'
      );
    }
  };

  const handleClose = () => {
    setSubmitError(null);
    onClose();
  };

  const validateCollectionName = (value: string) => {
    if (!value.trim()) {
      return 'Collection name is required';
    }
    if (value.length < 3) {
      return 'Collection name must be at least 3 characters';
    }
    if (value.length > 50) {
      return 'Collection name must be less than 50 characters';
    }
    if (!/^[a-zA-Z0-9_-]+$/.test(value)) {
      return 'Collection name can only contain letters, numbers, hyphens, and underscores';
    }
    return true;
  };

  const hasVectors = Boolean(collection && collection.pointsCount > 0);

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
        <Typography variant="h6" component="h2" sx={{ fontWeight: 600 }}>
          Edit Collection
        </Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
          Update collection settings and metadata.
        </Typography>
      </DialogTitle>

      <form onSubmit={handleSubmit(handleFormSubmit)}>
        <DialogContent>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
            {(error || submitError) && (
              <Alert severity="error">{error || submitError}</Alert>
            )}

            {/* Collection Name */}
            <Controller
              name="name"
              control={control}
              rules={{ validate: validateCollectionName }}
              render={({ field }: { field: FieldValues }) => (
                <TextField
                  {...field}
                  label="Collection Name"
                  error={!!errors.name}
                  helperText={
                    errors.name?.message ||
                    'Unique identifier for your collection'
                  }
                  required
                  fullWidth
                />
              )}
            />

            {/* Description */}
            <Controller
              name="description"
              control={control}
              render={({ field }: { field: FieldValues }) => (
                <TextField
                  {...field}
                  label="Description"
                  placeholder="Optional description of this collection"
                  multiline
                  rows={3}
                  helperText="Describe what this collection stores"
                  fullWidth
                />
              )}
            />

            {/* Vector Size - Read-only if has vectors */}
            <Controller
              name="vectorSize"
              control={control}
              render={({ field }: { field: FieldValues }) => (
                <TextField
                  {...field}
                  label="Vector Dimensions"
                  type="number"
                  error={!!errors.vectorSize}
                  helperText={
                    hasVectors
                      ? 'Cannot be changed when collection contains vectors'
                      : 'Number of dimensions in each vector'
                  }
                  required
                  fullWidth
                  disabled={hasVectors}
                  InputProps={{
                    endAdornment: hasVectors ? (
                      <Chip
                        label="Read-only"
                        size="small"
                        color="warning"
                        variant="outlined"
                      />
                    ) : null,
                  }}
                />
              )}
            />

            {/* Distance Metric - Read-only if has vectors */}
            <Controller
              name="distance"
              control={control}
              render={({ field }: { field: FieldValues }) => (
                <FormControl fullWidth disabled={hasVectors}>
                  <InputLabel>Distance Metric</InputLabel>
                  <Select
                    {...field}
                    label="Distance Metric"
                    endAdornment={
                      hasVectors ? (
                        <Chip
                          label="Read-only"
                          size="small"
                          color="warning"
                          variant="outlined"
                          sx={{ mr: 2 }}
                        />
                      ) : null
                    }
                  >
                    <MenuItem value="cosine">
                      <Box>
                        <Typography variant="body2" sx={{ fontWeight: 500 }}>
                          Cosine
                        </Typography>
                        <Typography variant="caption" color="text.secondary">
                          Best for normalized vectors
                        </Typography>
                      </Box>
                    </MenuItem>
                    <MenuItem value="euclidean">
                      <Box>
                        <Typography variant="body2" sx={{ fontWeight: 500 }}>
                          Euclidean
                        </Typography>
                        <Typography variant="caption" color="text.secondary">
                          Standard geometric distance
                        </Typography>
                      </Box>
                    </MenuItem>
                    <MenuItem value="dot">
                      <Box>
                        <Typography variant="body2" sx={{ fontWeight: 500 }}>
                          Dot Product
                        </Typography>
                        <Typography variant="caption" color="text.secondary">
                          For unnormalized vectors
                        </Typography>
                      </Box>
                    </MenuItem>
                  </Select>
                  {hasVectors && (
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={{ mt: 0.5 }}
                    >
                      Cannot be changed when collection contains vectors
                    </Typography>
                  )}
                </FormControl>
              )}
            />

            {/* Collection Stats */}
            <Box
              sx={{
                p: 2,
                bgcolor: 'grey.50',
                borderRadius: 1,
                border: '1px solid',
                borderColor: 'grey.200',
              }}
            >
              <Typography variant="subtitle2" gutterBottom>
                Collection Statistics
              </Typography>
              <Box sx={{ display: 'flex', gap: 3 }}>
                <Box>
                  <Typography variant="caption" color="text.secondary">
                    Vectors
                  </Typography>
                  <Typography variant="body2" sx={{ fontWeight: 500 }}>
                    {collection.pointsCount.toLocaleString()}
                  </Typography>
                </Box>
                <Box>
                  <Typography variant="caption" color="text.secondary">
                    Created
                  </Typography>
                  <Typography variant="body2" sx={{ fontWeight: 500 }}>
                    {new Date(collection.createdAt).toLocaleDateString()}
                  </Typography>
                </Box>
                <Box>
                  <Typography variant="caption" color="text.secondary">
                    Updated
                  </Typography>
                  <Typography variant="body2" sx={{ fontWeight: 500 }}>
                    {new Date(collection.updatedAt).toLocaleDateString()}
                  </Typography>
                </Box>
              </Box>
            </Box>

            {hasVectors && (
              <Alert severity="warning">
                <Typography variant="body2">
                  <strong>Note:</strong> Vector dimensions and distance metric
                  cannot be changed because this collection contains{' '}
                  {collection.pointsCount.toLocaleString()} vectors.
                </Typography>
              </Alert>
            )}
          </Box>
        </DialogContent>

        <DialogActions sx={{ px: 3, pb: 3 }}>
          <Button onClick={handleClose} disabled={loading}>
            Cancel
          </Button>
          <Button
            type="submit"
            variant="contained"
            disabled={!isValid || !isDirty || loading}
            startIcon={loading ? <CircularProgress size={16} /> : null}
            sx={{ borderRadius: 2 }}
          >
            {loading ? 'Updating...' : 'Update Collection'}
          </Button>
        </DialogActions>
      </form>
    </Dialog>
  );
};

export default EditCollectionForm;
