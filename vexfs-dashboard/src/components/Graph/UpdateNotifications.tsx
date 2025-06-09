import React, { useState, useCallback, useEffect } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  IconButton,
  Chip,
  Stack,
  Collapse,
  Button,
  Badge,
  Tooltip,
  Drawer,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  ListItemSecondaryAction,
  Divider,
  Alert,
} from '@mui/material';
import {
  Notifications as NotificationsIcon,
  Close as CloseIcon,
  ExpandMore as ExpandMoreIcon,
  ExpandLess as ExpandLessIcon,
  Clear as ClearIcon,
  Settings as SettingsIcon,
  Add as AddIcon,
  Edit as EditIcon,
  Delete as DeleteIcon,
  Schema as SchemaIcon,
  Merge as MergeIcon,
  Sync as SyncIcon,
  Group as GroupIcon,
  Person as PersonIcon,
  Schedule as ScheduleIcon,
} from '@mui/icons-material';
import { formatDistanceToNow } from 'date-fns';
import { useRealTime } from './RealTimeProvider';
import type {
  GraphEvent,
  RealTimeNotification,
  GraphEventType,
} from '../../types/realtime';

export interface UpdateNotificationsProps {
  maxNotifications?: number;
  autoHideDelay?: number;
  showUserInfo?: boolean;
  enableGrouping?: boolean;
  enableFiltering?: boolean;
}

interface NotificationGroup {
  type: GraphEventType;
  count: number;
  latestEvent: GraphEvent;
  events: GraphEvent[];
  expanded: boolean;
}

