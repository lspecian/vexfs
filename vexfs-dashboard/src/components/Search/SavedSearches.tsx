import React, { useState, useEffect } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Button,
  IconButton,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  List,
  ListItem,
  ListItemText,
  ListItemSecondaryAction,
  Chip,
  Alert,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Autocomplete,
  Divider,
  Menu,
  ListItemIcon,
} from '@mui/material';
import {
  Save as SaveIcon,
  Delete as DeleteIcon,
  Edit as EditIcon,
  PlayArrow as PlayIcon,
  Share as ShareIcon,
  Folder as FolderIcon,
  Search as SearchIcon,
  MoreVert as MoreVertIcon,
  Public as PublicIcon,
  Lock as PrivateIcon,
} from '@mui/icons-material';
import { formatDistanceToNow } from 'date-fns';
import type { SavedSearch, AdvancedSearchQuery } from '../../types';

interface SavedSearchesProps {
  savedSearches: SavedSearch[];
  onExecuteSearch: (query: AdvancedSearchQuery) => void;
  onSaveSearch: (
    name: string,
    query: AdvancedSearchQuery,
    description?: string,
    tags?: string[],
    category?: string,
    isPublic?: boolean
  ) => Promise<void>;
  onDeleteSearch: (searchId: string) => Promise<void>;
  onUpdateSearch: (
    searchId: string,
    updates: Partial<SavedSearch>
  ) => Promise<void>;
  currentQuery?: AdvancedSearchQuery;
  loading?: boolean;
}

