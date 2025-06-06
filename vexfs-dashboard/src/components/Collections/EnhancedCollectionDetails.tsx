import React, { useState, useEffect } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Button,
  Chip,
  Divider,
  IconButton,
  Alert,
  CircularProgress,
  Tabs,
  Tab,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
} from '@mui/material';
import {
  Edit as EditIcon,
  Delete as DeleteIcon,
  Add as AddIcon,
  Search as SearchIcon,
  ArrowBack as ArrowBackIcon,
  Storage as StorageIcon,
  Timeline as TimelineIcon,
  List as ListIcon,
} from '@mui/icons-material';
import { format } from 'date-fns';
import { vexfsApi } from '../../services/api';
import VectorsList from '../Vectors/VectorsList';
import VectorDetails from '../Vectors/VectorDetails';
import VectorSearch from '../Vectors/VectorSearch';
import AddVectorForm from '../Vectors/AddVectorForm';
import type {
  VexFSCollection,
  VexFSPoint,
  VexFSSearchResult,
  VectorListFilters,
  VectorSearchQuery,
} from '../../types';

interface EnhancedCollectionDetailsProps {
  collection: VexFSCollection;
  loading?: boolean;
  error?: string | null;
  onBack: () => void;
  onEdit: () => void;
  onDelete: () => void;
}

interface TabPanelProps {
  children?: React.ReactNode;
  index: number;
  value: number;
}

const TabPanel: React.FC<TabPanelProps> = ({ children, value, index }) => {
  return (
    <div hidden={value !== index}>
      {value === index && <Box sx={{ py: 3 }}>{children}</Box>}
    </div>
  );
};

