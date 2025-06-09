import { useCallback, useEffect, useRef, useState } from 'react';
import { io, Socket } from 'socket.io-client';
import type {
  ConnectionState,
  ConnectionStatus,
  GraphEvent,
  GraphSubscription,
  WebSocketConfig,
  RealTimeMetrics,
} from '../../types/realtime';

// Default WebSocket configuration
const DEFAULT_CONFIG: WebSocketConfig = {
  url: 'ws://localhost:7680',
  reconnectInterval: 1000,
  maxReconnectAttempts: 5,
  heartbeatInterval: 30000,
  timeout: 10000,
  enableCompression: true,
};

export interface WebSocketManagerProps {
  config?: Partial<WebSocketConfig>;
  onConnectionChange?: (status: ConnectionStatus) => void;
  onEvent?: (event: GraphEvent) => void;
  onError?: (error: Error) => void;
}

export class WebSocketManager {
  private socket: Socket | null = null;
  private config: WebSocketConfig;
  private connectionStatus: ConnectionStatus;
  private subscriptions = new Map<string, GraphSubscription>();
  private heartbeatInterval: NodeJS.Timeout | null = null;
  private reconnectTimeout: NodeJS.Timeout | null = null;
  private metrics: RealTimeMetrics;
  
  // Event handlers
  private onConnectionChange?: (status: ConnectionStatus) => void;
  private onEvent?: (event: GraphEvent) => void;
  private onError?: (error: Error) => void;

  constructor(props: WebSocketManagerProps = {}) {
    this.config = { ...DEFAULT_CONFIG, ...props.config };
    this.onConnectionChange = props.onConnectionChange;
    this.onEvent = props.onEvent;
    this.onError = props.onError;
    
    this.connectionStatus = {
      state: 'disconnected',
      reconnectAttempts: 0,
    };

    this.metrics = {
      messagesReceived: 0,
      messagesSent: 0,
      eventsProcessed: 0,
      conflictsDetected: 0,
      conflictsResolved: 0,
      averageLatency: 0,
      connectionUptime: 0,
      reconnectionCount: 0,
      lastUpdated: new Date(),
    };
  }

  // Connection Management
  async connect(): Promise<void> {
    if (this.socket?.connected) {
      return;
    }

    try {
      this.updateConnectionStatus({ state: 'connecting' });

      this.socket = io(this.config.url, {
        timeout: this.config.timeout,
        reconnection: false, // We handle reconnection manually
        transports: ['websocket', 'polling'],
      });

      this.setupEventHandlers();
      
      return new Promise((resolve, reject) => {
        const timeout = setTimeout(() => {
          reject(new Error('Connection timeout'));
        }, this.config.timeout);

        this.socket!.on('connect', () => {
          clearTimeout(timeout);
          this.updateConnectionStatus({
            state: 'connected',
            connectedAt: new Date(),
            reconnectAttempts: 0,
          });
          this.startHeartbeat();
          resolve();
        });

        this.socket!.on('connect_error', (error) => {
          clearTimeout(timeout);
          this.handleConnectionError(error);
          reject(error);
        });
      });
    } catch (error) {
      this.handleConnectionError(error as Error);
      throw error;
    }
  }

  disconnect(): void {
    this.stopHeartbeat();
    this.clearReconnectTimeout();
    
    if (this.socket) {
      this.socket.disconnect();
      this.socket = null;
    }
    
    this.updateConnectionStatus({ state: 'disconnected' });
  }

  async reconnect(): Promise<void> {
    this.disconnect();
    
    if (this.connectionStatus.reconnectAttempts >= this.config.maxReconnectAttempts) {
      this.updateConnectionStatus({
        state: 'error',
        error: 'Maximum reconnection attempts exceeded',
      });
      return;
    }

    this.updateConnectionStatus({
      state: 'reconnecting',
      reconnectAttempts: this.connectionStatus.reconnectAttempts + 1,
    });

    const delay = Math.min(
      this.config.reconnectInterval * Math.pow(2, this.connectionStatus.reconnectAttempts),
      30000
    );

    this.reconnectTimeout = setTimeout(async () => {
      try {
        await this.connect();
        this.metrics.reconnectionCount++;
      } catch (error) {
        this.scheduleReconnect();
      }
    }, delay);
  }

  // Event Handling
  private setupEventHandlers(): void {
    if (!this.socket) return;

    this.socket.on('disconnect', (reason) => {
      this.updateConnectionStatus({
        state: 'disconnected',
        error: `Disconnected: ${reason}`,
      });
      
      if (reason === 'io server disconnect') {
        // Server initiated disconnect, don't reconnect
        return;
      }
      
      this.scheduleReconnect();
    });

    this.socket.on('error', (error) => {
      this.handleConnectionError(error);
    });

    this.socket.on('pong', () => {
      const now = Date.now();
      if (this.connectionStatus.lastHeartbeat) {
        const latency = now - this.connectionStatus.lastHeartbeat.getTime();
        this.updateConnectionStatus({ latency });
      }
    });

    // Graph event handlers
    this.socket.on('graph.event', (event: GraphEvent) => {
      this.handleGraphEvent(event);
    });

    this.socket.on('graph.node.created', (event: GraphEvent) => {
      this.handleGraphEvent(event);
    });

    this.socket.on('graph.node.updated', (event: GraphEvent) => {
      this.handleGraphEvent(event);
    });

    this.socket.on('graph.node.deleted', (event: GraphEvent) => {
      this.handleGraphEvent(event);
    });

    this.socket.on('graph.edge.created', (event: GraphEvent) => {
      this.handleGraphEvent(event);
    });

    this.socket.on('graph.edge.updated', (event: GraphEvent) => {
      this.handleGraphEvent(event);
    });

    this.socket.on('graph.edge.deleted', (event: GraphEvent) => {
      this.handleGraphEvent(event);
    });

    this.socket.on('graph.schema.updated', (event: GraphEvent) => {
      this.handleGraphEvent(event);
    });

    this.socket.on('graph.bulk.operation', (event: GraphEvent) => {
      this.handleGraphEvent(event);
    });

    this.socket.on('graph.conflict.detected', (event: GraphEvent) => {
      this.handleGraphEvent(event);
      this.metrics.conflictsDetected++;
    });

    this.socket.on('graph.sync.required', (event: GraphEvent) => {
      this.handleGraphEvent(event);
    });
  }