const UpdateNotifications: React.FC<UpdateNotificationsProps> = ({
  maxNotifications = 50,
  autoHideDelay = 5000,
  showUserInfo = true,
  enableGrouping = true,
  enableFiltering = false,
}) => {
  const { state, subscribe, unsubscribe } = useRealTime();
  const [notifications, setNotifications] = useState<RealTimeNotification[]>([]);
  const [events, setEvents] = useState<GraphEvent[]>([]);
  const [groupedEvents, setGroupedEvents] = useState<Map<GraphEventType, NotificationGroup>>(new Map());
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [expandedGroups, setExpandedGroups] = useState<Set<GraphEventType>>(new Set());
  const [filteredTypes, setFilteredTypes] = useState<Set<GraphEventType>>(new Set());

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
        'graph.conflict.detected',
        'graph.sync.required',
      ],
      callback: handleGraphEvent,
    });

    return () => {
      unsubscribe(subscriptionId);
    };
  }, []);

  // Handle incoming graph events
  const handleGraphEvent = useCallback((event: GraphEvent) => {
    // Add to events list
    setEvents(prev => {
      const newEvents = [event, ...prev].slice(0, maxNotifications);
      return newEvents;
    });

    // Create notification
    const notification = createNotificationFromEvent(event);
    if (notification) {
      setNotifications(prev => {
        const newNotifications = [notification, ...prev].slice(0, maxNotifications);
        return newNotifications;
      });

      // Auto-hide notification
      if (notification.autoHide) {
        setTimeout(() => {
          removeNotification(notification.id);
        }, notification.duration || autoHideDelay);
      }
    }

    // Update grouped events
    if (enableGrouping) {
      updateGroupedEvents(event);
    }
  }, [maxNotifications, autoHideDelay, enableGrouping]);

  // Create notification from graph event
  const createNotificationFromEvent = useCallback((event: GraphEvent): RealTimeNotification | null => {
    const baseNotification = {
      id: `notif_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      timestamp: new Date(),
      event,
      autoHide: true,
      duration: autoHideDelay,
    };

    switch (event.type) {
      case 'graph.node.created':
        return {
          ...baseNotification,
          type: 'success',
          title: 'Node Created',
          message: `New ${(event as any).data.node.node_type} node added`,
        };
      case 'graph.node.updated':
        return {
          ...baseNotification,
          type: 'info',
          title: 'Node Updated',
          message: `Node ${(event as any).data.nodeId} modified`,
        };
      case 'graph.node.deleted':
        return {
          ...baseNotification,
          type: 'warning',
          title: 'Node Deleted',
          message: `Node ${(event as any).data.nodeId} removed`,
        };
      case 'graph.edge.created':
        return {
          ...baseNotification,
          type: 'success',
          title: 'Edge Created',
          message: `New ${(event as any).data.edge.edge_type} connection added`,
        };
      case 'graph.edge.updated':
        return {
          ...baseNotification,
          type: 'info',
          title: 'Edge Updated',
          message: `Edge ${(event as any).data.edgeId} modified`,
        };
      case 'graph.edge.deleted':
        return {
          ...baseNotification,
          type: 'warning',
          title: 'Edge Deleted',
          message: `Edge ${(event as any).data.edgeId} removed`,
        };
      case 'graph.schema.updated':
        return {
          ...baseNotification,
          type: 'info',
          title: 'Schema Updated',
          message: 'Graph schema has been modified',
        };
      case 'graph.bulk.operation':
        const bulkData = (event as any).data;
        return {
          ...baseNotification,
          type: 'info',
          title: 'Bulk Operation',
          message: `${bulkData.operationType} ${bulkData.count} ${bulkData.entityType}`,
        };
      case 'graph.conflict.detected':
        return {
          ...baseNotification,
          type: 'error',
          title: 'Conflict Detected',
          message: `Concurrent modification on ${(event as any).data.entityType}`,
          autoHide: false,
          actions: [
            {
              label: 'Resolve',
              action: () => {
                // Open conflict resolution dialog
                console.log('Opening conflict resolution');
              },
            },
          ],
        };
      case 'graph.sync.required':
        return {
          ...baseNotification,
          type: 'warning',
          title: 'Sync Required',
          message: `Graph synchronization needed: ${(event as any).data.reason}`,
          autoHide: false,
          actions: [
            {
              label: 'Sync Now',
              action: () => {
                // Trigger sync
                console.log('Triggering sync');
              },
            },
          ],
        };
      default:
        return null;
    }
  }, [autoHideDelay]);

  // Update grouped events
  const updateGroupedEvents = useCallback((event: GraphEvent) => {
    setGroupedEvents(prev => {
      const newGroups = new Map(prev);
      const existing = newGroups.get(event.type);
      
      if (existing) {
        newGroups.set(event.type, {
          ...existing,
          count: existing.count + 1,
          latestEvent: event,
          events: [event, ...existing.events].slice(0, 10), // Keep last 10 events per type
        });
      } else {
        newGroups.set(event.type, {
          type: event.type,
          count: 1,
          latestEvent: event,
          events: [event],
          expanded: false,
        });
      }
      
      return newGroups;
    });
  }, []);

  // Notification management
  const removeNotification = useCallback((notificationId: string) => {
    setNotifications(prev => prev.filter(n => n.id !== notificationId));
  }, []);

  const clearAllNotifications = useCallback(() => {
    setNotifications([]);
  }, []);

  const clearAllEvents = useCallback(() => {
    setEvents([]);
    setGroupedEvents(new Map());
  }, []);

  // Group management
  const toggleGroupExpansion = useCallback((eventType: GraphEventType) => {
    setExpandedGroups(prev => {
      const newSet = new Set(prev);
      if (newSet.has(eventType)) {
        newSet.delete(eventType);
      } else {
        newSet.add(eventType);
      }
      return newSet;
    });
  }, []);

  // Event type filtering
  const toggleEventTypeFilter = useCallback((eventType: GraphEventType) => {
    setFilteredTypes(prev => {
      const newSet = new Set(prev);
      if (newSet.has(eventType)) {
        newSet.delete(eventType);
      } else {
        newSet.add(eventType);
      }
      return newSet;
    });
  }, []);

  // Get icon for event type
  const getEventIcon = (eventType: GraphEventType) => {
    switch (eventType) {
      case 'graph.node.created':
      case 'graph.edge.created':
        return <AddIcon />;
      case 'graph.node.updated':
      case 'graph.edge.updated':
        return <EditIcon />;
      case 'graph.node.deleted':
      case 'graph.edge.deleted':
        return <DeleteIcon />;
      case 'graph.schema.updated':
        return <SchemaIcon />;
      case 'graph.bulk.operation':
        return <GroupIcon />;
      case 'graph.conflict.detected':
        return <MergeIcon />;
      case 'graph.sync.required':
        return <SyncIcon />;
      default:
        return <NotificationsIcon />;
    }
  };

  // Get color for event type
  const getEventColor = (eventType: GraphEventType): 'success' | 'info' | 'warning' | 'error' => {
    if (eventType.includes('created')) return 'success';
    if (eventType.includes('updated')) return 'info';
    if (eventType.includes('deleted')) return 'warning';
    if (eventType.includes('conflict')) return 'error';
    return 'info';
  };

  // Filter events based on selected filters
  const filteredEvents = events.filter(event => 
    filteredTypes.size === 0 || !filteredTypes.has(event.type)
  );

  const unreadCount = notifications.filter(n => n.autoHide).length;

  return (
    <>
      {/* Notification Bell */}
      <Tooltip title="Real-time Updates">
        <IconButton
          onClick={() => setDrawerOpen(true)}
          color="inherit"
        >
          <Badge badgeContent={unreadCount} color="error">
            <NotificationsIcon />
          </Badge>
        </IconButton>
      </Tooltip>

      {/* Notifications Drawer */}
      <Drawer
        anchor="right"
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        PaperProps={{
          sx: { width: 400 }
        }}
      >
        <Box sx={{ p: 2 }}>
          <Box display="flex" alignItems="center" justifyContent="space-between" mb={2}>
            <Typography variant="h6">
              Real-time Updates
            </Typography>
            <Box>
              <Tooltip title="Clear All">
                <IconButton size="small" onClick={clearAllEvents}>
                  <ClearIcon />
                </IconButton>
              </Tooltip>
              <IconButton size="small" onClick={() => setDrawerOpen(false)}>
                <CloseIcon />
              </IconButton>
            </Box>
          </Box>

          {/* Connection Status */}
          <Alert 
            severity={state.connectionStatus.state === 'connected' ? 'success' : 'warning'}
            sx={{ mb: 2 }}
          >
            Connection: {state.connectionStatus.state}
          </Alert>

          {/* Event Type Filters */}
          {enableFiltering && (
            <Box mb={2}>
              <Typography variant="subtitle2" gutterBottom>
                Filter Event Types
              </Typography>
              <Stack direction="row" spacing={1} flexWrap="wrap">
                {Array.from(new Set(events.map(e => e.type))).map(eventType => (
                  <Chip
                    key={eventType}
                    label={eventType.replace('graph.', '').replace('.', ' ')}
                    size="small"
                    color={filteredTypes.has(eventType) ? 'default' : getEventColor(eventType)}
                    onClick={() => toggleEventTypeFilter(eventType)}
                    variant={filteredTypes.has(eventType) ? 'outlined' : 'filled'}
                  />
                ))}
              </Stack>
            </Box>
          )}

          {/* Grouped Events */}
          {enableGrouping && groupedEvents.size > 0 && (
            <Box mb={2}>
              <Typography variant="subtitle2" gutterBottom>
                Event Summary
              </Typography>
              {Array.from(groupedEvents.values()).map(group => (
                <Card key={group.type} variant="outlined" sx={{ mb: 1 }}>
                  <CardContent sx={{ p: 1, '&:last-child': { pb: 1 } }}>
                    <Box
                      display="flex"
                      alignItems="center"
                      justifyContent="space-between"
                      onClick={() => toggleGroupExpansion(group.type)}
                      sx={{ cursor: 'pointer' }}
                    >
                      <Box display="flex" alignItems="center" gap={1}>
                        {getEventIcon(group.type)}
                        <Typography variant="body2">
                          {group.type.replace('graph.', '').replace('.', ' ')}
                        </Typography>
                        <Chip label={group.count} size="small" />
                      </Box>
                      {expandedGroups.has(group.type) ? <ExpandLessIcon /> : <ExpandMoreIcon />}
                    </Box>
                    
                    <Collapse in={expandedGroups.has(group.type)}>
                      <List dense>
                        {group.events.map((event, index) => (
                          <ListItem key={index}>
                            <ListItemText
                              primary={`Event ${index + 1}`}
                              secondary={formatDistanceToNow(new Date(event.timestamp), { addSuffix: true })}
                            />
                            {showUserInfo && 'userId' in event && event.userId && (
                              <ListItemSecondaryAction>
                                <Tooltip title={`Modified by ${event.userId}`}>
                                  <PersonIcon fontSize="small" />
                                </Tooltip>
                              </ListItemSecondaryAction>
                            )}
                          </ListItem>
                        ))}
                      </List>
                    </Collapse>
                  </CardContent>
                </Card>
              ))}
            </Box>
          )}

          {/* Individual Events */}
          <Typography variant="subtitle2" gutterBottom>
            Recent Events ({filteredEvents.length})
          </Typography>
          
          <List>
            {filteredEvents.map((event, index) => (
              <React.Fragment key={`${event.type}-${event.timestamp}-${index}`}>
                <ListItem>
                  <ListItemIcon>
                    {getEventIcon(event.type)}
                  </ListItemIcon>
                  <ListItemText
                    primary={event.type.replace('graph.', '').replace('.', ' ')}
                    secondary={
                      <Box>
                        <Typography variant="caption" display="block">
                          {formatDistanceToNow(new Date(event.timestamp), { addSuffix: true })}
                        </Typography>
                        {showUserInfo && 'userId' in event && event.userId && (
                          <Typography variant="caption" display="block">
                            by {event.userId}
                          </Typography>
                        )}
                      </Box>
                    }
                  />
                  <ListItemSecondaryAction>
                    <Chip
                      label={getEventColor(event.type)}
                      size="small"
                      color={getEventColor(event.type)}
                    />
                  </ListItemSecondaryAction>
                </ListItem>
                {index < filteredEvents.length - 1 && <Divider />}
              </React.Fragment>
            ))}
          </List>

          {filteredEvents.length === 0 && (
            <Typography variant="body2" color="text.secondary" textAlign="center" py={4}>
              No events to display
            </Typography>
          )}
        </Box>
      </Drawer>
    </>
  );
};

export default UpdateNotifications;