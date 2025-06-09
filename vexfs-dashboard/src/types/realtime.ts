// Real-Time Graph Updates Type Definitions

import type { NodeId, EdgeId, NodeResponse, EdgeResponse } from './graph';

// WebSocket Connection States
export type ConnectionState = 'connecting' | 'connected' | 'disconnected' | 'reconnecting' | 'error';

// Real-Time Event Types
export type GraphEventType =
  | 'graph.node.created'
  | 'graph.node.updated'
  | 'graph.node.deleted'
  | 'graph.edge.created'
  | 'graph.edge.updated'
  | 'graph.edge.deleted'
  | 'graph.schema.updated'
  | 'graph.bulk.operation'
  | 'graph.query.result'
  | 'graph.conflict.detected'
  | 'graph.sync.required';

// Event Payloads
export interface NodeCreatedEvent {
  type: 'graph.node.created';
  timestamp: string;
  userId?: string;
  data: {
    node: NodeResponse;
  };
}

export interface NodeUpdatedEvent {
  type: 'graph.node.updated';
  timestamp: string;
  userId?: string;
  data: {
    nodeId: NodeId;
    node: NodeResponse;
    previousVersion?: number;
    changes: Record<string, any>;
  };
}

export interface NodeDeletedEvent {
  type: 'graph.node.deleted';
  timestamp: string;
  userId?: string;
  data: {
    nodeId: NodeId;
    affectedEdges: EdgeId[];
  };
}

export interface EdgeCreatedEvent {
  type: 'graph.edge.created';
  timestamp: string;
  userId?: string;
  data: {
    edge: EdgeResponse;
  };
}

export interface EdgeUpdatedEvent {
  type: 'graph.edge.updated';
  timestamp: string;
  userId?: string;
  data: {
    edgeId: EdgeId;
    edge: EdgeResponse;
    previousVersion?: number;
    changes: Record<string, any>;
  };
}

export interface EdgeDeletedEvent {
  type: 'graph.edge.deleted';
  timestamp: string;
  userId?: string;
  data: {
    edgeId: EdgeId;
    sourceId: NodeId;
    targetId: NodeId;
  };
}

export interface SchemaUpdatedEvent {
  type: 'graph.schema.updated';
  timestamp: string;
  userId?: string;
  data: {
    schemaChanges: Record<string, any>;
    affectedNodes: NodeId[];
    affectedEdges: EdgeId[];
  };
}

export interface BulkOperationEvent {
  type: 'graph.bulk.operation';
  timestamp: string;
  userId?: string;
  data: {
    operationType: 'create' | 'update' | 'delete';
    entityType: 'nodes' | 'edges';
    count: number;
    affectedIds: (NodeId | EdgeId)[];
    summary: string;
  };
}

export interface QueryResultEvent {
  type: 'graph.query.result';
  timestamp: string;
  userId?: string;
  data: {
    queryId: string;
    results: any[];
    executionTime: number;
  };
}

export interface ConflictDetectedEvent {
  type: 'graph.conflict.detected';
  timestamp: string;
  userId?: string;
  data: {
    entityType: 'node' | 'edge';
    entityId: NodeId | EdgeId;
    conflictType: 'concurrent_modification' | 'version_mismatch';
    localVersion: number;
    remoteVersion: number;
    conflictingChanges: Record<string, any>;
  };
}

export interface SyncRequiredEvent {
  type: 'graph.sync.required';
  timestamp: string;
  data: {
    reason: 'connection_restored' | 'conflict_resolved' | 'manual_request';
    affectedEntities: {
      nodes: NodeId[];
      edges: EdgeId[];
    };
  };
}

// Union type for all graph events
export type GraphEvent =
  | NodeCreatedEvent
  | NodeUpdatedEvent
  | NodeDeletedEvent
  | EdgeCreatedEvent
  | EdgeUpdatedEvent
  | EdgeDeletedEvent
  | SchemaUpdatedEvent
  | BulkOperationEvent
  | QueryResultEvent
  | ConflictDetectedEvent
  | SyncRequiredEvent;

