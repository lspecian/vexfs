import React from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Button,
  Chip,
  Grid,
  Divider,
  IconButton,
  Alert,
  CircularProgress,
} from '@mui/material';
import {
  Edit as EditIcon,
  Delete as DeleteIcon,
  Add as AddIcon,
  Search as SearchIcon,
  ArrowBack as ArrowBackIcon,
  Storage as StorageIcon,
  Timeline as TimelineIcon,
} from '@mui/icons-material';
import { format } from 'date-fns';
import type { VexFSCollection } from '../../types';

interface CollectionDetailsProps {
  collection: VexFSCollection;
  loading?: boolean;
  error?: string | null;
  onBack: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onAddVectors: () => void;
  onSearchVectors: () => void;
}

const CollectionDetails: React.FC<CollectionDetailsProps> = ({
  collection,
  loading = false,
  error = null,
  onBack,
  onEdit,
  onDelete,
  onAddVectors,
  onSearchVectors,
}) => {
  const formatDate = (dateString: string) => {
    try {
      return format(new Date(dateString), 'PPpp');
    } catch {
      return 'Invalid date';
    }
  };

  const getDistanceMetricColor = (distance: string) => {
    switch (distance) {
      case 'cosine':
        return 'primary';
      case 'euclidean':
        return 'secondary';
      case 'dot':
        return 'success';
      default:
        return 'default';
    }
  };

  const getDistanceMetricDescription = (distance: string) => {
    switch (distance) {
      case 'cosine':
        return 'Measures the cosine of the angle between vectors. Best for normalized vectors.';
      case 'euclidean':
        return 'Measures the straight-line distance between vectors in space.';
      case 'dot':
        return 'Measures the dot product between vectors. Good for unnormalized vectors.';
      default:
        return 'Unknown distance metric';
    }
  };

  if (loading) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
        <CircularProgress />
      </Box>
    );
  }

  if (error) {
    return (
      <Alert severity="error" sx={{ mb: 3 }}>
        Failed to load collection details: {error}
      </Alert>
    );
  }

  return (
    <Box>
      {/* Header with Navigation */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          mb: 3,
          gap: 2,
        }}
      >
        <IconButton onClick={onBack} sx={{ mr: 1 }}>
          <ArrowBackIcon />
        </IconButton>
        <Box sx={{ flexGrow: 1 }}>
          <Typography variant="h4" component="h1" sx={{ fontWeight: 600 }}>
            {collection.name}
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Collection Details
          </Typography>
        </Box>
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button
            variant="outlined"
            startIcon={<SearchIcon />}
            onClick={onSearchVectors}
            sx={{ borderRadius: 2 }}
          >
            Search Vectors
          </Button>
          <Button
            variant="outlined"
            startIcon={<AddIcon />}
            onClick={onAddVectors}
            sx={{ borderRadius: 2 }}
          >
            Add Vectors
          </Button>
          <Button
            variant="outlined"
            startIcon={<EditIcon />}
            onClick={onEdit}
            sx={{ borderRadius: 2 }}
          >
            Edit
          </Button>
          <Button
            variant="outlined"
            color="error"
            startIcon={<DeleteIcon />}
            onClick={onDelete}
            sx={{ borderRadius: 2 }}
          >
            Delete
          </Button>
        </Box>
      </Box>

      <Box sx={{ display: 'flex', flexDirection: { xs: 'column', md: 'row' }, gap: 3 }}>
        {/* Basic Information */}
        <Box sx={{ flex: { xs: '1', md: '2' } }}>
          <Card sx={{ mb: 3 }}>
            <CardContent>
              <Typography variant="h6" gutterBottom sx={{ fontWeight: 600 }}>
                Basic Information
              </Typography>
              <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                <Box>
                  <Typography variant="subtitle2" color="text.secondary">
                    Name
                  </Typography>
                  <Typography variant="body1" sx={{ fontWeight: 500 }}>
                    {collection.name}
                  </Typography>
                </Box>
                {collection.description && (
                  <Box>
                    <Typography variant="subtitle2" color="text.secondary">
                      Description
                    </Typography>
                    <Typography variant="body1">
                      {collection.description}
                    </Typography>
                  </Box>
                )}
                <Box>
                  <Typography variant="subtitle2" color="text.secondary">
                    Collection ID
                  </Typography>
                  <Typography
                    variant="body2"
                    sx={{ fontFamily: 'monospace', color: 'text.secondary' }}
                  >
                    {collection.id}
                  </Typography>
                </Box>
              </Box>
            </CardContent>
          </Card>

          {/* Vector Configuration */}
          <Card sx={{ mb: 3 }}>
            <CardContent>
              <Typography variant="h6" gutterBottom sx={{ fontWeight: 600 }}>
                Vector Configuration
              </Typography>
              <Grid container spacing={3}>
                <Box sx={{ flex: '1', minWidth: '200px' }}>
                  <Box>
                    <Typography variant="subtitle2" color="text.secondary">
                      Vector Dimensions
                    </Typography>
                    <Typography variant="h4" sx={{ fontWeight: 600, mb: 1 }}>
                      {collection.vectorSize}
                    </Typography>
                    <Typography variant="body2" color="text.secondary">
                      Number of dimensions per vector
                    </Typography>
                  </Box>
                </Box>
                <Box sx={{ flex: '1', minWidth: '200px' }}>
                  <Box>
                    <Typography variant="subtitle2" color="text.secondary">
                      Distance Metric
                    </Typography>
                    <Box
                      sx={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: 1,
                        mb: 1,
                      }}
                    >
                      <Chip
                        label={collection.distance}
                        color={
                          getDistanceMetricColor(collection.distance) as any
                        }
                        variant="outlined"
                        size="small"
                      />
                    </Box>
                    <Typography variant="body2" color="text.secondary">
                      {getDistanceMetricDescription(collection.distance)}
                    </Typography>
                  </Box>
                </Box>
              </Grid>
            </CardContent>
          </Card>

          {/* Recent Activity */}
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom sx={{ fontWeight: 600 }}>
                Recent Activity
              </Typography>
              <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
                  <TimelineIcon color="primary" />
                  <Box>
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      Collection Created
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
                      {formatDate(collection.createdAt)}
                    </Typography>
                  </Box>
                </Box>
                <Divider />
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
                  <StorageIcon color="secondary" />
                  <Box>
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      Last Updated
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
                      {formatDate(collection.updatedAt)}
                    </Typography>
                  </Box>
                </Box>
              </Box>
            </CardContent>
          </Card>
        </Box>

        {/* Statistics Sidebar */}
        <Box sx={{ flex: { xs: '1', md: '1' }, maxWidth: { md: '400px' } }}>
          <Card sx={{ mb: 3 }}>
            <CardContent>
              <Typography variant="h6" gutterBottom sx={{ fontWeight: 600 }}>
                Statistics
              </Typography>
              <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
                <Box sx={{ textAlign: 'center' }}>
                  <Typography
                    variant="h3"
                    sx={{ fontWeight: 600, color: 'primary.main' }}
                  >
                    {collection.pointsCount.toLocaleString()}
                  </Typography>
                  <Typography variant="body2" color="text.secondary">
                    Total Vectors
                  </Typography>
                </Box>
                <Divider />
                <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                  <Typography variant="body2" color="text.secondary">
                    Dimensions
                  </Typography>
                  <Typography variant="body2" sx={{ fontWeight: 500 }}>
                    {collection.vectorSize}
                  </Typography>
                </Box>
                <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                  <Typography variant="body2" color="text.secondary">
                    Distance Metric
                  </Typography>
                  <Chip
                    label={collection.distance}
                    size="small"
                    color={getDistanceMetricColor(collection.distance) as any}
                    variant="outlined"
                  />
                </Box>
                <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                  <Typography variant="body2" color="text.secondary">
                    Storage Size
                  </Typography>
                  <Typography variant="body2" sx={{ fontWeight: 500 }}>
                    {(
                      (collection.pointsCount * collection.vectorSize * 4) /
                      1024 /
                      1024
                    ).toFixed(2)}{' '}
                    MB
                  </Typography>
                </Box>
              </Box>
            </CardContent>
          </Card>

          {/* Quick Actions */}
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom sx={{ fontWeight: 600 }}>
                Quick Actions
              </Typography>
              <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
                <Button
                  fullWidth
                  variant="outlined"
                  startIcon={<AddIcon />}
                  onClick={onAddVectors}
                  sx={{ justifyContent: 'flex-start', borderRadius: 2 }}
                >
                  Add Vectors
                </Button>
                <Button
                  fullWidth
                  variant="outlined"
                  startIcon={<SearchIcon />}
                  onClick={onSearchVectors}
                  sx={{ justifyContent: 'flex-start', borderRadius: 2 }}
                >
                  Search Vectors
                </Button>
                <Button
                  fullWidth
                  variant="outlined"
                  startIcon={<EditIcon />}
                  onClick={onEdit}
                  sx={{ justifyContent: 'flex-start', borderRadius: 2 }}
                >
                  Edit Collection
                </Button>
                <Divider sx={{ my: 1 }} />
                <Button
                  fullWidth
                  variant="outlined"
                  color="error"
                  startIcon={<DeleteIcon />}
                  onClick={onDelete}
                  sx={{ justifyContent: 'flex-start', borderRadius: 2 }}
                >
                  Delete Collection
                </Button>
              </Box>
            </CardContent>
          </Card>
        </Box>
      </Box>
    </Box>
  );
};

export default CollectionDetails;
