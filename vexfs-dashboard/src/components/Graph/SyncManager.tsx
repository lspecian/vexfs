import React, { useState, useCallback, useEffect, useRef } from 'react';
import {
  Box,
  Button,
  Card,
  CardContent,
  Typography,
  LinearProgress,
  Alert,
  Chip,
  Stack,
  IconButton,
  Tooltip,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  Divider,
} from '@mui/material';
import {
  Sync as SyncIcon,
  SyncProblem as SyncProblemIcon,
  CheckCircle as CheckCircleIcon,
  Error as ErrorIcon,
  Warning as WarningIcon,
  Refresh as RefreshIcon,
  History as HistoryIcon,
  CloudSync as CloudSyncIcon,
  Schedule as ScheduleIcon,
} from '@mui/icons-material';
import { formatDistanceToNow } from 'date-fns';
import { useRealTime } from './RealTimeProvider';
import type {
  GraphEvent,
  SyncRequiredEvent,
} from '../../types/realtime';
import type { NodeId, EdgeId } from '../../types/graph';

export interface SyncManagerProps {
  autoSync?: boolean;
  syncInterval?: number;
  showSyncHistory?: boolean;
  onSyncComplete?: (result: SyncResult) => void;
  onSyncError?: (error: Error) => void;
}

interface SyncResult {
  success: boolean;
  timestamp: Date;
  syncedNodes: NodeId[];
  syncedEdges: EdgeId[];
  conflicts: number;
  errors: string[];
  duration: number;
}

interface SyncOperation {
  id: string;
  type: 'manual' | 'auto' | 'conflict_resolution' | 'connection_restored';
  status: 'pending' | 'in_progress' | 'completed' | 'failed';
  startTime: Date;
  endTime?: Date;
  result?: SyncResult;
  error?: string;
}

