import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useSnackbar } from 'notistack';
import { useRealTime } from './RealTimeProvider';
import type {
  GraphEvent,
  NodeCreatedEvent,
  NodeUpdatedEvent,
  NodeDeletedEvent,
  EdgeCreatedEvent,
  EdgeUpdatedEvent,
  EdgeDeletedEvent,
  SchemaUpdatedEvent,
  BulkOperationEvent,
  UpdateQueueItem,
} from '../../types/realtime';
import type { NodeResponse, EdgeResponse, NodeId, EdgeId } from '../../types/graph';

export interface GraphUpdateHandlerProps {
  onNodeCreated?: (node: NodeResponse) => void;
  onNodeUpdated?: (nodeId: NodeId, node: NodeResponse, changes: Record<string, any>) => void;
  onNodeDeleted?: (nodeId: NodeId, affectedEdges: EdgeId[]) => void;
  onEdgeCreated?: (edge: EdgeResponse) => void;
  onEdgeUpdated?: (edgeId: EdgeId, edge: EdgeResponse, changes: Record<string, any>) => void;
  onEdgeDeleted?: (edgeId: EdgeId, sourceId: NodeId, targetId: NodeId) => void;
  onSchemaUpdated?: (schemaChanges: Record<string, any>) => void;
  onBulkOperation?: (operation: BulkOperationEvent['data']) => void;
  enableOptimisticUpdates?: boolean;
  enableBatching?: boolean;
  batchSize?: number;
  batchTimeout?: number;
}

interface OptimisticUpdate {
  id: string;
  type: 'node' | 'edge';
  operation: 'create' | 'update' | 'delete';
  entityId: NodeId | EdgeId;
  originalData?: any;
  newData?: any;
  timestamp: Date;
  confirmed: boolean;
}

