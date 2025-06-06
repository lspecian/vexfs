import React, { useState } from 'react';
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
} from '@mui/material';
import { useForm, Controller, type FieldValues } from 'react-hook-form';

interface CreateCollectionFormProps {
  open: boolean;
  onClose: () => void;
  onSubmit: (data: CreateCollectionData) => Promise<void>;
  loading?: boolean;
  error?: string | null;
}

interface CreateCollectionData {
  name: string;
  description?: string;
  vectorSize: number;
  distance: 'cosine' | 'euclidean' | 'dot';
}

const CreateCollectionForm: React.FC<CreateCollectionFormProps> = ({
  open,
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
    formState: { errors, isValid },
  } = useForm<CreateCollectionData>({
    defaultValues: {
      name: '',
      description: '',
      vectorSize: 384,
      distance: 'cosine',
    },
    mode: 'onChange',
  });

  const handleFormSubmit = async (data: CreateCollectionData) => {
    try {
      setSubmitError(null);
      await onSubmit(data);
      reset();
      onClose();
    } catch (err) {
      setSubmitError(
        err instanceof Error ? err.message : 'Failed to create collection'
      );
    }
  };

  const handleClose = () => {
    reset();
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

  const validateVectorSize = (value: number) => {
    if (!value || value <= 0) {
      return 'Vector size must be a positive number';
    }
    if (value > 4096) {
      return 'Vector size cannot exceed 4096 dimensions';
    }
    if (!Number.isInteger(value)) {
      return 'Vector size must be a whole number';
    }
    return true;
  };

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
          Create New Collection
        </Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
          Collections store vectors with specific configurations for similarity
          search.
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
                  placeholder="e.g., documents, embeddings, vectors"
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
                  helperText="Describe what this collection will store"
                  fullWidth
                />
              )}
            />

            {/* Vector Size */}
            <Controller
              name="vectorSize"
              control={control}
              rules={{ validate: validateVectorSize }}
              render={({ field }: { field: FieldValues }) => (
                <TextField
                  {...field}
                  label="Vector Dimensions"
                  type="number"
                  placeholder="384"
                  error={!!errors.vectorSize}
                  helperText={
                    errors.vectorSize?.message ||
                    'Number of dimensions in each vector (e.g., 384 for sentence transformers)'
                  }
                  required
                  fullWidth
                  onChange={e => field.onChange(parseInt(e.target.value, 10))}
                />
              )}
            />

            {/* Distance Metric */}
            <Controller
              name="distance"
              control={control}
              render={({ field }: { field: FieldValues }) => (
                <FormControl fullWidth>
                  <InputLabel>Distance Metric</InputLabel>
                  <Select {...field} label="Distance Metric">
                    <MenuItem value="cosine">
                      <Box>
                        <Typography variant="body2" sx={{ fontWeight: 500 }}>
                          Cosine
                        </Typography>
                        <Typography variant="caption" color="text.secondary">
                          Best for normalized vectors (recommended)
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
                </FormControl>
              )}
            />

            {/* Info Box */}
            <Alert severity="info" sx={{ mt: 2 }}>
              <Typography variant="body2">
                <strong>Note:</strong> Vector dimensions and distance metric
                cannot be changed after creation. Make sure these settings match
                your embedding model.
              </Typography>
            </Alert>
          </Box>
        </DialogContent>

        <DialogActions sx={{ px: 3, pb: 3 }}>
          <Button onClick={handleClose} disabled={loading}>
            Cancel
          </Button>
          <Button
            type="submit"
            variant="contained"
            disabled={!isValid || loading}
            startIcon={loading ? <CircularProgress size={16} /> : null}
            sx={{ borderRadius: 2 }}
          >
            {loading ? 'Creating...' : 'Create Collection'}
          </Button>
        </DialogActions>
      </form>
    </Dialog>
  );
};

export default CreateCollectionForm;
