import React, { useState, useCallback, useRef, useEffect } from 'react';
import {
  Box,
  TextField,
  InputAdornment,
  IconButton,
  Button,
  ButtonGroup,
  Chip,
  Paper,
  List,
  ListItem,
  ListItemText,
  ListItemIcon,
  Popper,
  ClickAwayListener,
  CircularProgress,
  Tooltip,
  useTheme,
} from '@mui/material';
import {
  Search as SearchIcon,
  Psychology as PsychologyIcon,
  Hub as HubIcon,
  AutoAwesome as AutoAwesomeIcon,
  FilterList as FilterIcon,
  Clear as ClearIcon,
  Lightbulb as LightbulbIcon,
  TrendingUp as TrendingIcon,
  Category as CategoryIcon,
} from '@mui/icons-material';

import type {
  SemanticSearchMode,
  SearchSuggestion,
  SearchQueryInputProps,
} from '../../types/semantic';

const SearchQueryInput: React.FC<SearchQueryInputProps> = ({
  value,
  onChange,
  onSearch,
  mode,
  onModeChange,
  suggestions = [],
  isLoading = false,
  placeholder = 'Search using natural language...',
  className,
}) => {
  const theme = useTheme();
  const inputRef = useRef<HTMLInputElement>(null);
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [selectedSuggestionIndex, setSelectedSuggestionIndex] = useState(-1);

  // Search mode configuration
  const searchModes: Array<{
    mode: SemanticSearchMode;
    label: string;
    icon: React.ReactNode;
    description: string;
    color: 'primary' | 'secondary' | 'success' | 'warning';
  }> = [
    {
      mode: 'natural_language',
      label: 'Natural Language',
      icon: <PsychologyIcon />,
      description: 'Search using natural language queries',
      color: 'primary',
    },
    {
      mode: 'vector_similarity',
      label: 'Vector Similarity',
      icon: <HubIcon />,
      description: 'Find semantically similar nodes',
      color: 'secondary',
    },
    {
      mode: 'hybrid',
      label: 'Hybrid Search',
      icon: <AutoAwesomeIcon />,
      description: 'Combine keyword and semantic search',
      color: 'success',
    },
    {
      mode: 'clustering',
      label: 'Clustering',
      icon: <FilterIcon />,
      description: 'Group semantically related results',
      color: 'warning',
    },
  ];

  const currentModeConfig = searchModes.find(m => m.mode === mode) || searchModes[0];

  // Handle input changes
  const handleInputChange = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = event.target.value;
    onChange(newValue);
    
    // Show suggestions if there's input and suggestions available
    if (newValue.trim() && suggestions.length > 0) {
      setShowSuggestions(true);
      setAnchorEl(inputRef.current);
    } else {
      setShowSuggestions(false);
    }
  }, [onChange, suggestions.length]);

  // Handle key events
  const handleKeyDown = useCallback((event: React.KeyboardEvent) => {
    if (event.key === 'Enter') {
      event.preventDefault();
      if (selectedSuggestionIndex >= 0 && suggestions[selectedSuggestionIndex]) {
        onChange(suggestions[selectedSuggestionIndex].text);
        setShowSuggestions(false);
      } else {
        onSearch();
      }
    } else if (event.key === 'Escape') {
      setShowSuggestions(false);
      setSelectedSuggestionIndex(-1);
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      setSelectedSuggestionIndex(prev => 
        prev < suggestions.length - 1 ? prev + 1 : prev
      );
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      setSelectedSuggestionIndex(prev => prev > 0 ? prev - 1 : -1);
    }
  }, [selectedSuggestionIndex, suggestions, onChange, onSearch]);

  // Handle suggestion selection
  const handleSuggestionSelect = useCallback((suggestion: SearchSuggestion) => {
    onChange(suggestion.text);
    setShowSuggestions(false);
    setSelectedSuggestionIndex(-1);
    // Auto-search on suggestion selection
    setTimeout(() => onSearch(), 100);
  }, [onChange, onSearch]);

  // Handle click away
  const handleClickAway = useCallback(() => {
    setShowSuggestions(false);
    setSelectedSuggestionIndex(-1);
  }, []);

  // Clear input
  const handleClear = useCallback(() => {
    onChange('');
    setShowSuggestions(false);
    inputRef.current?.focus();
  }, [onChange]);

  // Get suggestion icon
  const getSuggestionIcon = (suggestion: SearchSuggestion) => {
    switch (suggestion.type) {
      case 'query': return <LightbulbIcon fontSize="small" />;
      case 'filter': return <FilterIcon fontSize="small" />;
      case 'concept': return <CategoryIcon fontSize="small" />;
      default: return <TrendingIcon fontSize="small" />;
    }
  };

  // Example queries for each mode
  const getPlaceholderForMode = (searchMode: SemanticSearchMode): string => {
    switch (searchMode) {
      case 'natural_language':
        return 'Find files related to machine learning algorithms...';
      case 'vector_similarity':
        return 'Find nodes similar to the selected reference...';
      case 'hybrid':
        return 'Combine keyword and semantic search...';
      case 'clustering':
        return 'Group related files by semantic similarity...';
      default:
        return placeholder;
    }
  };

  return (
    <Box className={className}>
      {/* Search Mode Selector */}
      <Box sx={{ mb: 2 }}>
        <ButtonGroup variant="outlined" size="small" fullWidth>
          {searchModes.map((modeConfig) => (
            <Tooltip key={modeConfig.mode} title={modeConfig.description}>
              <Button
                variant={mode === modeConfig.mode ? 'contained' : 'outlined'}
                color={mode === modeConfig.mode ? modeConfig.color : 'inherit'}
                startIcon={modeConfig.icon}
                onClick={() => onModeChange(modeConfig.mode)}
                sx={{ 
                  flex: 1,
                  minWidth: 0,
                  '& .MuiButton-startIcon': { mr: { xs: 0, sm: 1 } },
                  '& .MuiButton-startIcon > *:nth-of-type(1)': { fontSize: '1rem' },
                }}
              >
                <Box component="span" sx={{ display: { xs: 'none', sm: 'inline' } }}>
                  {modeConfig.label}
                </Box>
              </Button>
            </Tooltip>
          ))}
        </ButtonGroup>
      </Box>

      {/* Current Mode Indicator */}
      <Box sx={{ mb: 1, display: 'flex', alignItems: 'center', gap: 1 }}>
        <Chip
          icon={currentModeConfig.icon as React.ReactElement}
          label={currentModeConfig.label}
          color={currentModeConfig.color}
          size="small"
          variant="outlined"
        />
        <Box sx={{ fontSize: '0.875rem', color: 'text.secondary', flexGrow: 1 }}>
          {currentModeConfig.description}
        </Box>
      </Box>

      {/* Search Input */}
      <ClickAwayListener onClickAway={handleClickAway}>
        <Box sx={{ position: 'relative' }}>
          <TextField
            ref={inputRef}
            fullWidth
            variant="outlined"
            value={value}
            onChange={handleInputChange}
            onKeyDown={handleKeyDown}
            placeholder={getPlaceholderForMode(mode)}
            disabled={isLoading}
            InputProps={{
              startAdornment: (
                <InputAdornment position="start">
                  {isLoading ? (
                    <CircularProgress size={20} />
                  ) : (
                    currentModeConfig.icon
                  )}
                </InputAdornment>
              ),
              endAdornment: (
                <InputAdornment position="end">
                  {value && (
                    <IconButton
                      size="small"
                      onClick={handleClear}
                      disabled={isLoading}
                      edge="end"
                    >
                      <ClearIcon />
                    </IconButton>
                  )}
                  <IconButton
                    onClick={onSearch}
                    disabled={!value.trim() || isLoading}
                    color="primary"
                    edge="end"
                  >
                    <SearchIcon />
                  </IconButton>
                </InputAdornment>
              ),
            }}
            sx={{
              '& .MuiOutlinedInput-root': {
                backgroundColor: theme.palette.background.paper,
                '&:hover': {
                  backgroundColor: theme.palette.action.hover,
                },
                '&.Mui-focused': {
                  backgroundColor: theme.palette.background.paper,
                },
              },
            }}
          />

          {/* Suggestions Popper */}
          <Popper
            open={showSuggestions && suggestions.length > 0}
            anchorEl={anchorEl}
            placement="bottom-start"
            style={{ width: anchorEl?.clientWidth, zIndex: 1300 }}
          >
            <Paper
              elevation={8}
              sx={{
                maxHeight: 300,
                overflow: 'auto',
                border: 1,
                borderColor: 'divider',
              }}
            >
              <List dense>
                {suggestions.map((suggestion, index) => (
                  <ListItem
                    key={`${suggestion.text}-${index}`}
                    button
                    selected={index === selectedSuggestionIndex}
                    onClick={() => handleSuggestionSelect(suggestion)}
                    sx={{
                      '&.Mui-selected': {
                        backgroundColor: theme.palette.action.selected,
                      },
                    }}
                  >
                    <ListItemIcon sx={{ minWidth: 32 }}>
                      {getSuggestionIcon(suggestion)}
                    </ListItemIcon>
                    <ListItemText
                      primary={suggestion.text}
                      secondary={suggestion.category}
                      primaryTypographyProps={{
                        variant: 'body2',
                        noWrap: true,
                      }}
                      secondaryTypographyProps={{
                        variant: 'caption',
                        noWrap: true,
                      }}
                    />
                    <Box
                      sx={{
                        ml: 1,
                        px: 1,
                        py: 0.25,
                        borderRadius: 1,
                        backgroundColor: theme.palette.action.hover,
                        fontSize: '0.75rem',
                        color: 'text.secondary',
                      }}
                    >
                      {Math.round(suggestion.confidence * 100)}%
                    </Box>
                  </ListItem>
                ))}
              </List>
            </Paper>
          </Popper>
        </Box>
      </ClickAwayListener>

      {/* Quick Examples */}
      {!value && (
        <Box sx={{ mt: 1 }}>
          <Box sx={{ fontSize: '0.75rem', color: 'text.secondary', mb: 0.5 }}>
            Try these examples:
          </Box>
          <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
            {mode === 'natural_language' && [
              'Find configuration files',
              'Show documents about databases',
              'Files modified last week',
            ].map((example) => (
              <Chip
                key={example}
                label={example}
                size="small"
                variant="outlined"
                clickable
                onClick={() => onChange(example)}
                sx={{ fontSize: '0.75rem', height: 24 }}
              />
            ))}
            {mode === 'vector_similarity' && [
              'Similar to selected node',
              'Find related documents',
              'Semantic duplicates',
            ].map((example) => (
              <Chip
                key={example}
                label={example}
                size="small"
                variant="outlined"
                clickable
                onClick={() => onChange(example)}
                sx={{ fontSize: '0.75rem', height: 24 }}
              />
            ))}
          </Box>
        </Box>
      )}
    </Box>
  );
};

export default SearchQueryInput;