const GraphUpdateHandler: React.FC<GraphUpdateHandlerProps> = ({
  onNodeCreated,
  onNodeUpdated,
  onNodeDeleted,
  onEdgeCreated,
  onEdgeUpdated,
  onEdgeDeleted,
  onSchemaUpdated,
  onBulkOperation,
  enableOptimisticUpdates = true,
  enableBatching = true,
  batchSize = 10,
  batchTimeout = 1000,
}) => {
  const { enqueueSnackbar } = useSnackbar();
  const { subscribe, unsubscribe, state } = useRealTime();
  
  // Optimistic updates tracking
  const [optimisticUpdates, setOptimisticUpdates] = useState<Map<string, OptimisticUpdate>>(new Map());
  const optimisticTimeoutRef = useRef<Map<string, NodeJS.Timeout>>(new Map());
  
  // Batch processing
  const [pendingUpdates, setPendingUpdates] = useState<GraphEvent[]>([]);
  const batchTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  
  // Update processing queue
  const processingQueue = useRef<UpdateQueueItem[]>([]);
  const isProcessing = useRef(false);

  // Subscribe to all graph events
  useEffect(() => {
    const subscriptionId = subscribe({
      eventTypes: [
        'graph.node.created',
        'graph.node.updated',
        'graph.node.deleted',
        'graph.edge.created',
        'graph.edge.updated',
        'graph.edge.deleted',
        'graph.schema.updated',
        'graph.bulk.operation',
      ],
      callback: handleGraphEvent,
    });

    return () => {
      unsubscribe(subscriptionId);
    };
  }, []);

  // Handle incoming graph events
  const handleGraphEvent = useCallback((event: GraphEvent) => {
    if (enableBatching) {
      addToBatch(event);
    } else {
      processEvent(event);
    }
  }, [enableBatching, batchSize, batchTimeout]);

  // Batch processing
  const addToBatch = useCallback((event: GraphEvent) => {
    setPendingUpdates(prev => [...prev, event]);

    if (pendingUpdates.length + 1 >= batchSize) {
      flushBatch();
    } else if (!batchTimeoutRef.current) {
      batchTimeoutRef.current = setTimeout(flushBatch, batchTimeout);
    }
  }, [pendingUpdates.length, batchSize, batchTimeout]);

  const flushBatch = useCallback(() => {
    if (batchTimeoutRef.current) {
      clearTimeout(batchTimeoutRef.current);
      batchTimeoutRef.current = null;
    }

    const events = [...pendingUpdates];
    setPendingUpdates([]);

    // Group events by type for efficient processing
    const groupedEvents = events.reduce((groups, event) => {
      const type = event.type;
      if (!groups[type]) {
        groups[type] = [];
      }
      groups[type].push(event);
      return groups;
    }, {} as Record<string, GraphEvent[]>);

    // Process each group
    Object.entries(groupedEvents).forEach(([type, eventGroup]) => {
      processBatchedEvents(type, eventGroup);
    });
  }, [pendingUpdates]);

  const processBatchedEvents = useCallback((eventType: string, events: GraphEvent[]) => {
    switch (eventType) {
      case 'graph.node.created':
        events.forEach(event => processNodeCreated(event as NodeCreatedEvent));
        break;
      case 'graph.node.updated':
        events.forEach(event => processNodeUpdated(event as NodeUpdatedEvent));
        break;
      case 'graph.node.deleted':
        events.forEach(event => processNodeDeleted(event as NodeDeletedEvent));
        break;
      case 'graph.edge.created':
        events.forEach(event => processEdgeCreated(event as EdgeCreatedEvent));
        break;
      case 'graph.edge.updated':
        events.forEach(event => processEdgeUpdated(event as EdgeUpdatedEvent));
        break;
      case 'graph.edge.deleted':
        events.forEach(event => processEdgeDeleted(event as EdgeDeletedEvent));
        break;
      case 'graph.schema.updated':
        events.forEach(event => processSchemaUpdated(event as SchemaUpdatedEvent));
        break;
      case 'graph.bulk.operation':
        events.forEach(event => processBulkOperation(event as BulkOperationEvent));
        break;
    }
  }, []);

  // Individual event processing
  const processEvent = useCallback((event: GraphEvent) => {
    switch (event.type) {
      case 'graph.node.created':
        processNodeCreated(event as NodeCreatedEvent);
        break;
      case 'graph.node.updated':
        processNodeUpdated(event as NodeUpdatedEvent);
        break;
      case 'graph.node.deleted':
        processNodeDeleted(event as NodeDeletedEvent);
        break;
      case 'graph.edge.created':
        processEdgeCreated(event as EdgeCreatedEvent);
        break;
      case 'graph.edge.updated':
        processEdgeUpdated(event as EdgeUpdatedEvent);
        break;
      case 'graph.edge.deleted':
        processEdgeDeleted(event as EdgeDeletedEvent);
        break;
      case 'graph.schema.updated':
        processSchemaUpdated(event as SchemaUpdatedEvent);
        break;
      case 'graph.bulk.operation':
        processBulkOperation(event as BulkOperationEvent);
        break;
    }
  }, []);

  // Node event processors
  const processNodeCreated = useCallback((event: NodeCreatedEvent) => {
    const { node } = event.data;
    
    // Check for optimistic update confirmation
    const optimisticUpdate = findOptimisticUpdate('node', 'create', node.id);
    if (optimisticUpdate) {
      confirmOptimisticUpdate(optimisticUpdate.id);
    }

    if (onNodeCreated) {
      onNodeCreated(node);
    }

    enqueueSnackbar(`Node created: ${node.node_type}`, { variant: 'success' });
  }, [onNodeCreated, enqueueSnackbar]);

  const processNodeUpdated = useCallback((event: NodeUpdatedEvent) => {
    const { nodeId, node, changes } = event.data;
    
    // Check for optimistic update confirmation
    const optimisticUpdate = findOptimisticUpdate('node', 'update', nodeId);
    if (optimisticUpdate) {
      confirmOptimisticUpdate(optimisticUpdate.id);
    }

    if (onNodeUpdated) {
      onNodeUpdated(nodeId, node, changes);
    }

    enqueueSnackbar(`Node updated: ${nodeId}`, { variant: 'info' });
  }, [onNodeUpdated, enqueueSnackbar]);

  const processNodeDeleted = useCallback((event: NodeDeletedEvent) => {
    const { nodeId, affectedEdges } = event.data;
    
    // Check for optimistic update confirmation
    const optimisticUpdate = findOptimisticUpdate('node', 'delete', nodeId);
    if (optimisticUpdate) {
      confirmOptimisticUpdate(optimisticUpdate.id);
    }

    if (onNodeDeleted) {
      onNodeDeleted(nodeId, affectedEdges);
    }

    enqueueSnackbar(`Node deleted: ${nodeId}`, { variant: 'warning' });
  }, [onNodeDeleted, enqueueSnackbar]);

  // Edge event processors
  const processEdgeCreated = useCallback((event: EdgeCreatedEvent) => {
    const { edge } = event.data;
    
    // Check for optimistic update confirmation
    const optimisticUpdate = findOptimisticUpdate('edge', 'create', edge.id);
    if (optimisticUpdate) {
      confirmOptimisticUpdate(optimisticUpdate.id);
    }

    if (onEdgeCreated) {
      onEdgeCreated(edge);
    }

    enqueueSnackbar(`Edge created: ${edge.edge_type}`, { variant: 'success' });
  }, [onEdgeCreated, enqueueSnackbar]);

  const processEdgeUpdated = useCallback((event: EdgeUpdatedEvent) => {
    const { edgeId, edge, changes } = event.data;
    
    // Check for optimistic update confirmation
    const optimisticUpdate = findOptimisticUpdate('edge', 'update', edgeId);
    if (optimisticUpdate) {
      confirmOptimisticUpdate(optimisticUpdate.id);
    }

    if (onEdgeUpdated) {
      onEdgeUpdated(edgeId, edge, changes);
    }

    enqueueSnackbar(`Edge updated: ${edgeId}`, { variant: 'info' });
  }, [onEdgeUpdated, enqueueSnackbar]);

  const processEdgeDeleted = useCallback((event: EdgeDeletedEvent) => {
    const { edgeId, sourceId, targetId } = event.data;
    
    // Check for optimistic update confirmation
    const optimisticUpdate = findOptimisticUpdate('edge', 'delete', edgeId);
    if (optimisticUpdate) {
      confirmOptimisticUpdate(optimisticUpdate.id);
    }

    if (onEdgeDeleted) {
      onEdgeDeleted(edgeId, sourceId, targetId);
    }

    enqueueSnackbar(`Edge deleted: ${edgeId}`, { variant: 'warning' });
  }, [onEdgeDeleted, enqueueSnackbar]);

  // Schema and bulk operation processors
  const processSchemaUpdated = useCallback((event: SchemaUpdatedEvent) => {
    const { schemaChanges } = event.data;

    if (onSchemaUpdated) {
      onSchemaUpdated(schemaChanges);
    }

    enqueueSnackbar('Graph schema updated', { variant: 'info' });
  }, [onSchemaUpdated, enqueueSnackbar]);

  const processBulkOperation = useCallback((event: BulkOperationEvent) => {
    const { data } = event;

    if (onBulkOperation) {
      onBulkOperation(data);
    }

    enqueueSnackbar(
      `Bulk ${data.operationType}: ${data.count} ${data.entityType}`,
      { variant: 'info' }
    );
  }, [onBulkOperation, enqueueSnackbar]);

  // Optimistic updates management
  const createOptimisticUpdate = useCallback((
    type: 'node' | 'edge',
    operation: 'create' | 'update' | 'delete',
    entityId: NodeId | EdgeId,
    originalData?: any,
    newData?: any
  ): string => {
    const id = `optimistic_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    
    const update: OptimisticUpdate = {
      id,
      type,
      operation,
      entityId,
      originalData,
      newData,
      timestamp: new Date(),
      confirmed: false,
    };

    setOptimisticUpdates(prev => new Map(prev).set(id, update));

    // Set timeout for rollback if not confirmed
    const timeout = setTimeout(() => {
      rollbackOptimisticUpdate(id);
    }, 10000); // 10 second timeout

    optimisticTimeoutRef.current.set(id, timeout);

    return id;
  }, []);

  const confirmOptimisticUpdate = useCallback((updateId: string) => {
    setOptimisticUpdates(prev => {
      const newMap = new Map(prev);
      const update = newMap.get(updateId);
      if (update) {
        newMap.set(updateId, { ...update, confirmed: true });
      }
      return newMap;
    });

    // Clear timeout
    const timeout = optimisticTimeoutRef.current.get(updateId);
    if (timeout) {
      clearTimeout(timeout);
      optimisticTimeoutRef.current.delete(updateId);
    }

    // Remove confirmed update after a delay
    setTimeout(() => {
      setOptimisticUpdates(prev => {
        const newMap = new Map(prev);
        newMap.delete(updateId);
        return newMap;
      });
    }, 1000);
  }, []);

  const rollbackOptimisticUpdate = useCallback((updateId: string) => {
    const update = optimisticUpdates.get(updateId);
    if (!update || update.confirmed) {
      return;
    }

    // Apply rollback based on operation type
    switch (update.operation) {
      case 'create':
        // Remove the optimistically created entity
        if (update.type === 'node' && onNodeDeleted) {
          onNodeDeleted(update.entityId as NodeId, []);
        } else if (update.type === 'edge' && onEdgeDeleted) {
          const edge = update.newData as EdgeResponse;
          onEdgeDeleted(update.entityId as EdgeId, edge.source_id, edge.target_id);
        }
        break;
      case 'update':
        // Restore original data
        if (update.type === 'node' && onNodeUpdated && update.originalData) {
          onNodeUpdated(update.entityId as NodeId, update.originalData, {});
        } else if (update.type === 'edge' && onEdgeUpdated && update.originalData) {
          onEdgeUpdated(update.entityId as EdgeId, update.originalData, {});
        }
        break;
      case 'delete':
        // Restore the deleted entity
        if (update.type === 'node' && onNodeCreated && update.originalData) {
          onNodeCreated(update.originalData);
        } else if (update.type === 'edge' && onEdgeCreated && update.originalData) {
          onEdgeCreated(update.originalData);
        }
        break;
    }

    // Remove the optimistic update
    setOptimisticUpdates(prev => {
      const newMap = new Map(prev);
      newMap.delete(updateId);
      return newMap;
    });

    // Clear timeout
    const timeout = optimisticTimeoutRef.current.get(updateId);
    if (timeout) {
      clearTimeout(timeout);
      optimisticTimeoutRef.current.delete(updateId);
    }

    enqueueSnackbar('Optimistic update rolled back', { variant: 'warning' });
  }, [optimisticUpdates, onNodeCreated, onNodeUpdated, onNodeDeleted, onEdgeCreated, onEdgeUpdated, onEdgeDeleted, enqueueSnackbar]);

  const findOptimisticUpdate = useCallback((
    type: 'node' | 'edge',
    operation: 'create' | 'update' | 'delete',
    entityId: NodeId | EdgeId
  ): OptimisticUpdate | undefined => {
    for (const update of optimisticUpdates.values()) {
      if (update.type === type && update.operation === operation && update.entityId === entityId) {
        return update;
      }
    }
    return undefined;
  }, [optimisticUpdates]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      // Clear all timeouts
      optimisticTimeoutRef.current.forEach(timeout => clearTimeout(timeout));
      optimisticTimeoutRef.current.clear();
      
      if (batchTimeoutRef.current) {
        clearTimeout(batchTimeoutRef.current);
      }
    };
  }, []);

  // Expose optimistic update functions
  const applyOptimisticUpdate = useCallback((
    type: 'node' | 'edge',
    operation: 'create' | 'update' | 'delete',
    entityId: NodeId | EdgeId,
    originalData?: any,
    newData?: any
  ) => {
    if (!enableOptimisticUpdates) {
      return null;
    }

    return createOptimisticUpdate(type, operation, entityId, originalData, newData);
  }, [enableOptimisticUpdates, createOptimisticUpdate]);

  // This component doesn't render anything, it just handles updates
  return null;
};

export default GraphUpdateHandler;

// Export the optimistic update functions for use by other components
export { GraphUpdateHandler };