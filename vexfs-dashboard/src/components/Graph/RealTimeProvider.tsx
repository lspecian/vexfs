import React, { createContext, useContext, useEffect, useRef, useState, useCallback } from 'react';
import { useSnackbar } from 'notistack';
import WebSocketManager from './WebSocketManager';
import type {
  ConnectionStatus,
  GraphEvent,
  GraphSubscription,
  RealTimeState,
  RealTimeContextValue,
  ConflictDetectedEvent,
  ConflictResolutionStrategy,
  ConflictResolution,
  UpdateQueueItem,
  RealTimeNotification,
  RealTimeMetrics,
} from '../../types/realtime';
import type { NodeResponse, EdgeResponse } from '../../types/graph';

// Create the Real-Time Context
const RealTimeContext = createContext<RealTimeContextValue | null>(null);

export interface RealTimeProviderProps {
  children: React.ReactNode;
  wsUrl?: string;
  enableOptimisticUpdates?: boolean;
  enableBatching?: boolean;
  batchSize?: number;
  batchTimeout?: number;
}

export const RealTimeProvider: React.FC<RealTimeProviderProps> = ({
  children,
  wsUrl = 'ws://localhost:7680',
  enableOptimisticUpdates: initialOptimisticUpdates = true,
  enableBatching: initialBatching = true,
  batchSize = 10,
  batchTimeout = 1000,
}) => {
  const { enqueueSnackbar } = useSnackbar();
  const wsManagerRef = useRef<WebSocketManager | null>(null);
  
  // Real-time state
  const [state, setState] = useState<RealTimeState>({
    connectionStatus: {
      state: 'disconnected',
      reconnectAttempts: 0,
    },
    subscriptions: new Map(),
    updateQueue: [],
    pendingConflicts: [],
    isOptimisticUpdatesEnabled: initialOptimisticUpdates,
    isBatchingEnabled: initialBatching,
    batchSize,
    batchTimeout,
  });

  const [notifications, setNotifications] = useState<RealTimeNotification[]>([]);
  const [metrics, setMetrics] = useState<RealTimeMetrics>({
    messagesReceived: 0,
    messagesSent: 0,
    eventsProcessed: 0,
    conflictsDetected: 0,
    conflictsResolved: 0,
    averageLatency: 0,
    connectionUptime: 0,
    reconnectionCount: 0,
    lastUpdated: new Date(),
  });

  // Batch processing
  const batchTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const pendingBatch = useRef<GraphEvent[]>([]);

  // Initialize WebSocket Manager
  useEffect(() => {
    wsManagerRef.current = new WebSocketManager({
      config: { url: wsUrl },
      onConnectionChange: handleConnectionChange,
      onEvent: handleGraphEvent,
      onError: handleError,
    });

    return () => {
      if (wsManagerRef.current) {
        wsManagerRef.current.destroy();
      }
    };
  }, [wsUrl]);

  // Connection Management
  const handleConnectionChange = useCallback((connectionStatus: ConnectionStatus) => {
    setState(prev => ({
      ...prev,
      connectionStatus,
    }));

    // Show connection status notifications
    if (connectionStatus.state === 'connected') {
      enqueueSnackbar('Connected to real-time updates', { variant: 'success' });
    } else if (connectionStatus.state === 'disconnected') {
      enqueueSnackbar('Disconnected from real-time updates', { variant: 'warning' });
    } else if (connectionStatus.state === 'error') {
      enqueueSnackbar(`Connection error: ${connectionStatus.error}`, { variant: 'error' });
    }
  }, [enqueueSnackbar]);

  const handleError = useCallback((error: Error) => {
    console.error('WebSocket error:', error);
    enqueueSnackbar(`Real-time connection error: ${error.message}`, { variant: 'error' });
  }, [enqueueSnackbar]);

  // Event Handling
  const handleGraphEvent = useCallback((event: GraphEvent) => {
    if (state.isBatchingEnabled) {
      addToBatch(event);
    } else {
      processEvent(event);
    }

    // Update metrics
    setMetrics(prev => ({
      ...prev,
      eventsProcessed: prev.eventsProcessed + 1,
      lastUpdated: new Date(),
    }));
  }, [state.isBatchingEnabled]);

  const addToBatch = useCallback((event: GraphEvent) => {
    pendingBatch.current.push(event);

    if (pendingBatch.current.length >= state.batchSize) {
      flushBatch();
    } else if (!batchTimeoutRef.current) {
      batchTimeoutRef.current = setTimeout(flushBatch, state.batchTimeout);
    }
  }, [state.batchSize, state.batchTimeout]);

  const flushBatch = useCallback(() => {
    if (batchTimeoutRef.current) {
      clearTimeout(batchTimeoutRef.current);
      batchTimeoutRef.current = null;
    }

    const events = [...pendingBatch.current];
    pendingBatch.current = [];

    events.forEach(processEvent);
  }, []);

  const processEvent = useCallback((event: GraphEvent) => {
    // Handle conflict detection
    if (event.type === 'graph.conflict.detected') {
      setState(prev => ({
        ...prev,
        pendingConflicts: [...prev.pendingConflicts, event as ConflictDetectedEvent],
      }));

      setMetrics(prev => ({
        ...prev,
        conflictsDetected: prev.conflictsDetected + 1,
      }));

      showConflictNotification(event as ConflictDetectedEvent);
      return;
    }

    // Add to update queue
    const queueItem: UpdateQueueItem = {
      id: `update_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      event,
      timestamp: new Date(),
      retryCount: 0,
      processed: false,
    };

    setState(prev => ({
      ...prev,
      updateQueue: [...prev.updateQueue, queueItem],
    }));

    // Show event notification
    showEventNotification(event);
  }, []);

  // Notification Management
  const showEventNotification = useCallback((event: GraphEvent) => {
    let title = '';
    let message = '';
    let type: 'info' | 'success' | 'warning' | 'error' = 'info';

    switch (event.type) {
      case 'graph.node.created':
        title = 'Node Created';
        message = `New node added to the graph`;
        type = 'success';
        break;
      case 'graph.node.updated':
        title = 'Node Updated';
        message = `Node properties have been modified`;
        type = 'info';
        break;
      case 'graph.node.deleted':
        title = 'Node Deleted';
        message = `Node has been removed from the graph`;
        type = 'warning';
        break;
      case 'graph.edge.created':
        title = 'Edge Created';
        message = `New connection added to the graph`;
        type = 'success';
        break;
      case 'graph.edge.updated':
        title = 'Edge Updated';
        message = `Edge properties have been modified`;
        type = 'info';
        break;
      case 'graph.edge.deleted':
        title = 'Edge Deleted';
        message = `Connection has been removed from the graph`;
        type = 'warning';
        break;
      case 'graph.schema.updated':
        title = 'Schema Updated';
        message = `Graph schema has been modified`;
        type = 'info';
        break;
      case 'graph.bulk.operation':
        title = 'Bulk Operation';
        message = `Multiple graph entities have been modified`;
        type = 'info';
        break;
      default:
        return; // Don't show notification for unknown events
    }

    const notification: RealTimeNotification = {
      id: `notif_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      type,
      title,
      message,
      event,
      timestamp: new Date(),
      autoHide: true,
      duration: 3000,
    };

    setNotifications(prev => [...prev, notification]);

    // Auto-remove notification
    setTimeout(() => {
      setNotifications(prev => prev.filter(n => n.id !== notification.id));
    }, notification.duration);
  }, []);

  const showConflictNotification = useCallback((conflictEvent: ConflictDetectedEvent) => {
    const notification: RealTimeNotification = {
      id: `conflict_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      type: 'error',
      title: 'Conflict Detected',
      message: `Concurrent modification detected on ${conflictEvent.data.entityType}`,
      event: conflictEvent,
      timestamp: new Date(),
      autoHide: false,
      actions: [
        {
          label: 'Resolve',
          action: () => {
            // Open conflict resolution dialog
            console.log('Opening conflict resolution for:', conflictEvent);
          },
        },
        {
          label: 'Dismiss',
          action: () => {
            setNotifications(prev => prev.filter(n => n.id !== notification.id));
          },
        },
      ],
    };

    setNotifications(prev => [...prev, notification]);
  }, []);

  // Context API Implementation
  const connect = useCallback(async () => {
    if (wsManagerRef.current) {
      await wsManagerRef.current.connect();
    }
  }, []);

  const disconnect = useCallback(() => {
    if (wsManagerRef.current) {
      wsManagerRef.current.disconnect();
    }
  }, []);

  const reconnect = useCallback(async () => {
    if (wsManagerRef.current) {
      await wsManagerRef.current.reconnect();
    }
  }, []);

  const subscribe = useCallback((subscription: Omit<GraphSubscription, 'id'>): string => {
    if (!wsManagerRef.current) {
      throw new Error('WebSocket manager not initialized');
    }

    const subscriptionId = wsManagerRef.current.subscribe(subscription);
    
    setState(prev => ({
      ...prev,
      subscriptions: new Map(prev.subscriptions).set(subscriptionId, {
        ...subscription,
        id: subscriptionId,
      }),
    }));

    return subscriptionId;
  }, []);

  const unsubscribe = useCallback((subscriptionId: string) => {
    if (wsManagerRef.current) {
      wsManagerRef.current.unsubscribe(subscriptionId);
    }

    setState(prev => {
      const newSubscriptions = new Map(prev.subscriptions);
      newSubscriptions.delete(subscriptionId);
      return {
        ...prev,
        subscriptions: newSubscriptions,
      };
    });
  }, []);

  const broadcastEvent = useCallback((event: Omit<GraphEvent, 'timestamp'>) => {
    if (!wsManagerRef.current) {
      throw new Error('WebSocket manager not initialized');
    }

    wsManagerRef.current.broadcastEvent(event);
    
    setMetrics(prev => ({
      ...prev,
      messagesSent: prev.messagesSent + 1,
      lastUpdated: new Date(),
    }));
  }, []);

  const resolveConflict = useCallback(async (
    conflictEvent: ConflictDetectedEvent,
    strategy: ConflictResolutionStrategy,
    manualResolution?: any
  ): Promise<ConflictResolution> => {
    // Implement conflict resolution logic
    const resolution: ConflictResolution = {
      strategy,
      resolvedEntity: manualResolution || conflictEvent.data.conflictingChanges,
      appliedChanges: conflictEvent.data.conflictingChanges,
      timestamp: new Date(),
    };

    // Remove from pending conflicts
    setState(prev => ({
      ...prev,
      pendingConflicts: prev.pendingConflicts.filter(c => c !== conflictEvent),
    }));

    setMetrics(prev => ({
      ...prev,
      conflictsResolved: prev.conflictsResolved + 1,
    }));

    return resolution;
  }, []);

  const syncGraphState = useCallback(async () => {
    // Implement graph state synchronization
    console.log('Syncing graph state...');
    
    setState(prev => ({
      ...prev,
      lastSyncTimestamp: new Date(),
    }));
  }, []);

  const getChangeHistory = useCallback(async (since?: Date): Promise<GraphEvent[]> => {
    // Implement change history retrieval
    return state.updateQueue
      .filter(item => !since || item.timestamp >= since)
      .map(item => item.event);
  }, [state.updateQueue]);

  const enableOptimisticUpdates = useCallback((enabled: boolean) => {
    setState(prev => ({
      ...prev,
      isOptimisticUpdatesEnabled: enabled,
    }));
  }, []);

  const rollbackOptimisticUpdate = useCallback((updateId: string) => {
    setState(prev => ({
      ...prev,
      updateQueue: prev.updateQueue.filter(item => item.id !== updateId),
    }));
  }, []);

  const enableBatching = useCallback((enabled: boolean, newBatchSize?: number, timeout?: number) => {
    setState(prev => ({
      ...prev,
      isBatchingEnabled: enabled,
      batchSize: newBatchSize || prev.batchSize,
      batchTimeout: timeout || prev.batchTimeout,
    }));
  }, []);

  // Update metrics periodically
  useEffect(() => {
    const interval = setInterval(() => {
      if (wsManagerRef.current) {
        setMetrics(wsManagerRef.current.getMetrics());
      }
    }, 5000);

    return () => clearInterval(interval);
  }, []);

  // Context value
  const contextValue: RealTimeContextValue = {
    state,
    connectionStatus: state.connectionStatus,
    connect,
    disconnect,
    reconnect,
    subscribe,
    unsubscribe,
    broadcastEvent,
    resolveConflict,
    syncGraphState,
    getChangeHistory,
    enableOptimisticUpdates,
    rollbackOptimisticUpdate,
    enableBatching,
    flushBatch,
  };

  return (
    <RealTimeContext.Provider value={contextValue}>
      {children}
    </RealTimeContext.Provider>
  );
};

// Hook to use the Real-Time context
export const useRealTime = (): RealTimeContextValue => {
  const context = useContext(RealTimeContext);
  if (!context) {
    throw new Error('useRealTime must be used within a RealTimeProvider');
  }
  return context;
};

// Hook to use connection status
export const useConnectionStatus = (): ConnectionStatus => {
  const { connectionStatus } = useRealTime();
  return connectionStatus;
};

// Hook to use real-time metrics
export const useRealTimeMetrics = (): RealTimeMetrics => {
  const context = useContext(RealTimeContext);
  if (!context) {
    throw new Error('useRealTimeMetrics must be used within a RealTimeProvider');
  }
  
  const [metrics, setMetrics] = useState<RealTimeMetrics>({
    messagesReceived: 0,
    messagesSent: 0,
    eventsProcessed: 0,
    conflictsDetected: 0,
    conflictsResolved: 0,
    averageLatency: 0,
    connectionUptime: 0,
    reconnectionCount: 0,
    lastUpdated: new Date(),
  });

  useEffect(() => {
    const interval = setInterval(() => {
      // Get metrics from WebSocket manager
      setMetrics({
        messagesReceived: context.state.updateQueue.length,
        messagesSent: 0, // This would come from WebSocket manager
        eventsProcessed: context.state.updateQueue.filter(item => item.processed).length,
        conflictsDetected: context.state.pendingConflicts.length,
        conflictsResolved: 0, // This would be tracked separately
        averageLatency: context.connectionStatus.latency || 0,
        connectionUptime: context.connectionStatus.connectedAt 
          ? Date.now() - context.connectionStatus.connectedAt.getTime() 
          : 0,
        reconnectionCount: context.connectionStatus.reconnectAttempts,
        lastUpdated: new Date(),
      });
    }, 1000);

    return () => clearInterval(interval);
  }, [context]);

  return metrics;
};

export default RealTimeProvider;