// WebSocket Configuration
export interface WebSocketConfig {
  url: string;
  reconnectInterval: number;
  maxReconnectAttempts: number;
  heartbeatInterval: number;
  timeout: number;
  enableCompression: boolean;
}

// Connection Status
export interface ConnectionStatus {
  state: ConnectionState;
  connectedAt?: Date;
  lastHeartbeat?: Date;
  reconnectAttempts: number;
  latency?: number;
  error?: string;
}

// Real-Time Subscription
export interface GraphSubscription {
  id: string;
  eventTypes: GraphEventType[];
  filters?: {
    nodeTypes?: string[];
    edgeTypes?: string[];
    nodeIds?: NodeId[];
    edgeIds?: EdgeId[];
    userId?: string;
  };
  callback: (event: GraphEvent) => void;
}

// Update Queue Item
export interface UpdateQueueItem {
  id: string;
  event: GraphEvent;
  timestamp: Date;
  retryCount: number;
  processed: boolean;
}

// Conflict Resolution Strategy
export type ConflictResolutionStrategy = 
  | 'local_wins'
  | 'remote_wins'
  | 'merge'
  | 'manual'
  | 'latest_timestamp';

// Conflict Resolution Result
export interface ConflictResolution {
  strategy: ConflictResolutionStrategy;
  resolvedEntity: NodeResponse | EdgeResponse;
  appliedChanges: Record<string, any>;
  timestamp: Date;
}

// Real-Time State
export interface RealTimeState {
  connectionStatus: ConnectionStatus;
  subscriptions: Map<string, GraphSubscription>;
  updateQueue: UpdateQueueItem[];
  pendingConflicts: ConflictDetectedEvent[];
  lastSyncTimestamp?: Date;
  isOptimisticUpdatesEnabled: boolean;
  isBatchingEnabled: boolean;
  batchSize: number;
  batchTimeout: number;
}

// Real-Time Context
export interface RealTimeContextValue {
  state: RealTimeState;
  connectionStatus: ConnectionStatus;
  
  // Connection Management
  connect: () => Promise<void>;
  disconnect: () => void;
  reconnect: () => Promise<void>;
  
  // Subscription Management
  subscribe: (subscription: Omit<GraphSubscription, 'id'>) => string;
  unsubscribe: (subscriptionId: string) => void;
  
  // Event Broadcasting
  broadcastEvent: (event: Omit<GraphEvent, 'timestamp'>) => void;
  
  // Conflict Resolution
  resolveConflict: (
    conflictEvent: ConflictDetectedEvent,
    strategy: ConflictResolutionStrategy,
    manualResolution?: any
  ) => Promise<ConflictResolution>;
  
  // Synchronization
  syncGraphState: () => Promise<void>;
  getChangeHistory: (since?: Date) => Promise<GraphEvent[]>;
  
  // Optimistic Updates
  enableOptimisticUpdates: (enabled: boolean) => void;
  rollbackOptimisticUpdate: (updateId: string) => void;
  
  // Batching
  enableBatching: (enabled: boolean, batchSize?: number, timeout?: number) => void;
  flushBatch: () => void;
}

// Real-Time Notification
export interface RealTimeNotification {
  id: string;
  type: 'info' | 'success' | 'warning' | 'error';
  title: string;
  message: string;
  event?: GraphEvent;
  timestamp: Date;
  autoHide?: boolean;
  duration?: number;
  actions?: Array<{
    label: string;
    action: () => void;
  }>;
}

// Performance Metrics
export interface RealTimeMetrics {
  messagesReceived: number;
  messagesSent: number;
  eventsProcessed: number;
  conflictsDetected: number;
  conflictsResolved: number;
  averageLatency: number;
  connectionUptime: number;
  reconnectionCount: number;
  lastUpdated: Date;
}

// All types are already exported above