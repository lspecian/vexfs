import React, { useState, useCallback } from 'react';
import {
  Box,
  Paper,
  Typography,
  List,
  ListItem,
  ListItemText,
  ListItemIcon,
  ListItemButton,
  ListItemSecondaryAction,
  IconButton,
  Chip,
  Tabs,
  Tab,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  Button,
  Divider,
  Alert,
  Tooltip,
  useTheme,
} from '@mui/material';
import {
  History as HistoryIcon,
  Bookmark as BookmarkIcon,
  Delete as DeleteIcon,
  Save as SaveIcon,
  Psychology as PsychologyIcon,
  Hub as HubIcon,
  AutoAwesome as AutoAwesomeIcon,
  FilterList as FilterIcon,
  AccessTime as AccessTimeIcon,
  TrendingUp as TrendingUpIcon,
} from '@mui/icons-material';

import type {
  SearchHistoryEntry,
  SavedSemanticSearch,
  SearchHistoryProps,
  SemanticSearchMode,
} from '../../types/semantic';

interface TabPanelProps {
  children?: React.ReactNode;
  index: number;
  value: number;
}

const TabPanel: React.FC<TabPanelProps> = ({ children, value, index, ...other }) => (
  <div
    role="tabpanel"
    hidden={value !== index}
    id={`search-history-tabpanel-${index}`}
    aria-labelledby={`search-history-tab-${index}`}
    {...other}
  >
    {value === index && <Box>{children}</Box>}
  </div>
);

