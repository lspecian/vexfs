import React, { useState, useCallback } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  Typography,
  Box,
  Card,
  CardContent,
  Chip,
  Stack,
  Divider,
  Alert,
  RadioGroup,
  FormControlLabel,
  Radio,
  TextField,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  IconButton,
  Tooltip,
} from '@mui/material';
import {
  ExpandMore as ExpandMoreIcon,
  Person as PersonIcon,
  Schedule as ScheduleIcon,
  Merge as MergeIcon,
  Undo as UndoIcon,
  Check as CheckIcon,
  Close as CloseIcon,
  Info as InfoIcon,
} from '@mui/icons-material';
import { formatDistanceToNow } from 'date-fns';
import { useRealTime } from './RealTimeProvider';
import type {
  ConflictDetectedEvent,
  ConflictResolutionStrategy,
  ConflictResolution,
} from '../../types/realtime';
import type { NodeResponse, EdgeResponse } from '../../types/graph';

export interface ConflictResolverProps {
  open: boolean;
  conflict: ConflictDetectedEvent | null;
  onResolve: (resolution: ConflictResolution) => void;
  onCancel: () => void;
}

interface ConflictData {
  local: any;
  remote: any;
  merged?: any;
}

const ConflictResolver: React.FC<ConflictResolverProps> = ({
  open,
  conflict,
  onResolve,
  onCancel,
}) => {
  const { resolveConflict } = useRealTime();
  const [selectedStrategy, setSelectedStrategy] = useState<ConflictResolutionStrategy>('remote_wins');
  const [manualResolution, setManualResolution] = useState<any>(null);
  const [mergedData, setMergedData] = useState<any>(null);
  const [isResolving, setIsResolving] = useState(false);

  // Reset state when conflict changes
  React.useEffect(() => {
    if (conflict) {
      setSelectedStrategy('remote_wins');
      setManualResolution(null);
      setMergedData(null);
      
      // Initialize merged data with remote changes
      if (conflict.data.conflictingChanges) {
        setMergedData({ ...conflict.data.conflictingChanges });
      }
    }
  }, [conflict]);

  const handleStrategyChange = useCallback((strategy: ConflictResolutionStrategy) => {
    setSelectedStrategy(strategy);
    
    if (strategy === 'merge' && conflict) {
      // Initialize merge with both local and remote changes
      const merged = {
        ...conflict.data.conflictingChanges,
        // Add any additional merge logic here
      };
      setMergedData(merged);
    }
  }, [conflict]);

  const handleManualFieldChange = useCallback((field: string, value: any) => {
    setMergedData((prev: any) => ({
      ...prev,
      [field]: value,
    }));
  }, []);

  const handleResolve = useCallback(async () => {
    if (!conflict) return;

    setIsResolving(true);
    
    try {
      let resolutionData = null;
      
      switch (selectedStrategy) {
        case 'local_wins':
          // Keep local version (no changes needed)
          break;
        case 'remote_wins':
          resolutionData = conflict.data.conflictingChanges;
          break;
        case 'merge':
          resolutionData = mergedData;
          break;
        case 'manual':
          resolutionData = manualResolution;
          break;
        case 'latest_timestamp':
          // Use the version with the latest timestamp
          resolutionData = conflict.data.remoteVersion > conflict.data.localVersion
            ? conflict.data.conflictingChanges
            : null;
          break;
      }

      const resolution = await resolveConflict(conflict, selectedStrategy, resolutionData);
      onResolve(resolution);
    } catch (error) {
      console.error('Failed to resolve conflict:', error);
    } finally {
      setIsResolving(false);
    }
  }, [conflict, selectedStrategy, mergedData, manualResolution, resolveConflict, onResolve]);

  const renderConflictDetails = () => {
    if (!conflict) return null;

    const { entityType, entityId, conflictType, localVersion, remoteVersion } = conflict.data;

    return (
      <Card variant="outlined" sx={{ mb: 2 }}>
        <CardContent>
          <Stack spacing={2}>
            <Box display="flex" alignItems="center" justifyContent="space-between">
              <Typography variant="h6">
                Conflict Details
              </Typography>
              <Chip
                label={conflictType.replace('_', ' ')}
                color="error"
                size="small"
              />
            </Box>
            
            <Box display="grid" gridTemplateColumns="1fr 1fr" gap={2}>
              <Typography variant="body2">
                <strong>Entity Type:</strong> {entityType}
              </Typography>
              <Typography variant="body2">
                <strong>Entity ID:</strong> {entityId}
              </Typography>
              <Typography variant="body2">
                <strong>Local Version:</strong> {localVersion}
              </Typography>
              <Typography variant="body2">
                <strong>Remote Version:</strong> {remoteVersion}
              </Typography>
            </Box>

            {conflict.userId && (
              <Box display="flex" alignItems="center" gap={1}>
                <PersonIcon fontSize="small" />
                <Typography variant="body2">
                  Modified by: {conflict.userId}
                </Typography>
              </Box>
            )}

            <Box display="flex" alignItems="center" gap={1}>
              <ScheduleIcon fontSize="small" />
              <Typography variant="body2">
                {formatDistanceToNow(new Date(conflict.timestamp), { addSuffix: true })}
              </Typography>
            </Box>
          </Stack>
        </CardContent>
      </Card>
    );
  };

  const renderChangesComparison = () => {
    if (!conflict?.data.conflictingChanges) return null;

    const changes = conflict.data.conflictingChanges;
    const changeKeys = Object.keys(changes);

    return (
      <Accordion>
        <AccordionSummary expandIcon={<ExpandMoreIcon />}>
          <Typography variant="subtitle1">
            Conflicting Changes ({changeKeys.length} fields)
          </Typography>
        </AccordionSummary>
        <AccordionDetails>
          <Stack spacing={2}>
            {changeKeys.map((key) => (
              <Box key={key}>
                <Typography variant="subtitle2" gutterBottom>
                  {key}
                </Typography>
                <Box display="grid" gridTemplateColumns="1fr 1fr" gap={2}>
                  <Card variant="outlined">
                    <CardContent>
                      <Typography variant="caption" color="text.secondary">
                        Local Value
                      </Typography>
                      <Typography variant="body2" sx={{ mt: 1 }}>
                        {JSON.stringify(changes[key], null, 2)}
                      </Typography>
                    </CardContent>
                  </Card>
                  <Card variant="outlined">
                    <CardContent>
                      <Typography variant="caption" color="text.secondary">
                        Remote Value
                      </Typography>
                      <Typography variant="body2" sx={{ mt: 1 }}>
                        {JSON.stringify(changes[key], null, 2)}
                      </Typography>
                    </CardContent>
                  </Card>
                </Box>
                {selectedStrategy === 'merge' && (
                  <TextField
                    fullWidth
                    label="Merged Value"
                    value={mergedData?.[key] || ''}
                    onChange={(e) => handleManualFieldChange(key, e.target.value)}
                    sx={{ mt: 1 }}
                    size="small"
                  />
                )}
              </Box>
            ))}
          </Stack>
        </AccordionDetails>
      </Accordion>
    );
  };

  const renderResolutionStrategies = () => {
    return (
      <Box>
        <Typography variant="subtitle1" gutterBottom>
          Resolution Strategy
        </Typography>
        <RadioGroup
          value={selectedStrategy}
          onChange={(e) => handleStrategyChange(e.target.value as ConflictResolutionStrategy)}
        >
          <FormControlLabel
            value="remote_wins"
            control={<Radio />}
            label={
              <Box>
                <Typography variant="body2">
                  <strong>Accept Remote Changes</strong>
                </Typography>
                <Typography variant="caption" color="text.secondary">
                  Use the remote version and discard local changes
                </Typography>
              </Box>
            }
          />
          
          <FormControlLabel
            value="local_wins"
            control={<Radio />}
            label={
              <Box>
                <Typography variant="body2">
                  <strong>Keep Local Changes</strong>
                </Typography>
                <Typography variant="caption" color="text.secondary">
                  Keep the local version and ignore remote changes
                </Typography>
              </Box>
            }
          />
          
          <FormControlLabel
            value="merge"
            control={<Radio />}
            label={
              <Box>
                <Typography variant="body2">
                  <strong>Merge Changes</strong>
                </Typography>
                <Typography variant="caption" color="text.secondary">
                  Combine local and remote changes manually
                </Typography>
              </Box>
            }
          />
          
          <FormControlLabel
            value="latest_timestamp"
            control={<Radio />}
            label={
              <Box>
                <Typography variant="body2">
                  <strong>Use Latest Version</strong>
                </Typography>
                <Typography variant="caption" color="text.secondary">
                  Automatically choose the version with the latest timestamp
                </Typography>
              </Box>
            }
          />
          
          <FormControlLabel
            value="manual"
            control={<Radio />}
            label={
              <Box>
                <Typography variant="body2">
                  <strong>Manual Resolution</strong>
                </Typography>
                <Typography variant="caption" color="text.secondary">
                  Provide a completely custom resolution
                </Typography>
              </Box>
            }
          />
        </RadioGroup>

        {selectedStrategy === 'manual' && (
          <TextField
            fullWidth
            multiline
            rows={4}
            label="Manual Resolution (JSON)"
            value={manualResolution ? JSON.stringify(manualResolution, null, 2) : ''}
            onChange={(e) => {
              try {
                const parsed = JSON.parse(e.target.value);
                setManualResolution(parsed);
              } catch {
                // Invalid JSON, keep the text for editing
              }
            }}
            sx={{ mt: 2 }}
            helperText="Enter the resolved data as JSON"
          />
        )}
      </Box>
    );
  };

  if (!conflict) return null;

  return (
    <Dialog
      open={open}
      onClose={onCancel}
      maxWidth="md"
      fullWidth
      PaperProps={{
        sx: { minHeight: '60vh' }
      }}
    >
      <DialogTitle>
        <Box display="flex" alignItems="center" gap={1}>
          <MergeIcon color="error" />
          <Typography variant="h6">
            Resolve Conflict
          </Typography>
        </Box>
      </DialogTitle>
      
      <DialogContent>
        <Stack spacing={3}>
          <Alert severity="warning">
            A conflict has been detected due to concurrent modifications. 
            Please choose how to resolve this conflict.
          </Alert>

          {renderConflictDetails()}
          
          {renderChangesComparison()}
          
          <Divider />
          
          {renderResolutionStrategies()}
        </Stack>
      </DialogContent>
      
      <DialogActions>
        <Button
          onClick={onCancel}
          startIcon={<CloseIcon />}
        >
          Cancel
        </Button>
        
        <Button
          onClick={handleResolve}
          variant="contained"
          startIcon={<CheckIcon />}
          disabled={isResolving || (selectedStrategy === 'manual' && !manualResolution)}
          color="primary"
        >
          {isResolving ? 'Resolving...' : 'Resolve Conflict'}
        </Button>
      </DialogActions>
    </Dialog>
  );
};

export default ConflictResolver;