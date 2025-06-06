import React, { useState } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Button,
  IconButton,
  Divider,
  Chip,
  Alert,
  CircularProgress,
  Breadcrumbs,
  Link,
  Tabs,
  Tab,
  TextField,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
} from '@mui/material';
import {
  ArrowBack as ArrowBackIcon,
  Edit as EditIcon,
  Delete as DeleteIcon,
  Search as SearchIcon,
  Download as DownloadIcon,
  ContentCopy as CopyIcon,
  Save as SaveIcon,
  Cancel as CancelIcon,
} from '@mui/icons-material';
import { formatNumber } from '../../utils';
import VectorVisualization from './VectorVisualization';
import type { VexFSPoint } from '../../types';

interface VectorDetailsProps {
  vector: VexFSPoint;
  collectionName: string;
  loading?: boolean;
  error?: string | null;
  onBack: () => void;
  onEdit: (vector: VexFSPoint) => void;
  onDelete: (vector: VexFSPoint) => void;
  onFindSimilar: (vector: VexFSPoint) => void;
  onExport: (vector: VexFSPoint) => void;
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

const VectorDetails: React.FC<VectorDetailsProps> = ({
  vector,
  collectionName,
  loading = false,
  error = null,
  onBack,
  onEdit,
  onDelete,
  onFindSimilar,
  onExport,
}) => {
  const [activeTab, setActiveTab] = useState(0);
  const [editingMetadata, setEditingMetadata] = useState(false);
  const [metadataValue, setMetadataValue] = useState(
    JSON.stringify(vector.payload || {}, null, 2)
  );
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);

  const handleTabChange = (_event: React.SyntheticEvent, newValue: number) => {
    setActiveTab(newValue);
  };

  const handleSaveMetadata = () => {
    try {
      const parsedMetadata = JSON.parse(metadataValue);
      const updatedVector = { ...vector, payload: parsedMetadata };
      onEdit(updatedVector);
      setEditingMetadata(false);
    } catch (err) {
      console.error('Invalid JSON:', err);
    }
  };

  const handleCancelEdit = () => {
    setMetadataValue(JSON.stringify(vector.payload || {}, null, 2));
    setEditingMetadata(false);
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch (err) {
      console.error('Failed to copy to clipboard:', err);
    }
  };

  const handleCopyVector = () => {
    const vectorData = JSON.stringify(vector, null, 2);
    copyToClipboard(vectorData);
  };

  const handleCopyVectorData = () => {
    const vectorDataStr = JSON.stringify(vector.vector);
    copyToClipboard(vectorDataStr);
  };