const SearchHistory: React.FC<SearchHistoryProps> = ({
  history,
  savedSearches,
  onHistorySelect,
  onSavedSearchSelect,
  onSaveSearch,
  onDeleteSavedSearch,
  className,
}) => {
  const theme = useTheme();
  const [activeTab, setActiveTab] = useState(0);
  const [saveDialogOpen, setSaveDialogOpen] = useState(false);
  const [selectedHistoryEntry, setSelectedHistoryEntry] = useState<SearchHistoryEntry | null>(null);
  const [saveName, setSaveName] = useState('');
  const [saveDescription, setSaveDescription] = useState('');
  const [saveTags, setSaveTags] = useState('');

  // Get mode icon
  const getModeIcon = (mode: SemanticSearchMode) => {
    switch (mode) {
      case 'natural_language': return <PsychologyIcon />;
      case 'vector_similarity': return <HubIcon />;
      case 'hybrid': return <AutoAwesomeIcon />;
      case 'clustering': return <FilterIcon />;
      default: return <PsychologyIcon />;
    }
  };

  // Get mode color
  const getModeColor = (mode: SemanticSearchMode) => {
    switch (mode) {
      case 'natural_language': return theme.palette.primary.main;
      case 'vector_similarity': return theme.palette.secondary.main;
      case 'hybrid': return theme.palette.success.main;
      case 'clustering': return theme.palette.warning.main;
      default: return theme.palette.primary.main;
    }
  };

  // Format time ago
  const formatTimeAgo = (timestamp: string) => {
    const now = new Date();
    const time = new Date(timestamp);
    const diffMs = now.getTime() - time.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return time.toLocaleDateString();
  };

  // Handle save search dialog
  const handleSaveClick = useCallback((entry: SearchHistoryEntry) => {
    setSelectedHistoryEntry(entry);
    setSaveName(`Search: ${entry.query.query.slice(0, 30)}${entry.query.query.length > 30 ? '...' : ''}`);
    setSaveDescription('');
    setSaveTags('');
    setSaveDialogOpen(true);
  }, []);

  const handleSaveConfirm = useCallback(async () => {
    if (!selectedHistoryEntry || !saveName.trim()) return;

    const search: Omit<SavedSemanticSearch, 'id' | 'created_at' | 'updated_at'> = {
      name: saveName.trim(),
      description: saveDescription.trim() || undefined,
      query: selectedHistoryEntry.query,
      tags: saveTags.split(',').map(tag => tag.trim()).filter(Boolean),
      user_id: undefined,
      is_public: false,
      usage_count: 0,
    };

    await onSaveSearch(search);
    setSaveDialogOpen(false);
    setSelectedHistoryEntry(null);
    setSaveName('');
    setSaveDescription('');
    setSaveTags('');
  }, [selectedHistoryEntry, saveName, saveDescription, saveTags, onSaveSearch]);

  // Render history item
  const renderHistoryItem = (entry: SearchHistoryEntry) => (
    <ListItem key={entry.id} disablePadding>
      <ListItemButton onClick={() => onHistorySelect(entry)}>
        <ListItemIcon>
          {getModeIcon(entry.query.mode)}
        </ListItemIcon>
        <ListItemText
          primary={
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
              <Typography variant="body2" noWrap sx={{ flexGrow: 1 }}>
                {entry.query.query}
              </Typography>
              <Chip
                label={entry.query.mode.replace('_', ' ')}
                size="small"
                sx={{
                  backgroundColor: getModeColor(entry.query.mode),
                  color: 'white',
                  fontSize: '0.7rem',
                }}
              />
            </Box>
          }
          secondary={
            <Box>
              <Typography variant="caption" color="text.secondary">
                {entry.results_count} results • {entry.execution_time_ms}ms • {formatTimeAgo(entry.timestamp)}
              </Typography>
            </Box>
          }
        />
        <ListItemSecondaryAction>
          <Tooltip title="Save search">
            <IconButton
              size="small"
              onClick={(e) => {
                e.stopPropagation();
                handleSaveClick(entry);
              }}
            >
              <SaveIcon fontSize="small" />
            </IconButton>
          </Tooltip>
        </ListItemSecondaryAction>
      </ListItemButton>
    </ListItem>
  );

  // Render saved search item
  const renderSavedSearchItem = (search: SavedSemanticSearch) => (
    <ListItem key={search.id} disablePadding>
      <ListItemButton onClick={() => onSavedSearchSelect(search)}>
        <ListItemIcon>
          <BookmarkIcon color="primary" />
        </ListItemIcon>
        <ListItemText
          primary={
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
              <Typography variant="body2" fontWeight="medium" noWrap sx={{ flexGrow: 1 }}>
                {search.name}
              </Typography>
              <Chip
                label={search.query.mode.replace('_', ' ')}
                size="small"
                sx={{
                  backgroundColor: getModeColor(search.query.mode),
                  color: 'white',
                  fontSize: '0.7rem',
                }}
              />
            </Box>
          }
          secondary={
            <Box>
              <Typography variant="caption" color="text.secondary" noWrap>
                {search.description || search.query.query}
              </Typography>
              <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5, mt: 0.5 }}>
                {search.tags.slice(0, 3).map((tag) => (
                  <Chip
                    key={tag}
                    label={tag}
                    size="small"
                    variant="outlined"
                    sx={{ fontSize: '0.6rem', height: 16 }}
                  />
                ))}
                {search.tags.length > 3 && (
                  <Chip
                    label={`+${search.tags.length - 3}`}
                    size="small"
                    variant="outlined"
                    sx={{ fontSize: '0.6rem', height: 16 }}
                  />
                )}
              </Box>
              <Typography variant="caption" color="text.secondary">
                Used {search.usage_count} times • {formatTimeAgo(search.created_at)}
              </Typography>
            </Box>
          }
        />
        <ListItemSecondaryAction>
          <Tooltip title="Delete saved search">
            <IconButton
              size="small"
              onClick={(e) => {
                e.stopPropagation();
                onDeleteSavedSearch(search.id);
              }}
            >
              <DeleteIcon fontSize="small" />
            </IconButton>
          </Tooltip>
        </ListItemSecondaryAction>
      </ListItemButton>
    </ListItem>
  );

  return (
    <Paper sx={{ height: '100%', display: 'flex', flexDirection: 'column' }} className={className}>
      {/* Header */}
      <Box sx={{ p: 2, borderBottom: 1, borderColor: 'divider' }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <HistoryIcon color="primary" />
          <Typography variant="h6">
            Search History
          </Typography>
        </Box>
      </Box>

      {/* Tabs */}
      <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
        <Tabs value={activeTab} onChange={(_, newValue) => setActiveTab(newValue)}>
          <Tab
            label={
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <AccessTimeIcon fontSize="small" />
                Recent ({history.length})
              </Box>
            }
          />
          <Tab
            label={
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <BookmarkIcon fontSize="small" />
                Saved ({savedSearches.length})
              </Box>
            }
          />
        </Tabs>
      </Box>

      {/* Tab Content */}
      <Box sx={{ flexGrow: 1, overflow: 'auto' }}>
        <TabPanel value={activeTab} index={0}>
          {history.length > 0 ? (
            <List dense>
              {history.map(renderHistoryItem)}
            </List>
          ) : (
            <Box sx={{ p: 3, textAlign: 'center' }}>
              <AccessTimeIcon sx={{ fontSize: 48, color: 'text.secondary', mb: 2 }} />
              <Typography variant="h6" color="text.secondary">
                No search history
              </Typography>
              <Typography variant="body2" color="text.secondary">
                Your recent searches will appear here
              </Typography>
            </Box>
          )}
        </TabPanel>

        <TabPanel value={activeTab} index={1}>
          {savedSearches.length > 0 ? (
            <List dense>
              {savedSearches.map(renderSavedSearchItem)}
            </List>
          ) : (
            <Box sx={{ p: 3, textAlign: 'center' }}>
              <BookmarkIcon sx={{ fontSize: 48, color: 'text.secondary', mb: 2 }} />
              <Typography variant="h6" color="text.secondary">
                No saved searches
              </Typography>
              <Typography variant="body2" color="text.secondary">
                Save your favorite searches for quick access
              </Typography>
            </Box>
          )}
        </TabPanel>
      </Box>

      {/* Save Search Dialog */}
      <Dialog open={saveDialogOpen} onClose={() => setSaveDialogOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle>Save Search</DialogTitle>
        <DialogContent>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2, mt: 1 }}>
            <TextField
              fullWidth
              label="Name"
              value={saveName}
              onChange={(e) => setSaveName(e.target.value)}
              placeholder="Enter a name for this search"
            />
            <TextField
              fullWidth
              label="Description"
              value={saveDescription}
              onChange={(e) => setSaveDescription(e.target.value)}
              placeholder="Optional description"
              multiline
              rows={2}
            />
            <TextField
              fullWidth
              label="Tags"
              value={saveTags}
              onChange={(e) => setSaveTags(e.target.value)}
              placeholder="Enter tags separated by commas"
              helperText="e.g., machine learning, files, analysis"
            />
            {selectedHistoryEntry && (
              <Box>
                <Typography variant="subtitle2" gutterBottom>
                  Search Query:
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  {selectedHistoryEntry.query.query}
                </Typography>
              </Box>
            )}
          </Box>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setSaveDialogOpen(false)}>Cancel</Button>
          <Button
            onClick={handleSaveConfirm}
            variant="contained"
            disabled={!saveName.trim()}
          >
            Save
          </Button>
        </DialogActions>
      </Dialog>
    </Paper>
  );
};

export default SearchHistory;