const SyncManager: React.FC<SyncManagerProps> = ({
  autoSync = true,
  syncInterval = 30000, // 30 seconds
  showSyncHistory = true,
  onSyncComplete,
  onSyncError,
}) => {
  const { state, syncGraphState, getChangeHistory, subscribe, unsubscribe } = useRealTime();
  
  const [syncOperations, setSyncOperations] = useState<SyncOperation[]>([]);
  const [currentSync, setCurrentSync] = useState<SyncOperation | null>(null);
  const [lastSyncTime, setLastSyncTime] = useState<Date | null>(null);
  const [syncRequired, setSyncRequired] = useState(false);
  const [syncDialogOpen, setSyncDialogOpen] = useState(false);
  const [pendingChanges, setPendingChanges] = useState<GraphEvent[]>([]);
  
  const autoSyncIntervalRef = useRef<NodeJS.Timeout | null>(null);
  const syncInProgress = useRef(false);

  // Subscribe to sync required events
  useEffect(() => {
    const subscriptionId = subscribe({
      eventTypes: ['graph.sync.required'],
      callback: handleSyncRequired,
    });

    return () => {
      unsubscribe(subscriptionId);
    };
  }, []);

  // Setup auto-sync
  useEffect(() => {
    if (autoSync && syncInterval > 0) {
      autoSyncIntervalRef.current = setInterval(() => {
        if (!syncInProgress.current && state.connectionStatus.state === 'connected') {
          performSync('auto');
        }
      }, syncInterval);
    }

    return () => {
      if (autoSyncIntervalRef.current) {
        clearInterval(autoSyncIntervalRef.current);
      }
    };
  }, [autoSync, syncInterval, state.connectionStatus.state]);

  // Handle sync required events
  const handleSyncRequired = useCallback((event: GraphEvent) => {
    const syncEvent = event as SyncRequiredEvent;
    setSyncRequired(true);
    
    // Auto-sync if enabled and reason is connection restored
    if (autoSync && syncEvent.data.reason === 'connection_restored') {
      performSync('connection_restored');
    }
  }, [autoSync]);

  // Perform synchronization
  const performSync = useCallback(async (
    type: SyncOperation['type'] = 'manual'
  ): Promise<SyncResult> => {
    if (syncInProgress.current) {
      throw new Error('Sync already in progress');
    }

    syncInProgress.current = true;
    const startTime = new Date();
    
    const operation: SyncOperation = {
      id: `sync_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      type,
      status: 'in_progress',
      startTime,
    };

    setCurrentSync(operation);
    setSyncOperations(prev => [operation, ...prev].slice(0, 20)); // Keep last 20 operations

    try {
      // Get pending changes since last sync
      const changes = await getChangeHistory(lastSyncTime || undefined);
      setPendingChanges(changes);

      // Perform the actual sync
      await syncGraphState();

      // Calculate sync result
      const endTime = new Date();
      const duration = endTime.getTime() - startTime.getTime();
      
      const result: SyncResult = {
        success: true,
        timestamp: endTime,
        syncedNodes: extractNodeIds(changes),
        syncedEdges: extractEdgeIds(changes),
        conflicts: state.pendingConflicts.length,
        errors: [],
        duration,
      };

      // Update operation
      const completedOperation: SyncOperation = {
        ...operation,
        status: 'completed',
        endTime,
        result,
      };

      setCurrentSync(null);
      setSyncOperations(prev => 
        prev.map(op => op.id === operation.id ? completedOperation : op)
      );
      setLastSyncTime(endTime);
      setSyncRequired(false);

      if (onSyncComplete) {
        onSyncComplete(result);
      }

      return result;
    } catch (error) {
      const endTime = new Date();
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      
      const failedOperation: SyncOperation = {
        ...operation,
        status: 'failed',
        endTime,
        error: errorMessage,
      };

      setCurrentSync(null);
      setSyncOperations(prev => 
        prev.map(op => op.id === operation.id ? failedOperation : op)
      );

      if (onSyncError) {
        onSyncError(error instanceof Error ? error : new Error(errorMessage));
      }

      throw error;
    } finally {
      syncInProgress.current = false;
    }
  }, [state.pendingConflicts.length, lastSyncTime, syncGraphState, getChangeHistory, onSyncComplete, onSyncError]);

  // Extract node IDs from events
  const extractNodeIds = useCallback((events: GraphEvent[]): NodeId[] => {
    const nodeIds = new Set<NodeId>();
    
    events.forEach(event => {
      if (event.type.includes('node')) {
        const nodeEvent = event as any;
        if (nodeEvent.data?.nodeId) {
          nodeIds.add(nodeEvent.data.nodeId);
        }
        if (nodeEvent.data?.node?.id) {
          nodeIds.add(nodeEvent.data.node.id);
        }
      }
    });
    
    return Array.from(nodeIds);
  }, []);

  // Extract edge IDs from events
  const extractEdgeIds = useCallback((events: GraphEvent[]): EdgeId[] => {
    const edgeIds = new Set<EdgeId>();
    
    events.forEach(event => {
      if (event.type.includes('edge')) {
        const edgeEvent = event as any;
        if (edgeEvent.data?.edgeId) {
          edgeIds.add(edgeEvent.data.edgeId);
        }
        if (edgeEvent.data?.edge?.id) {
          edgeIds.add(edgeEvent.data.edge.id);
        }
      }
    });
    
    return Array.from(edgeIds);
  }, []);

  // Manual sync trigger
  const handleManualSync = useCallback(async () => {
    try {
      await performSync('manual');
    } catch (error) {
      console.error('Manual sync failed:', error);
    }
  }, [performSync]);

  // Get sync status
  const getSyncStatus = () => {
    if (currentSync) {
      return {
        status: 'syncing',
        message: 'Synchronizing graph state...',
        color: 'info' as const,
        icon: <SyncIcon className="animate-spin" />,
      };
    }

    if (syncRequired) {
      return {
        status: 'required',
        message: 'Synchronization required',
        color: 'warning' as const,
        icon: <SyncProblemIcon />,
      };
    }

    if (state.connectionStatus.state !== 'connected') {
      return {
        status: 'disconnected',
        message: 'Disconnected - sync unavailable',
        color: 'error' as const,
        icon: <ErrorIcon />,
      };
    }

    const lastOperation = syncOperations[0];
    if (lastOperation?.status === 'failed') {
      return {
        status: 'error',
        message: 'Last sync failed',
        color: 'error' as const,
        icon: <ErrorIcon />,
      };
    }

    return {
      status: 'synced',
      message: lastSyncTime 
        ? `Last synced ${formatDistanceToNow(lastSyncTime, { addSuffix: true })}`
        : 'Ready to sync',
      color: 'success' as const,
      icon: <CheckCircleIcon />,
    };
  };

  const syncStatus = getSyncStatus();

  return (
    <>
      <Card variant="outlined">
        <CardContent>
          <Box display="flex" alignItems="center" justifyContent="space-between" mb={2}>
            <Box display="flex" alignItems="center" gap={1}>
              {syncStatus.icon}
              <Typography variant="h6">
                Graph Synchronization
              </Typography>
            </Box>
            
            <Stack direction="row" spacing={1}>
              <Chip
                label={syncStatus.status}
                color={syncStatus.color}
                size="small"
              />
              
              {showSyncHistory && (
                <Tooltip title="View Sync History">
                  <IconButton size="small" onClick={() => setSyncDialogOpen(true)}>
                    <HistoryIcon />
                  </IconButton>
                </Tooltip>
              )}
            </Stack>
          </Box>

          <Typography variant="body2" color="text.secondary" gutterBottom>
            {syncStatus.message}
          </Typography>

          {currentSync && (
            <Box mt={2}>
              <LinearProgress />
              <Typography variant="caption" color="text.secondary" mt={1}>
                Sync started {formatDistanceToNow(currentSync.startTime, { addSuffix: true })}
              </Typography>
            </Box>
          )}

          {syncRequired && !currentSync && (
            <Alert severity="warning" sx={{ mt: 2 }}>
              Graph synchronization is required. Some changes may not be reflected.
            </Alert>
          )}

          {state.pendingConflicts.length > 0 && (
            <Alert severity="error" sx={{ mt: 2 }}>
              {state.pendingConflicts.length} unresolved conflicts detected.
              Resolve conflicts before syncing.
            </Alert>
          )}

          <Box display="flex" gap={1} mt={2}>
            <Button
              variant="contained"
              startIcon={<SyncIcon />}
              onClick={handleManualSync}
              disabled={currentSync !== null || state.connectionStatus.state !== 'connected'}
              size="small"
            >
              Sync Now
            </Button>
            
            <Button
              variant="outlined"
              startIcon={<RefreshIcon />}
              onClick={() => setSyncRequired(true)}
              size="small"
            >
              Check for Changes
            </Button>
          </Box>

          {/* Auto-sync status */}
          <Box mt={2}>
            <Typography variant="caption" color="text.secondary">
              Auto-sync: {autoSync ? 'Enabled' : 'Disabled'}
              {autoSync && ` (every ${Math.round(syncInterval / 1000)}s)`}
            </Typography>
          </Box>
        </CardContent>
      </Card>

      {/* Sync History Dialog */}
      <Dialog
        open={syncDialogOpen}
        onClose={() => setSyncDialogOpen(false)}
        maxWidth="md"
        fullWidth
      >
        <DialogTitle>
          <Box display="flex" alignItems="center" gap={1}>
            <HistoryIcon />
            Synchronization History
          </Box>
        </DialogTitle>
        
        <DialogContent>
          {syncOperations.length === 0 ? (
            <Typography variant="body2" color="text.secondary" textAlign="center" py={4}>
              No synchronization operations yet
            </Typography>
          ) : (
            <List>
              {syncOperations.map((operation, index) => (
                <React.Fragment key={operation.id}>
                  <ListItem>
                    <ListItemIcon>
                      {operation.status === 'completed' && <CheckCircleIcon color="success" />}
                      {operation.status === 'failed' && <ErrorIcon color="error" />}
                      {operation.status === 'in_progress' && <SyncIcon color="info" />}
                      {operation.status === 'pending' && <ScheduleIcon color="warning" />}
                    </ListItemIcon>
                    
                    <ListItemText
                      primary={
                        <Box display="flex" alignItems="center" gap={1}>
                          <Typography variant="body2">
                            {operation.type.replace('_', ' ')} sync
                          </Typography>
                          <Chip
                            label={operation.status}
                            size="small"
                            color={
                              operation.status === 'completed' ? 'success' :
                              operation.status === 'failed' ? 'error' :
                              operation.status === 'in_progress' ? 'info' : 'default'
                            }
                          />
                        </Box>
                      }
                      secondary={
                        <Stack spacing={0.5}>
                          <Typography variant="caption">
                            Started: {formatDistanceToNow(operation.startTime, { addSuffix: true })}
                          </Typography>
                          
                          {operation.endTime && (
                            <Typography variant="caption">
                              Duration: {operation.endTime.getTime() - operation.startTime.getTime()}ms
                            </Typography>
                          )}
                          
                          {operation.result && (
                            <Typography variant="caption">
                              Synced: {operation.result.syncedNodes.length} nodes, {operation.result.syncedEdges.length} edges
                            </Typography>
                          )}
                          
                          {operation.error && (
                            <Typography variant="caption" color="error">
                              Error: {operation.error}
                            </Typography>
                          )}
                        </Stack>
                      }
                    />
                  </ListItem>
                  {index < syncOperations.length - 1 && <Divider />}
                </React.Fragment>
              ))}
            </List>
          )}
        </DialogContent>
        
        <DialogActions>
          <Button onClick={() => setSyncDialogOpen(false)}>
            Close
          </Button>
        </DialogActions>
      </Dialog>
    </>
  );
};

export default SyncManager;