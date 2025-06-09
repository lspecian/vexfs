import React, { useState } from 'react';
import {
  Box,
  Typography,
  Button,
  Card,
  CardContent,
  CardActions,
  Grid,
  Alert,
  Chip,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  ListItemSecondaryAction,
  IconButton,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  LinearProgress,
  Tooltip,
  Divider,
} from '@mui/material';
import {
  CheckCircle as ValidIcon,
  Error as ErrorIcon,
  Warning as WarningIcon,
  Info as InfoIcon,
  Refresh as RefreshIcon,
  Build as FixIcon,
  ExpandMore as ExpandMoreIcon,
  Assessment as StatsIcon,
} from '@mui/icons-material';
import type { 
  GraphSchema, 
  SchemaValidationResult, 
  SchemaValidationError, 
  SchemaValidationWarning 
} from '../../types/schema';

interface SchemaValidatorProps {
  schema: GraphSchema | null;
  validationResults: SchemaValidationResult | null;
  onValidate: () => void;
  onFixIssue: (issueId: string) => void;
}

const SchemaValidator: React.FC<SchemaValidatorProps> = ({
  schema,
  validationResults,
  onValidate,
  onFixIssue,
}) => {
  const [isValidating, setIsValidating] = useState(false);

  const handleValidate = async () => {
    setIsValidating(true);
    try {
      await onValidate();
    } finally {
      setIsValidating(false);
    }
  };

  const getSeverityIcon = (severity: 'error' | 'warning' | 'info') => {
    switch (severity) {
      case 'error':
        return <ErrorIcon color="error" />;
      case 'warning':
        return <WarningIcon color="warning" />;
      case 'info':
        return <InfoIcon color="info" />;
      default:
        return <InfoIcon />;
    }
  };

  const getSeverityColor = (severity: 'error' | 'warning' | 'info') => {
    switch (severity) {
      case 'error':
        return 'error';
      case 'warning':
        return 'warning';
      case 'info':
        return 'info';
      default:
        return 'default';
    }
  };

  const getValidationSummary = () => {
    if (!validationResults) return null;

    const { errors, warnings, statistics } = validationResults;
    const totalIssues = errors.length + warnings.length;
    const errorCount = errors.length;
    const warningCount = warnings.length;

    return (
      <Card sx={{ mb: 2 }}>
        <CardContent>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 2 }}>
            <StatsIcon />
            <Typography variant="h6">Validation Summary</Typography>
            {validationResults.isValid ? (
              <Chip icon={<ValidIcon />} label="Valid" color="success" />
            ) : (
              <Chip icon={<ErrorIcon />} label="Invalid" color="error" />
            )}
          </Box>

          <Grid container spacing={2}>
            <Grid item xs={12} md={3}>
              <Box sx={{ textAlign: 'center' }}>
                <Typography variant="h4" color={totalIssues > 0 ? 'error.main' : 'success.main'}>
                  {totalIssues}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Total Issues
                </Typography>
              </Box>
            </Grid>
            <Grid item xs={12} md={3}>
              <Box sx={{ textAlign: 'center' }}>
                <Typography variant="h4" color="error.main">
                  {errorCount}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Errors
                </Typography>
              </Box>
            </Grid>
            <Grid item xs={12} md={3}>
              <Box sx={{ textAlign: 'center' }}>
                <Typography variant="h4" color="warning.main">
                  {warningCount}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Warnings
                </Typography>
              </Box>
            </Grid>
            <Grid item xs={12} md={3}>
              <Box sx={{ textAlign: 'center' }}>
                <Typography variant="h4" color="success.main">
                  {((statistics.validNodes + statistics.validEdges) / 
                    (statistics.totalNodes + statistics.totalEdges) * 100).toFixed(1)}%
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Valid Items
                </Typography>
              </Box>
            </Grid>
          </Grid>

          <Divider sx={{ my: 2 }} />

          <Grid container spacing={2}>
            <Grid item xs={6} md={3}>
              <Typography variant="body2" color="text.secondary">
                Total Nodes: {statistics.totalNodes}
              </Typography>
            </Grid>
            <Grid item xs={6} md={3}>
              <Typography variant="body2" color="text.secondary">
                Valid Nodes: {statistics.validNodes}
              </Typography>
            </Grid>
            <Grid item xs={6} md={3}>
              <Typography variant="body2" color="text.secondary">
                Total Edges: {statistics.totalEdges}
              </Typography>
            </Grid>
            <Grid item xs={6} md={3}>
              <Typography variant="body2" color="text.secondary">
                Valid Edges: {statistics.validEdges}
              </Typography>
            </Grid>
          </Grid>
        </CardContent>
        <CardActions>
          <Button
            startIcon={<RefreshIcon />}
            onClick={handleValidate}
            disabled={isValidating || !schema}
            variant="outlined"
          >
            Re-validate
          </Button>
        </CardActions>
      </Card>
    );
  };

  const renderValidationIssues = () => {
    if (!validationResults) return null;

    const { errors, warnings } = validationResults;
    const allIssues = [
      ...errors.map(e => ({ ...e, severity: 'error' as const })),
      ...warnings.map(w => ({ ...w, severity: 'warning' as const, rule: 'warning', suggestedFix: w.recommendation })),
    ];

    if (allIssues.length === 0) {
      return (
        <Alert severity="success" sx={{ mt: 2 }}>
          <Typography variant="h6">Schema is valid!</Typography>
          <Typography>
            No validation errors or warnings found. Your schema is properly configured.
          </Typography>
        </Alert>
      );
    }

    const groupedIssues = allIssues.reduce((acc, issue) => {
      const key = issue.type;
      if (!acc[key]) acc[key] = [];
      acc[key].push(issue);
      return acc;
    }, {} as Record<string, typeof allIssues>);

    return (
      <Box sx={{ mt: 2 }}>
        <Typography variant="h6" gutterBottom>
          Validation Issues
        </Typography>
        {Object.entries(groupedIssues).map(([type, issues]) => (
          <Accordion key={type} defaultExpanded={issues.some(i => i.severity === 'error')}>
            <AccordionSummary expandIcon={<ExpandMoreIcon />}>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <Typography variant="subtitle1" sx={{ textTransform: 'capitalize' }}>
                  {type} Issues ({issues.length})
                </Typography>
                {issues.some(i => i.severity === 'error') && (
                  <Chip size="small" color="error" label={issues.filter(i => i.severity === 'error').length} />
                )}
                {issues.some(i => i.severity === 'warning') && (
                  <Chip size="small" color="warning" label={issues.filter(i => i.severity === 'warning').length} />
                )}
              </Box>
            </AccordionSummary>
            <AccordionDetails>
              <List>
                {issues.map((issue, index) => (
                  <ListItem key={`${issue.id}-${index}`} divider>
                    <ListItemIcon>
                      {getSeverityIcon(issue.severity)}
                    </ListItemIcon>
                    <ListItemText
                      primary={
                        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                          <Typography variant="subtitle2">
                            {issue.targetId}
                          </Typography>
                          <Chip 
                            size="small" 
                            label={issue.rule} 
                            color={getSeverityColor(issue.severity)}
                            variant="outlined"
                          />
                        </Box>
                      }
                      secondary={
                        <Box>
                          <Typography variant="body2" color="text.primary">
                            {issue.message}
                          </Typography>
                          {issue.suggestedFix && (
                            <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
                              <strong>Suggested fix:</strong> {issue.suggestedFix}
                            </Typography>
                          )}
                        </Box>
                      }
                    />
                    <ListItemSecondaryAction>
                      {issue.suggestedFix && (
                        <Tooltip title="Apply suggested fix">
                          <IconButton
                            edge="end"
                            onClick={() => onFixIssue(issue.id)}
                            size="small"
                          >
                            <FixIcon />
                          </IconButton>
                        </Tooltip>
                      )}
                    </ListItemSecondaryAction>
                  </ListItem>
                ))}
              </List>
            </AccordionDetails>
          </Accordion>
        ))}
      </Box>
    );
  };

  if (!schema) {
    return (
      <Box sx={{ textAlign: 'center', py: 4 }}>
        <Typography variant="h6" color="text.secondary">
          No schema available for validation
        </Typography>
        <Typography variant="body2" color="text.secondary">
          Please load a schema first to perform validation.
        </Typography>
      </Box>
    );
  }

  return (
    <Box>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
        <Typography variant="h6">Schema Validation</Typography>
        <Button
          variant="contained"
          startIcon={<RefreshIcon />}
          onClick={handleValidate}
          disabled={isValidating}
        >
          {isValidating ? 'Validating...' : 'Validate Schema'}
        </Button>
      </Box>

      {isValidating && (
        <Box sx={{ mb: 2 }}>
          <LinearProgress />
          <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
            Validating schema and checking data compliance...
          </Typography>
        </Box>
      )}

      {getValidationSummary()}
      {renderValidationIssues()}

      {!validationResults && !isValidating && (
        <Alert severity="info">
          Click "Validate Schema" to check your schema for issues and verify data compliance.
        </Alert>
      )}
    </Box>
  );
};

export default SchemaValidator;