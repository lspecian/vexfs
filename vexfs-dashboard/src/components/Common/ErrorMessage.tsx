/**
 * Error Message Component
 * Displays user-friendly error messages with retry options
 */

import React from 'react';
import {
  Alert,
  AlertTitle,
  Button,
  Box,
  Typography,
  Collapse,
  IconButton,
} from '@mui/material';
import {
  Refresh,
  ExpandMore,
  ExpandLess,
  Error as ErrorIcon,
  Warning,
  Info,
} from '@mui/icons-material';
import { useState } from 'react';

export interface ErrorMessageProps {
  error: string | Error | null;
  title?: string;
  severity?: 'error' | 'warning' | 'info';
  showRetry?: boolean;
  showDetails?: boolean;
  onRetry?: () => void;
  onDismiss?: () => void;
  retryText?: string;
  className?: string;
}

export const ErrorMessage: React.FC<ErrorMessageProps> = ({
  error,
  title,
  severity = 'error',
  showRetry = false,
  showDetails = false,
  onRetry,
  onDismiss,
  retryText = 'Try Again',
  className,
}) => {
  const [showDetailedError, setShowDetailedError] = useState(false);

  if (!error) {
    return null;
  }

  const errorMessage = typeof error === 'string' ? error : error.message;
  const errorStack =
    typeof error === 'object' && error.stack ? error.stack : null;

  const getIcon = () => {
    switch (severity) {
      case 'warning':
        return <Warning />;
      case 'info':
        return <Info />;
      default:
        return <ErrorIcon />;
    }
  };

  const getTitle = () => {
    if (title) return title;

    switch (severity) {
      case 'warning':
        return 'Warning';
      case 'info':
        return 'Information';
      default:
        return 'Error';
    }
  };

  return (
    <Alert
      severity={severity}
      className={className}
      onClose={onDismiss}
      icon={getIcon()}
      action={
        <Box display="flex" alignItems="center" gap={1}>
          {showDetails && errorStack && (
            <IconButton
              size="small"
              onClick={() => setShowDetailedError(!showDetailedError)}
              aria-label="toggle error details"
            >
              {showDetailedError ? <ExpandLess /> : <ExpandMore />}
            </IconButton>
          )}
          {showRetry && onRetry && (
            <Button
              size="small"
              startIcon={<Refresh />}
              onClick={onRetry}
              variant="outlined"
              color="inherit"
            >
              {retryText}
            </Button>
          )}
        </Box>
      }
    >
      <AlertTitle>{getTitle()}</AlertTitle>
      <Typography variant="body2">{errorMessage}</Typography>

      {showDetails && errorStack && (
        <Collapse in={showDetailedError}>
          <Box mt={2}>
            <Typography variant="subtitle2" gutterBottom>
              Technical Details:
            </Typography>
            <Box
              component="pre"
              sx={{
                fontSize: '0.75rem',
                backgroundColor: 'rgba(0, 0, 0, 0.04)',
                padding: 1,
                borderRadius: 1,
                overflow: 'auto',
                maxHeight: 200,
                whiteSpace: 'pre-wrap',
              }}
            >
              {errorStack}
            </Box>
          </Box>
        </Collapse>
      )}
    </Alert>
  );
};

export default ErrorMessage;