const EnhancedCollectionDetails: React.FC<EnhancedCollectionDetailsProps> = ({
  collection,
  loading = false,
  error = null,
  onBack,
  onEdit,
  onDelete,
}) => {
  const [activeTab, setActiveTab] = useState(0);
  const [vectors, setVectors] = useState<VexFSPoint[]>([]);
  const [vectorsLoading, setVectorsLoading] = useState(false);
  const [vectorsError, setVectorsError] = useState<string | null>(null);
  const [selectedVector, setSelectedVector] = useState<VexFSPoint | null>(null);
  const [showVectorDetails, setShowVectorDetails] = useState(false);
  const [showAddVector, setShowAddVector] = useState(false);
  const [showVectorSearch, setShowVectorSearch] = useState(false);
  const [deleteVectorDialog, setDeleteVectorDialog] =
    useState<VexFSPoint | null>(null);

  // Pagination state
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState(25);
  const [totalCount, setTotalCount] = useState(0);
  const [filters, setFilters] = useState<VectorListFilters>({});

  const handleTabChange = (_event: React.SyntheticEvent, newValue: number) => {
    setActiveTab(newValue);
    if (newValue === 1 && vectors.length === 0) {
      loadVectors();
    }
  };

  const loadVectors = async () => {
    setVectorsLoading(true);
    setVectorsError(null);
    try {
      const response = await vexfsApi.getVectors(
        collection.name,
        pageSize,
        page * pageSize,
        filters
      );
      setVectors(response.items);
      setTotalCount(response.total);
    } catch (err) {
      setVectorsError(
        err instanceof Error ? err.message : 'Failed to load vectors'
      );
    } finally {
      setVectorsLoading(false);
    }
  };

  useEffect(() => {
    if (activeTab === 1) {
      loadVectors();
    }
  }, [page, pageSize, filters, activeTab]);

  const handleViewVector = (vector: VexFSPoint | VexFSSearchResult) => {
    setSelectedVector(vector as VexFSPoint);
    setShowVectorDetails(true);
  };

  const handleEditVector = async (vector: VexFSPoint) => {
    try {
      const success = await vexfsApi.updateVector(
        collection.name,
        vector.id,
        vector
      );
      if (success) {
        loadVectors();
        setShowVectorDetails(false);
      }
    } catch (err) {
      console.error('Failed to update vector:', err);
    }
  };

  const handleDeleteVector = (vector: VexFSPoint) => {
    setDeleteVectorDialog(vector);
  };

  const confirmDeleteVector = async () => {
    if (!deleteVectorDialog) return;

    try {
      const success = await vexfsApi.deleteVector(
        collection.name,
        deleteVectorDialog.id
      );
      if (success) {
        loadVectors();
        setDeleteVectorDialog(null);
        if (showVectorDetails && selectedVector?.id === deleteVectorDialog.id) {
          setShowVectorDetails(false);
        }
      }
    } catch (err) {
      console.error('Failed to delete vector:', err);
    }
  };

  const handleCopyVector = (vector: VexFSPoint) => {
    // Copy functionality handled in the component
    console.log('Vector copied:', vector.id);
  };

  const handleAddVectors = async (
    newVectors: VexFSPoint[]
  ): Promise<boolean> => {
    try {
      const success = await vexfsApi.batchAddVectors(
        collection.name,
        newVectors
      );
      if (success) {
        loadVectors();
        return true;
      }
      return false;
    } catch (err) {
      console.error('Failed to add vectors:', err);
      return false;
    }
  };

  const handleVectorSearch = async (
    query: VectorSearchQuery
  ): Promise<VexFSSearchResult[]> => {
    try {
      return await vexfsApi.searchVectors(collection.name, query);
    } catch (err) {
      console.error('Vector search failed:', err);
      return [];
    }
  };

  const handleExportVector = (vector: VexFSPoint) => {
    const dataStr = JSON.stringify(vector, null, 2);
    const dataBlob = new Blob([dataStr], { type: 'application/json' });
    const url = URL.createObjectURL(dataBlob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `vector_${vector.id}.json`;
    link.click();
    URL.revokeObjectURL(url);
  };

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
            onClick={() => setShowVectorSearch(true)}
            sx={{ borderRadius: 2 }}
          >
            Search Vectors
          </Button>
          <Button
            variant="outlined"
            startIcon={<AddIcon />}
            onClick={() => setShowAddVector(true)}
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

      {/* Tabs */}
      <Card>
        <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
          <Tabs value={activeTab} onChange={handleTabChange}>
            <Tab label="Overview" />
            <Tab label="Vectors" icon={<ListIcon />} />
          </Tabs>
        </Box>

        {/* Overview Tab */}
        <TabPanel value={activeTab} index={0}>
          <Box sx={{ display: 'flex', gap: 3 }}>
            {/* Main Content */}
            <Box sx={{ flex: 2 }}>
              {/* Basic Information */}
              <Card sx={{ mb: 3 }}>
                <CardContent>
                  <Typography
                    variant="h6"
                    gutterBottom
                    sx={{ fontWeight: 600 }}
                  >
                    Basic Information
                  </Typography>
                  <Box
                    sx={{
                      display: 'flex',
                      flexDirection: 'column',
                      gap: 2,
                    }}
                  >
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
                        sx={{
                          fontFamily: 'monospace',
                          color: 'text.secondary',
                        }}
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
                  <Typography
                    variant="h6"
                    gutterBottom
                    sx={{ fontWeight: 600 }}
                  >
                    Vector Configuration
                  </Typography>
                  <Box sx={{ display: 'flex', gap: 4 }}>
                    <Box sx={{ flex: 1 }}>
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
                    <Box sx={{ flex: 1 }}>
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
                </CardContent>
              </Card>

              {/* Recent Activity */}
              <Card>
                <CardContent>
                  <Typography
                    variant="h6"
                    gutterBottom
                    sx={{ fontWeight: 600 }}
                  >
                    Recent Activity
                  </Typography>
                  <Box
                    sx={{
                      display: 'flex',
                      flexDirection: 'column',
                      gap: 2,
                    }}
                  >
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
            <Box sx={{ flex: 1 }}>
              <Card sx={{ mb: 3 }}>
                <CardContent>
                  <Typography
                    variant="h6"
                    gutterBottom
                    sx={{ fontWeight: 600 }}
                  >
                    Statistics
                  </Typography>
                  <Box
                    sx={{
                      display: 'flex',
                      flexDirection: 'column',
                      gap: 3,
                    }}
                  >
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
                    <Box
                      sx={{
                        display: 'flex',
                        justifyContent: 'space-between',
                      }}
                    >
                      <Typography variant="body2" color="text.secondary">
                        Dimensions
                      </Typography>
                      <Typography variant="body2" sx={{ fontWeight: 500 }}>
                        {collection.vectorSize}
                      </Typography>
                    </Box>
                    <Box
                      sx={{
                        display: 'flex',
                        justifyContent: 'space-between',
                      }}
                    >
                      <Typography variant="body2" color="text.secondary">
                        Distance Metric
                      </Typography>
                      <Chip
                        label={collection.distance}
                        size="small"
                        color={
                          getDistanceMetricColor(collection.distance) as any
                        }
                        variant="outlined"
                      />
                    </Box>
                    <Box
                      sx={{
                        display: 'flex',
                        justifyContent: 'space-between',
                      }}
                    >
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
                  <Typography
                    variant="h6"
                    gutterBottom
                    sx={{ fontWeight: 600 }}
                  >
                    Quick Actions
                  </Typography>
                  <Box
                    sx={{
                      display: 'flex',
                      flexDirection: 'column',
                      gap: 1,
                    }}
                  >
                    <Button
                      fullWidth
                      variant="outlined"
                      startIcon={<AddIcon />}
                      onClick={() => setShowAddVector(true)}
                      sx={{ justifyContent: 'flex-start', borderRadius: 2 }}
                    >
                      Add Vectors
                    </Button>
                    <Button
                      fullWidth
                      variant="outlined"
                      startIcon={<SearchIcon />}
                      onClick={() => setShowVectorSearch(true)}
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
        </TabPanel>

        {/* Vectors Tab */}
        <TabPanel value={activeTab} index={1}>
          <VectorsList
            collectionName={collection.name}
            vectors={vectors}
            loading={vectorsLoading}
            error={vectorsError}
            totalCount={totalCount}
            page={page}
            pageSize={pageSize}
            onPageChange={setPage}
            onPageSizeChange={setPageSize}
            onFiltersChange={setFilters}
            onViewVector={handleViewVector}
            onEditVector={handleEditVector}
            onDeleteVector={handleDeleteVector}
            onCopyVector={handleCopyVector}
            onAddVector={() => setShowAddVector(true)}
          />
        </TabPanel>
      </Card>

      {/* Vector Details Dialog */}
      {showVectorDetails && selectedVector && (
        <Dialog
          open={showVectorDetails}
          onClose={() => setShowVectorDetails(false)}
          maxWidth="lg"
          fullWidth
        >
          <VectorDetails
            vector={selectedVector}
            collectionName={collection.name}
            onBack={() => setShowVectorDetails(false)}
            onEdit={handleEditVector}
            onDelete={handleDeleteVector}
            onFindSimilar={vector => {
              setSelectedVector(vector);
              setShowVectorDetails(false);
              setShowVectorSearch(true);
            }}
            onExport={handleExportVector}
          />
        </Dialog>
      )}

      {/* Add Vector Dialog */}
      <AddVectorForm
        open={showAddVector}
        collectionName={collection.name}
        vectorSize={collection.vectorSize}
        onClose={() => setShowAddVector(false)}
        onSubmit={handleAddVectors}
      />

      {/* Vector Search Dialog */}
      {showVectorSearch && (
        <Dialog
          open={showVectorSearch}
          onClose={() => setShowVectorSearch(false)}
          maxWidth="lg"
          fullWidth
        >
          <DialogTitle>Vector Search</DialogTitle>
          <DialogContent>
            <VectorSearch
              collectionName={collection.name}
              vectors={vectors}
              onSearch={handleVectorSearch}
              onViewVector={handleViewVector}
            />
          </DialogContent>
          <DialogActions>
            <Button onClick={() => setShowVectorSearch(false)}>Close</Button>
          </DialogActions>
        </Dialog>
      )}

      {/* Delete Vector Confirmation Dialog */}
      <Dialog
        open={!!deleteVectorDialog}
        onClose={() => setDeleteVectorDialog(null)}
      >
        <DialogTitle>Delete Vector</DialogTitle>
        <DialogContent>
          <Typography>
            Are you sure you want to delete vector "{deleteVectorDialog?.id}"?
            This action cannot be undone.
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDeleteVectorDialog(null)}>Cancel</Button>
          <Button
            onClick={confirmDeleteVector}
            color="error"
            variant="contained"
          >
            Delete
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
};

export default EnhancedCollectionDetails;