const SavedSearches: React.FC<SavedSearchesProps> = ({
  savedSearches,
  onExecuteSearch,
  onSaveSearch,
  onDeleteSearch,
  onUpdateSearch,
  currentQuery,
  loading = false,
}) => {
  const [saveDialogOpen, setSaveDialogOpen] = useState(false);
  const [editDialogOpen, setEditDialogOpen] = useState(false);
  const [selectedSearch, setSelectedSearch] = useState<SavedSearch | null>(
    null
  );
  const [filterCategory, setFilterCategory] = useState<string>('all');
  const [searchFilter, setSearchFilter] = useState<string>('');
  const [menuAnchor, setMenuAnchor] = useState<null | HTMLElement>(null);
  const [error, setError] = useState<string | null>(null);

  const [saveForm, setSaveForm] = useState({
    name: '',
    description: '',
    tags: [] as string[],
    category: '',
    isPublic: false,
  });

  useEffect(() => {
    if (error) {
      const timer = setTimeout(() => setError(null), 5000);
      return () => clearTimeout(timer);
    }
  }, [error]);

  const categories = Array.from(
    new Set(savedSearches.map(s => s.category).filter(Boolean))
  );

  const filteredSearches = savedSearches.filter(search => {
    const matchesCategory =
      filterCategory === 'all' || search.category === filterCategory;
    const matchesSearch =
      search.name.toLowerCase().includes(searchFilter.toLowerCase()) ||
      search.description?.toLowerCase().includes(searchFilter.toLowerCase()) ||
      search.tags.some(tag =>
        tag.toLowerCase().includes(searchFilter.toLowerCase())
      );

    return matchesCategory && matchesSearch;
  });

  const handleSaveSearch = async () => {
    if (!currentQuery) {
      setError('No current search query to save');
      return;
    }

    if (!saveForm.name.trim()) {
      setError('Search name is required');
      return;
    }

    try {
      await onSaveSearch(
        saveForm.name,
        currentQuery,
        saveForm.description,
        saveForm.tags,
        saveForm.category,
        saveForm.isPublic
      );
      setSaveDialogOpen(false);
      setSaveForm({
        name: '',
        description: '',
        tags: [],
        category: '',
        isPublic: false,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save search');
    }
  };

  const handleUpdateSearch = async () => {
    if (!selectedSearch) return;

    try {
      await onUpdateSearch(selectedSearch.id, {
        name: saveForm.name,
        description: saveForm.description,
        tags: saveForm.tags,
        category: saveForm.category,
        isPublic: saveForm.isPublic,
      });
      setEditDialogOpen(false);
      setSelectedSearch(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update search');
    }
  };

  const handleDeleteSearch = async (searchId: string) => {
    try {
      await onDeleteSearch(searchId);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete search');
    }
  };

  const openEditDialog = (search: SavedSearch) => {
    setSelectedSearch(search);
    setSaveForm({
      name: search.name,
      description: search.description || '',
      tags: search.tags,
      category: search.category || '',
      isPublic: search.isPublic,
    });
    setEditDialogOpen(true);
  };

  const getSearchTypeLabel = (query: AdvancedSearchQuery) => {
    switch (query.type) {
      case 'vector':
        return 'Vector Similarity';
      case 'metadata':
        return 'Metadata Filter';
      case 'hybrid':
        return 'Hybrid Search';
      default:
        return 'Unknown';
    }
  };

  const getSearchTypeColor = (query: AdvancedSearchQuery) => {
    switch (query.type) {
      case 'vector':
        return 'primary';
      case 'metadata':
        return 'secondary';
      case 'hybrid':
        return 'success';
      default:
        return 'default';
    }
  };

  return (
    <Card>
      <CardContent>
        <Box sx={{ display: 'flex', alignItems: 'center', mb: 3 }}>
          <FolderIcon sx={{ mr: 1 }} />
          <Typography variant="h6" sx={{ flexGrow: 1, fontWeight: 600 }}>
            Saved Searches
          </Typography>
          <Button
            variant="contained"
            startIcon={<SaveIcon />}
            onClick={() => setSaveDialogOpen(true)}
            disabled={!currentQuery}
            size="small"
          >
            Save Current
          </Button>
        </Box>

        {error && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {error}
          </Alert>
        )}

        {/* Filters */}
        <Box sx={{ display: 'flex', gap: 2, mb: 3 }}>
          <TextField
            size="small"
            placeholder="Search saved searches..."
            value={searchFilter}
            onChange={e => setSearchFilter(e.target.value)}
            sx={{ flexGrow: 1 }}
            InputProps={{
              startAdornment: (
                <SearchIcon sx={{ mr: 1, color: 'text.secondary' }} />
              ),
            }}
          />
          <FormControl size="small" sx={{ minWidth: 150 }}>
            <InputLabel>Category</InputLabel>
            <Select
              value={filterCategory}
              onChange={e => setFilterCategory(e.target.value)}
              label="Category"
            >
              <MenuItem value="all">All Categories</MenuItem>
              {categories.map(category => (
                <MenuItem key={category} value={category}>
                  {category}
                </MenuItem>
              ))}
            </Select>
          </FormControl>
        </Box>

        {/* Saved Searches List */}
        {filteredSearches.length === 0 ? (
          <Alert severity="info">
            {savedSearches.length === 0
              ? 'No saved searches yet. Save your current search to get started.'
              : 'No searches match your filters.'}
          </Alert>
        ) : (
          <List>
            {filteredSearches.map((search, index) => (
              <React.Fragment key={search.id}>
                <ListItem>
                  <ListItemText
                    primary={
                      <Box
                        sx={{ display: 'flex', alignItems: 'center', gap: 1 }}
                      >
                        <Typography
                          variant="subtitle1"
                          sx={{ fontWeight: 600 }}
                        >
                          {search.name}
                        </Typography>
                        <Chip
                          label={getSearchTypeLabel(search.query)}
                          color={getSearchTypeColor(search.query)}
                          size="small"
                        />
                        {search.isPublic ? (
                          <PublicIcon fontSize="small" color="action" />
                        ) : (
                          <PrivateIcon fontSize="small" color="action" />
                        )}
                      </Box>
                    }
                    secondary={
                      <Box>
                        {search.description && (
                          <Typography variant="body2" color="text.secondary">
                            {search.description}
                          </Typography>
                        )}
                        <Box
                          sx={{
                            display: 'flex',
                            alignItems: 'center',
                            gap: 1,
                            mt: 1,
                          }}
                        >
                          {search.tags.map(tag => (
                            <Chip
                              key={tag}
                              label={tag}
                              size="small"
                              variant="outlined"
                            />
                          ))}
                          <Typography variant="caption" color="text.secondary">
                            {formatDistanceToNow(new Date(search.updatedAt), {
                              addSuffix: true,
                            })}
                          </Typography>
                        </Box>
                      </Box>
                    }
                  />
                  <ListItemSecondaryAction>
                    <Box sx={{ display: 'flex', gap: 0.5 }}>
                      <IconButton
                        size="small"
                        onClick={() => onExecuteSearch(search.query)}
                        disabled={loading}
                      >
                        <PlayIcon />
                      </IconButton>
                      <IconButton
                        size="small"
                        onClick={e => {
                          setSelectedSearch(search);
                          setMenuAnchor(e.currentTarget);
                        }}
                      >
                        <MoreVertIcon />
                      </IconButton>
                    </Box>
                  </ListItemSecondaryAction>
                </ListItem>
                {index < filteredSearches.length - 1 && <Divider />}
              </React.Fragment>
            ))}
          </List>
        )}

        {/* Context Menu */}
        <Menu
          anchorEl={menuAnchor}
          open={Boolean(menuAnchor)}
          onClose={() => setMenuAnchor(null)}
        >
          <MenuItem
            onClick={() => {
              if (selectedSearch) openEditDialog(selectedSearch);
              setMenuAnchor(null);
            }}
          >
            <ListItemIcon>
              <EditIcon fontSize="small" />
            </ListItemIcon>
            Edit
          </MenuItem>
          <MenuItem
            onClick={() => {
              if (selectedSearch) {
                navigator.clipboard.writeText(
                  JSON.stringify(selectedSearch.query, null, 2)
                );
              }
              setMenuAnchor(null);
            }}
          >
            <ListItemIcon>
              <ShareIcon fontSize="small" />
            </ListItemIcon>
            Copy Query
          </MenuItem>
          <MenuItem
            onClick={() => {
              if (selectedSearch) handleDeleteSearch(selectedSearch.id);
              setMenuAnchor(null);
            }}
            sx={{ color: 'error.main' }}
          >
            <ListItemIcon>
              <DeleteIcon fontSize="small" color="error" />
            </ListItemIcon>
            Delete
          </MenuItem>
        </Menu>

        {/* Save Search Dialog */}
        <Dialog
          open={saveDialogOpen}
          onClose={() => setSaveDialogOpen(false)}
          maxWidth="sm"
          fullWidth
        >
          <DialogTitle>Save Search</DialogTitle>
          <DialogContent>
            <Box
              sx={{ display: 'flex', flexDirection: 'column', gap: 2, pt: 1 }}
            >
              <TextField
                label="Search Name"
                value={saveForm.name}
                onChange={e =>
                  setSaveForm({ ...saveForm, name: e.target.value })
                }
                fullWidth
                required
              />
              <TextField
                label="Description"
                value={saveForm.description}
                onChange={e =>
                  setSaveForm({ ...saveForm, description: e.target.value })
                }
                fullWidth
                multiline
                rows={2}
              />
              <Autocomplete
                multiple
                freeSolo
                options={[]}
                value={saveForm.tags}
                onChange={(_, value) =>
                  setSaveForm({ ...saveForm, tags: value })
                }
                renderInput={params => (
                  <TextField
                    {...params}
                    label="Tags"
                    placeholder="Add tags..."
                  />
                )}
              />
              <TextField
                label="Category"
                value={saveForm.category}
                onChange={e =>
                  setSaveForm({ ...saveForm, category: e.target.value })
                }
                fullWidth
              />
              <FormControl>
                <InputLabel>Visibility</InputLabel>
                <Select
                  value={saveForm.isPublic ? 'public' : 'private'}
                  onChange={e =>
                    setSaveForm({
                      ...saveForm,
                      isPublic: e.target.value === 'public',
                    })
                  }
                  label="Visibility"
                >
                  <MenuItem value="private">Private</MenuItem>
                  <MenuItem value="public">Public</MenuItem>
                </Select>
              </FormControl>
            </Box>
          </DialogContent>
          <DialogActions>
            <Button onClick={() => setSaveDialogOpen(false)}>Cancel</Button>
            <Button onClick={handleSaveSearch} variant="contained">
              Save Search
            </Button>
          </DialogActions>
        </Dialog>

        {/* Edit Search Dialog */}
        <Dialog
          open={editDialogOpen}
          onClose={() => setEditDialogOpen(false)}
          maxWidth="sm"
          fullWidth
        >
          <DialogTitle>Edit Search</DialogTitle>
          <DialogContent>
            <Box
              sx={{ display: 'flex', flexDirection: 'column', gap: 2, pt: 1 }}
            >
              <TextField
                label="Search Name"
                value={saveForm.name}
                onChange={e =>
                  setSaveForm({ ...saveForm, name: e.target.value })
                }
                fullWidth
                required
              />
              <TextField
                label="Description"
                value={saveForm.description}
                onChange={e =>
                  setSaveForm({ ...saveForm, description: e.target.value })
                }
                fullWidth
                multiline
                rows={2}
              />
              <Autocomplete
                multiple
                freeSolo
                options={[]}
                value={saveForm.tags}
                onChange={(_, value) =>
                  setSaveForm({ ...saveForm, tags: value })
                }
                renderInput={params => (
                  <TextField
                    {...params}
                    label="Tags"
                    placeholder="Add tags..."
                  />
                )}
              />
              <TextField
                label="Category"
                value={saveForm.category}
                onChange={e =>
                  setSaveForm({ ...saveForm, category: e.target.value })
                }
                fullWidth
              />
              <FormControl>
                <InputLabel>Visibility</InputLabel>
                <Select
                  value={saveForm.isPublic ? 'public' : 'private'}
                  onChange={e =>
                    setSaveForm({
                      ...saveForm,
                      isPublic: e.target.value === 'public',
                    })
                  }
                  label="Visibility"
                >
                  <MenuItem value="private">Private</MenuItem>
                  <MenuItem value="public">Public</MenuItem>
                </Select>
              </FormControl>
            </Box>
          </DialogContent>
          <DialogActions>
            <Button onClick={() => setEditDialogOpen(false)}>Cancel</Button>
            <Button onClick={handleUpdateSearch} variant="contained">
              Update Search
            </Button>
          </DialogActions>
        </Dialog>
      </CardContent>
    </Card>
  );
};

export default SavedSearches;
