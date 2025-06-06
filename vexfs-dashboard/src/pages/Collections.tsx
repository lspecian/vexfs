import React, { useState, useCallback, useMemo } from 'react';
import { Box, Typography, Button, Alert, Snackbar } from '@mui/material';
import { Add as AddIcon } from '@mui/icons-material';
import { useCollections } from '../hooks/useVexFS';
import { ErrorBoundary } from '../components/Common/ErrorBoundary';
import CollectionsList from '../components/Collections/CollectionsList';
import CreateCollectionForm from '../components/Collections/CreateCollectionForm';
import EditCollectionForm from '../components/Collections/EditCollectionForm';
import CollectionDetails from '../components/Collections/CollectionDetails';
import DeleteCollectionDialog from '../components/Collections/DeleteCollectionDialog';
import type { VexFSCollection } from '../types';

type ViewMode = 'list' | 'details' | 'create' | 'edit';

interface CreateCollectionData {
  name: string;
  description?: string;
  vectorSize: number;
  distance: 'cosine' | 'euclidean' | 'dot';
}

interface EditCollectionData {
  name: string;
  description?: string;
  vectorSize: number;
  distance: 'cosine' | 'euclidean' | 'dot';
}

// Memoized header component to prevent unnecessary re-renders
const CollectionsHeader = React.memo<{
  onCreateClick: () => void;
}>(({ onCreateClick }) => (
  <Box
    sx={{
      display: 'flex',
      justifyContent: 'space-between',
      alignItems: 'center',
      mb: 3,
    }}
  >
    <Typography variant="h4" component="h1" sx={{ fontWeight: 600 }}>
      Collections
    </Typography>
    <Button
      variant="contained"
      startIcon={<AddIcon />}
      onClick={onCreateClick}
      sx={{ borderRadius: 2 }}
    >
      Create Collection
    </Button>
  </Box>
));

CollectionsHeader.displayName = 'CollectionsHeader';

// Memoized error display component
const ErrorDisplay = React.memo<{ error: string }>(({ error }) => (
  <Alert severity="error" sx={{ mb: 3 }}>
    Failed to load collections: {error}
  </Alert>
));

ErrorDisplay.displayName = 'ErrorDisplay';

// Memoized success snackbar component
const SuccessSnackbar = React.memo<{
  message: string | null;
  onClose: () => void;
}>(({ message, onClose }) => (
  <Snackbar
    open={!!message}
    autoHideDuration={6000}
    onClose={onClose}
    anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
  >
    <Alert onClose={onClose} severity="success" sx={{ width: '100%' }}>
      {message}
    </Alert>
  </Snackbar>
));

SuccessSnackbar.displayName = 'SuccessSnackbar';