  const handleDeleteConfirm = () => {
    onDelete(vector);
    setDeleteDialogOpen(false);
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
        Failed to load vector details: {error}
      </Alert>
    );
  }

  return (
    <Box>
      {/* Breadcrumb Navigation */}
      <Breadcrumbs sx={{ mb: 2 }}>
        <Link
          component="button"
          variant="body2"
          onClick={onBack}
          sx={{ textDecoration: 'none' }}
        >
          {collectionName}
        </Link>
        <Typography variant="body2" color="text.primary">
          Vector {String(vector.id).substring(0, 20)}
        </Typography>
      </Breadcrumbs>

      {/* Header */}
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
            Vector Details
          </Typography>
          <Typography variant="body2" color="text.secondary">
            ID: {vector.id}
          </Typography>
        </Box>
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button
            variant="outlined"
            startIcon={<SearchIcon />}
            onClick={() => onFindSimilar(vector)}
            sx={{ borderRadius: 2 }}
          >
            Find Similar
          </Button>
          <Button
            variant="outlined"
            startIcon={<DownloadIcon />}
            onClick={() => onExport(vector)}
            sx={{ borderRadius: 2 }}
          >
            Export
          </Button>
          <Button
            variant="outlined"
            startIcon={<EditIcon />}
            onClick={() => setEditingMetadata(true)}
            sx={{ borderRadius: 2 }}
          >
            Edit
          </Button>
          <Button
            variant="outlined"
            color="error"
            startIcon={<DeleteIcon />}
            onClick={() => setDeleteDialogOpen(true)}
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
            <Tab label="Vector Data" />
            <Tab label="Metadata" />
            <Tab label="Visualization" />
          </Tabs>
        </Box>

        {/* Overview Tab */}
        <TabPanel value={activeTab} index={0}>
          <CardContent>
            <Typography variant="h6" gutterBottom sx={{ fontWeight: 600 }}>
              Vector Information
            </Typography>
            <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
              <Box>
                <Typography variant="subtitle2" color="text.secondary">
                  Vector ID
                </Typography>
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                  <Typography
                    variant="body1"
                    sx={{ fontFamily: 'monospace', fontWeight: 500 }}
                  >
                    {vector.id}
                  </Typography>
                  <IconButton
                    size="small"
                    onClick={() => copyToClipboard(String(vector.id))}
                  >
                    <CopyIcon fontSize="small" />
                  </IconButton>
                </Box>
              </Box>
              <Box>
                <Typography variant="subtitle2" color="text.secondary">
                  Dimensions
                </Typography>
                <Chip
                  label={formatNumber(vector.vector.length)}
                  color="primary"
                  variant="outlined"
                />
              </Box>
              <Box>
                <Typography variant="subtitle2" color="text.secondary">
                  Metadata Fields
                </Typography>
                <Typography variant="body1">
                  {vector.payload ? Object.keys(vector.payload).length : 0}{' '}
                  fields
                </Typography>
              </Box>
            </Box>
          </CardContent>
        </TabPanel>

        {/* Vector Data Tab */}
        <TabPanel value={activeTab} index={1}>
          <CardContent>
            <Box
              sx={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                mb: 2,
              }}
            >
              <Typography variant="h6" sx={{ fontWeight: 600 }}>
                Vector Data ({formatNumber(vector.vector.length)} dimensions)
              </Typography>
              <Button
                size="small"
                startIcon={<CopyIcon />}
                onClick={handleCopyVectorData}
                variant="outlined"
              >
                Copy Data
              </Button>
            </Box>
            <Box
              sx={{
                maxHeight: 400,
                overflow: 'auto',
                bgcolor: 'grey.50',
                p: 2,
                borderRadius: 1,
                fontFamily: 'monospace',
                fontSize: '0.875rem',
              }}
            >
              <pre>{JSON.stringify(vector.vector, null, 2)}</pre>
            </Box>
          </CardContent>
        </TabPanel>

        {/* Metadata Tab */}
        <TabPanel value={activeTab} index={2}>
          <CardContent>
            <Box
              sx={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                mb: 2,
              }}
            >
              <Typography variant="h6" sx={{ fontWeight: 600 }}>
                Metadata
              </Typography>
              <Box sx={{ display: 'flex', gap: 1 }}>
                {editingMetadata ? (
                  <>
                    <Button
                      size="small"
                      startIcon={<SaveIcon />}
                      onClick={handleSaveMetadata}
                      variant="contained"
                    >
                      Save
                    </Button>
                    <Button
                      size="small"
                      startIcon={<CancelIcon />}
                      onClick={handleCancelEdit}
                      variant="outlined"
                    >
                      Cancel
                    </Button>
                  </>
                ) : (
                  <Button
                    size="small"
                    startIcon={<EditIcon />}
                    onClick={() => setEditingMetadata(true)}
                    variant="outlined"
                  >
                    Edit
                  </Button>
                )}
              </Box>
            </Box>
            {editingMetadata ? (
              <TextField
                fullWidth
                multiline
                rows={12}
                value={metadataValue}
                onChange={e => setMetadataValue(e.target.value)}
                variant="outlined"
                sx={{ fontFamily: 'monospace' }}
              />
            ) : (
              <Box
                sx={{
                  maxHeight: 400,
                  overflow: 'auto',
                  bgcolor: 'grey.50',
                  p: 2,
                  borderRadius: 1,
                  fontFamily: 'monospace',
                  fontSize: '0.875rem',
                }}
              >
                <pre>
                  {JSON.stringify(vector.payload || {}, null, 2) || '{}'}
                </pre>
              </Box>
            )}
          </CardContent>
        </TabPanel>

        {/* Visualization Tab */}
        <TabPanel value={activeTab} index={3}>
          <CardContent>
            <VectorVisualization
              vector={vector.vector}
              vectorId={vector.id}
              title={`Vector ${vector.id} Visualization`}
            />
          </CardContent>
        </TabPanel>
      </Card>

      {/* Delete Confirmation Dialog */}
      <Dialog
        open={deleteDialogOpen}
        onClose={() => setDeleteDialogOpen(false)}
      >
        <DialogTitle>Delete Vector</DialogTitle>
        <DialogContent>
          <Typography>
            Are you sure you want to delete vector "{vector.id}"? This action
            cannot be undone.
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDeleteDialogOpen(false)}>Cancel</Button>
          <Button
            onClick={handleDeleteConfirm}
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

export default VectorDetails;
