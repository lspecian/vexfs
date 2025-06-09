import React from 'react';
import {
  Box,
  Chip,
  Typography,
  Tooltip,
  IconButton,
  Paper,
  Stack,
  LinearProgress,
} from '@mui/material';
import {
  Wifi as ConnectedIcon,
  WifiOff as DisconnectedIcon,
  Sync as ReconnectingIcon,
  Error as ErrorIcon,
  Refresh as RefreshIcon,
  Info as InfoIcon,
} from '@mui/icons-material';
import { formatDistanceToNow } from 'date-fns';
import { useRealTime, useConnectionStatus, useRealTimeMetrics } from './RealTimeProvider';
import type { ConnectionState } from '../../types/realtime';

export interface ConnectionStatusProps {
  showDetails?: boolean;
  showMetrics?: boolean;
  compact?: boolean;
}

const ConnectionStatus: React.FC<ConnectionStatusProps> = ({
  showDetails = false,
  showMetrics = false,
  compact = false,
}) => {
  const { reconnect } = useRealTime();
  const connectionStatus = useConnectionStatus();
  const metrics = useRealTimeMetrics();

  const getStatusColor = (state: ConnectionState): 'success' | 'warning' | 'error' | 'info' => {
    switch (state) {
      case 'connected':
        return 'success';
      case 'connecting':
      case 'reconnecting':
        return 'info';
      case 'disconnected':
        return 'warning';
      case 'error':
        return 'error';
      default:
        return 'info';
    }
  };

  const getStatusIcon = (state: ConnectionState) => {
    switch (state) {
      case 'connected':
        return <ConnectedIcon />;
      case 'connecting':
      case 'reconnecting':
        return <ReconnectingIcon className="animate-spin" />;
      case 'disconnected':
        return <DisconnectedIcon />;
      case 'error':
        return <ErrorIcon />;
      default:
        return <InfoIcon />;
    }
  };

  const getStatusText = (state: ConnectionState): string => {
    switch (state) {
      case 'connected':
        return 'Connected';
      case 'connecting':
        return 'Connecting...';
      case 'reconnecting':
        return `Reconnecting... (${connectionStatus.reconnectAttempts}/5)`;
      case 'disconnected':
        return 'Disconnected';
      case 'error':
        return 'Connection Error';
      default:
        return 'Unknown';
    }
  };

  const formatLatency = (latency?: number): string => {
    if (!latency) return 'N/A';
    return `${latency}ms`;
  };

  const formatUptime = (uptime: number): string => {
    if (uptime === 0) return 'N/A';
    const seconds = Math.floor(uptime / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    
    if (hours > 0) {
      return `${hours}h ${minutes % 60}m`;
    } else if (minutes > 0) {
      return `${minutes}m ${seconds % 60}s`;
    } else {
      return `${seconds}s`;
    }
  };

  const handleReconnect = async () => {
    try {
      await reconnect();
    } catch (error) {
      console.error('Failed to reconnect:', error);
    }
  };

  if (compact) {
    return (
      <Tooltip title={`Real-time updates: ${getStatusText(connectionStatus.state)}`}>
        <Chip
          icon={getStatusIcon(connectionStatus.state)}
          label={getStatusText(connectionStatus.state)}
          color={getStatusColor(connectionStatus.state)}
          size="small"
          variant="outlined"
        />
      </Tooltip>
    );
  }

  return (
    <Paper elevation={1} sx={{ p: 2 }}>
      <Stack spacing={2}>
        {/* Main Status */}
        <Box display="flex" alignItems="center" justifyContent="space-between">
          <Box display="flex" alignItems="center" gap={1}>
            {getStatusIcon(connectionStatus.state)}
            <Typography variant="h6">
              Real-time Connection
            </Typography>
          </Box>
          
          <Box display="flex" alignItems="center" gap={1}>
            <Chip
              label={getStatusText(connectionStatus.state)}
              color={getStatusColor(connectionStatus.state)}
              size="small"
            />
            
            {(connectionStatus.state === 'disconnected' || connectionStatus.state === 'error') && (
              <Tooltip title="Reconnect">
                <IconButton size="small" onClick={handleReconnect}>
                  <RefreshIcon />
                </IconButton>
              </Tooltip>
            )}
          </Box>
        </Box>

        {/* Connection Progress */}
        {(connectionStatus.state === 'connecting' || connectionStatus.state === 'reconnecting') && (
          <LinearProgress />
        )}

        {/* Error Message */}
        {connectionStatus.state === 'error' && connectionStatus.error && (
          <Typography variant="body2" color="error">
            {connectionStatus.error}
          </Typography>
        )}

        {/* Connection Details */}
        {showDetails && connectionStatus.state === 'connected' && (
          <Stack spacing={1}>
            <Typography variant="subtitle2" color="text.secondary">
              Connection Details
            </Typography>
            
            <Box display="grid" gridTemplateColumns="1fr 1fr" gap={1}>
              <Typography variant="body2">
                <strong>Connected:</strong>{' '}
                {connectionStatus.connectedAt
                  ? formatDistanceToNow(connectionStatus.connectedAt, { addSuffix: true })
                  : 'N/A'}
              </Typography>
              
              <Typography variant="body2">
                <strong>Latency:</strong> {formatLatency(connectionStatus.latency)}
              </Typography>
              
              <Typography variant="body2">
                <strong>Last Heartbeat:</strong>{' '}
                {connectionStatus.lastHeartbeat
                  ? formatDistanceToNow(connectionStatus.lastHeartbeat, { addSuffix: true })
                  : 'N/A'}
              </Typography>
              
              <Typography variant="body2">
                <strong>Reconnections:</strong> {connectionStatus.reconnectAttempts}
              </Typography>
            </Box>
          </Stack>
        )}

        {/* Real-time Metrics */}
        {showMetrics && (
          <Stack spacing={1}>
            <Typography variant="subtitle2" color="text.secondary">
              Real-time Metrics
            </Typography>
            
            <Box display="grid" gridTemplateColumns="1fr 1fr" gap={1}>
              <Typography variant="body2">
                <strong>Messages Received:</strong> {metrics.messagesReceived.toLocaleString()}
              </Typography>
              
              <Typography variant="body2">
                <strong>Messages Sent:</strong> {metrics.messagesSent.toLocaleString()}
              </Typography>
              
              <Typography variant="body2">
                <strong>Events Processed:</strong> {metrics.eventsProcessed.toLocaleString()}
              </Typography>
              
              <Typography variant="body2">
                <strong>Conflicts Detected:</strong> {metrics.conflictsDetected.toLocaleString()}
              </Typography>
              
              <Typography variant="body2">
                <strong>Conflicts Resolved:</strong> {metrics.conflictsResolved.toLocaleString()}
              </Typography>
              
              <Typography variant="body2">
                <strong>Uptime:</strong> {formatUptime(metrics.connectionUptime)}
              </Typography>
            </Box>
            
            <Typography variant="caption" color="text.secondary">
              Last updated: {formatDistanceToNow(metrics.lastUpdated, { addSuffix: true })}
            </Typography>
          </Stack>
        )}
      </Stack>
    </Paper>
  );
};

export default ConnectionStatus;