  private handleGraphEvent(event: GraphEvent): void {
    this.metrics.messagesReceived++;
    this.metrics.eventsProcessed++;
    this.metrics.lastUpdated = new Date();

    // Notify subscriptions
    this.subscriptions.forEach((subscription) => {
      if (this.shouldNotifySubscription(subscription, event)) {
        try {
          subscription.callback(event);
        } catch (error) {
          console.error('Error in subscription callback:', error);
        }
      }
    });

    // Notify global event handler
    if (this.onEvent) {
      this.onEvent(event);
    }
  }

  private shouldNotifySubscription(subscription: GraphSubscription, event: GraphEvent): boolean {
    // Check event type filter
    if (!subscription.eventTypes.includes(event.type)) {
      return false;
    }

    // Check additional filters
    if (subscription.filters) {
      const { filters } = subscription;
      
      // User filter
      if (filters.userId && 'userId' in event && event.userId !== filters.userId) {
        return false;
      }

      // Node/Edge ID filters
      if (event.type.includes('node')) {
        const nodeEvent = event as any;
        if (filters.nodeIds && nodeEvent.data?.nodeId && 
            !filters.nodeIds.includes(nodeEvent.data.nodeId)) {
          return false;
        }
      }

      if (event.type.includes('edge')) {
        const edgeEvent = event as any;
        if (filters.edgeIds && edgeEvent.data?.edgeId && 
            !filters.edgeIds.includes(edgeEvent.data.edgeId)) {
          return false;
        }
      }

      // Type filters
      if (event.type.includes('node') && filters.nodeTypes) {
        const nodeEvent = event as any;
        if (nodeEvent.data?.node?.node_type && 
            !filters.nodeTypes.includes(nodeEvent.data.node.node_type)) {
          return false;
        }
      }

      if (event.type.includes('edge') && filters.edgeTypes) {
        const edgeEvent = event as any;
        if (edgeEvent.data?.edge?.edge_type && 
            !filters.edgeTypes.includes(edgeEvent.data.edge.edge_type)) {
          return false;
        }
      }
    }

    return true;
  }

  private handleConnectionError(error: Error): void {
    this.updateConnectionStatus({
      state: 'error',
      error: error.message,
    });

    if (this.onError) {
      this.onError(error);
    }

    this.scheduleReconnect();
  }

  private scheduleReconnect(): void {
    if (this.connectionStatus.reconnectAttempts < this.config.maxReconnectAttempts) {
      setTimeout(() => {
        this.reconnect();
      }, this.config.reconnectInterval);
    }
  }

  // Heartbeat Management
  private startHeartbeat(): void {
    this.stopHeartbeat();
    
    this.heartbeatInterval = setInterval(() => {
      if (this.socket?.connected) {
        this.updateConnectionStatus({ lastHeartbeat: new Date() });
        this.socket.emit('ping');
      }
    }, this.config.heartbeatInterval);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval);
      this.heartbeatInterval = null;
    }
  }

  private clearReconnectTimeout(): void {
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }
  }

  // Subscription Management
  subscribe(subscription: Omit<GraphSubscription, 'id'>): string {
    const id = `sub_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    this.subscriptions.set(id, { ...subscription, id });
    
    // Send subscription to server
    if (this.socket?.connected) {
      this.socket.emit('subscribe', {
        subscriptionId: id,
        eventTypes: subscription.eventTypes,
        filters: subscription.filters,
      });
    }
    
    return id;
  }

  unsubscribe(subscriptionId: string): void {
    this.subscriptions.delete(subscriptionId);
    
    // Send unsubscription to server
    if (this.socket?.connected) {
      this.socket.emit('unsubscribe', { subscriptionId });
    }
  }

  // Event Broadcasting
  broadcastEvent(event: Omit<GraphEvent, 'timestamp'>): void {
    if (!this.socket?.connected) {
      throw new Error('WebSocket not connected');
    }

    const fullEvent: GraphEvent = {
      ...event,
      timestamp: new Date().toISOString(),
    } as GraphEvent;

    this.socket.emit('graph.event', fullEvent);
    this.metrics.messagesSent++;
    this.metrics.lastUpdated = new Date();
  }

  // Status Management
  private updateConnectionStatus(updates: Partial<ConnectionStatus>): void {
    this.connectionStatus = { ...this.connectionStatus, ...updates };
    
    if (this.onConnectionChange) {
      this.onConnectionChange(this.connectionStatus);
    }
  }

  // Getters
  getConnectionStatus(): ConnectionStatus {
    return { ...this.connectionStatus };
  }

  getMetrics(): RealTimeMetrics {
    // Update uptime
    if (this.connectionStatus.connectedAt && this.connectionStatus.state === 'connected') {
      this.metrics.connectionUptime = Date.now() - this.connectionStatus.connectedAt.getTime();
    }
    
    return { ...this.metrics };
  }

  isConnected(): boolean {
    return this.socket?.connected ?? false;
  }

  // Cleanup
  destroy(): void {
    this.disconnect();
    this.subscriptions.clear();
  }
}

export default WebSocketManager;