const Collections: React.FC = () => {
  const {
    collections,
    loading,
    error,
    createCollection,
    updateCollection,
    deleteCollection,
  } = useCollections();

  const [viewMode, setViewMode] = useState<ViewMode>('list');
  const [selectedCollection, setSelectedCollection] =
    useState<VexFSCollection | null>(null);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [showEditForm, setShowEditForm] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [operationLoading, setOperationLoading] = useState(false);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  // Memoized navigation handlers to prevent unnecessary re-renders
  const handleViewDetails = useCallback((collection: VexFSCollection) => {
    setSelectedCollection(collection);
    setViewMode('details');
  }, []);

  const handleBackToList = useCallback(() => {
    setViewMode('list');
    setSelectedCollection(null);
  }, []);

  const handleCreateClick = useCallback(() => {
    setShowCreateForm(true);
  }, []);

  const handleEditClick = useCallback((collection?: VexFSCollection) => {
    if (collection) {
      setSelectedCollection(collection);
    }
    setShowEditForm(true);
  }, []);

  const handleDeleteClick = useCallback((collection: VexFSCollection) => {
    setSelectedCollection(collection);
    setShowDeleteDialog(true);
  }, []);

  // Memoized form handlers
  const handleCreateSubmit = useCallback(
    async (data: CreateCollectionData) => {
      setOperationLoading(true);
      setOperationError(null);
      try {
        await createCollection(data.name, data.vectorSize, data.distance);
        setSuccessMessage(`Collection "${data.name}" created successfully`);
        setShowCreateForm(false);
      } catch (err) {
        setOperationError(
          err instanceof Error ? err.message : 'Failed to create collection'
        );
        throw err;
      } finally {
        setOperationLoading(false);
      }
    },
    [createCollection]
  );

  const handleEditSubmit = useCallback(
    async (data: EditCollectionData) => {
      if (!selectedCollection) return;

      setOperationLoading(true);
      setOperationError(null);
      try {
        const updatedCollection = await updateCollection(
          selectedCollection.name,
          {
            description: data.description,
          }
        );
        setSelectedCollection(updatedCollection);
        setSuccessMessage(`Collection "${data.name}" updated successfully`);
        setShowEditForm(false);
      } catch (err) {
        setOperationError(
          err instanceof Error ? err.message : 'Failed to update collection'
        );
        throw err;
      } finally {
        setOperationLoading(false);
      }
    },
    [selectedCollection, updateCollection]
  );

  const handleDeleteConfirm = useCallback(async () => {
    if (!selectedCollection) return;

    setOperationLoading(true);
    setOperationError(null);
    try {
      await deleteCollection(selectedCollection.name);
      setSuccessMessage(
        `Collection "${selectedCollection.name}" deleted successfully`
      );
      setShowDeleteDialog(false);
      setSelectedCollection(null);
      if (viewMode === 'details') {
        setViewMode('list');
      }
    } catch (err) {
      setOperationError(
        err instanceof Error ? err.message : 'Failed to delete collection'
      );
      throw err;
    } finally {
      setOperationLoading(false);
    }
  }, [selectedCollection, deleteCollection, viewMode]);

  // Memoized close handlers
  const handleCloseCreateForm = useCallback(() => {
    setShowCreateForm(false);
    setOperationError(null);
  }, []);

  const handleCloseEditForm = useCallback(() => {
    setShowEditForm(false);
    setOperationError(null);
  }, []);

  const handleCloseDeleteDialog = useCallback(() => {
    setShowDeleteDialog(false);
    setOperationError(null);
  }, []);

  const handleCloseSnackbar = useCallback(() => {
    setSuccessMessage(null);
  }, []);

  // Memoized placeholder handlers for future features
  const handleAddVectors = useCallback(() => {
    // TODO: Implement add vectors functionality
    console.log('Add vectors to collection:', selectedCollection?.name);
  }, [selectedCollection?.name]);

  const handleSearchVectors = useCallback(() => {
    // TODO: Implement search vectors functionality
    console.log('Search vectors in collection:', selectedCollection?.name);
  }, [selectedCollection?.name]);

  // Memoize the current view content to prevent unnecessary re-renders
  const currentViewContent = useMemo(() => {
    if (viewMode === 'list') {
      return (
        <ErrorBoundary>
          <CollectionsList
            collections={collections}
            loading={loading}
            error={error}
            onViewDetails={handleViewDetails}
            onEdit={handleEditClick}
            onDelete={handleDeleteClick}
          />
        </ErrorBoundary>
      );
    }

    if (viewMode === 'details' && selectedCollection) {
      return (
        <ErrorBoundary>
          <CollectionDetails
            collection={selectedCollection}
            loading={loading}
            error={error}
            onBack={handleBackToList}
            onEdit={() => handleEditClick(selectedCollection)}
            onDelete={() => handleDeleteClick(selectedCollection)}
            onAddVectors={handleAddVectors}
            onSearchVectors={handleSearchVectors}
          />
        </ErrorBoundary>
      );
    }

    return null;
  }, [
    viewMode,
    collections,
    loading,
    error,
    selectedCollection,
    handleViewDetails,
    handleEditClick,
    handleDeleteClick,
    handleBackToList,
    handleAddVectors,
    handleSearchVectors,
  ]);

  return (
    <ErrorBoundary>
      <Box>
        {/* Header */}
        {viewMode === 'list' && (
          <CollectionsHeader onCreateClick={handleCreateClick} />
        )}

        {/* Error Display */}
        {error && <ErrorDisplay error={error} />}

        {/* Main Content */}
        {currentViewContent}

        {/* Create Collection Form */}
        <ErrorBoundary>
          <CreateCollectionForm
            open={showCreateForm}
            onClose={handleCloseCreateForm}
            onSubmit={handleCreateSubmit}
            loading={operationLoading}
            error={operationError}
          />
        </ErrorBoundary>

        {/* Edit Collection Form */}
        <ErrorBoundary>
          <EditCollectionForm
            open={showEditForm}
            collection={selectedCollection}
            onClose={handleCloseEditForm}
            onSubmit={handleEditSubmit}
            loading={operationLoading}
            error={operationError}
          />
        </ErrorBoundary>

        {/* Delete Confirmation Dialog */}
        <ErrorBoundary>
          <DeleteCollectionDialog
            open={showDeleteDialog}
            collection={selectedCollection}
            onClose={handleCloseDeleteDialog}
            onConfirm={handleDeleteConfirm}
            loading={operationLoading}
            error={operationError}
          />
        </ErrorBoundary>

        {/* Success Snackbar */}
        <SuccessSnackbar
          message={successMessage}
          onClose={handleCloseSnackbar}
        />
      </Box>
    </ErrorBoundary>
  );
};

export default React.memo(Collections);
