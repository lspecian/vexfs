import React, { useState } from 'react';
import {
  Card,
  CardContent,
  Typography,
  Box,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  ListItemSecondaryAction,
  IconButton,
  Chip,
  Button,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Tabs,
  Tab,
  Badge,
  useTheme,
} from '@mui/material';
import {
  Info as InfoIcon,
  Warning as WarningIcon,
  Error as ErrorIcon,
  PriorityHigh as CriticalIcon,
  Check as AcknowledgeIcon,
  Add as AddIcon,
  Settings as SettingsIcon,
  Notifications as NotificationsIcon,
} from '@mui/icons-material';
import type { Alert, AlertRule } from '../../types/monitoring';

interface AlertsPanelProps {
  alerts: Alert[];
  alertRules?: AlertRule[];
  loading?: boolean;
  onAcknowledgeAlert?: (alertId: string) => void;
  onCreateRule?: (
    rule: Omit<AlertRule, 'id' | 'createdAt' | 'updatedAt'>
  ) => void;
  onUpdateRule?: (ruleId: string, updates: Partial<AlertRule>) => void;
  onDeleteRule?: (ruleId: string) => void;
}

const getAlertIcon = (type: Alert['type']) => {
  switch (type) {
    case 'info':
      return <InfoIcon color="info" />;
    case 'warning':
      return <WarningIcon color="warning" />;
    case 'error':
      return <ErrorIcon color="error" />;
    case 'critical':
      return <CriticalIcon color="error" />;
    default:
      return <InfoIcon />;
  }
};

const getAlertColor = (type: Alert['type']) => {
  switch (type) {
    case 'info':
      return 'info';
    case 'warning':
      return 'warning';
    case 'error':
      return 'error';
    case 'critical':
      return 'error';
    default:
      return 'default';
  }
};

const formatTimestamp = (timestamp: string): string => {
  const now = new Date();
  const alertTime = new Date(timestamp);
  const diffMs = now.getTime() - alertTime.getTime();
  const diffMinutes = Math.floor(diffMs / 60000);

  if (diffMinutes < 1) return 'Just now';
  if (diffMinutes < 60) return `${diffMinutes}m ago`;
  const diffHours = Math.floor(diffMinutes / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  const diffDays = Math.floor(diffHours / 24);
  return `${diffDays}d ago`;
};

const AlertsPanel: React.FC<AlertsPanelProps> = ({
  alerts,
  alertRules = [],
  loading = false,
  onAcknowledgeAlert,
  onCreateRule,
  onUpdateRule,
  onDeleteRule,
}) => {
  const theme = useTheme();
  const [tabValue, setTabValue] = useState(0);
  const [ruleDialogOpen, setRuleDialogOpen] = useState(false);
  const [newRule, setNewRule] = useState({
    name: '',
    description: '',
    metric: '',
    condition: 'greater_than' as const,
    threshold: 0,
    severity: 'warning' as const,
    cooldown: 300,
    enabled: true,
  });

  const activeAlerts = alerts.filter(alert => !alert.acknowledged);
  const acknowledgedAlerts = alerts.filter(alert => alert.acknowledged);

  const handleTabChange = (_: React.SyntheticEvent, newValue: number) => {
    setTabValue(newValue);
  };

  const handleCreateRule = () => {
    if (onCreateRule) {
      onCreateRule(newRule);
      setRuleDialogOpen(false);
      setNewRule({
        name: '',
        description: '',
        metric: '',
        condition: 'greater_than',
        threshold: 0,
        severity: 'warning',
        cooldown: 300,
        enabled: true,
      });
    }
  };

  if (loading) {
    return (
      <Card>
        <CardContent>
          <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
            <Box
              sx={{
                width: 120,
                height: 32,
                backgroundColor: theme.palette.grey[300],
                borderRadius: 1,
              }}
            />
          </Box>
          <List>
            {[1, 2, 3].map(i => (
              <ListItem key={i}>
                <ListItemIcon>
                  <Box
                    sx={{
                      width: 24,
                      height: 24,
                      backgroundColor: theme.palette.grey[300],
                      borderRadius: '50%',
                    }}
                  />
                </ListItemIcon>
                <ListItemText
                  primary={
                    <Box
                      sx={{
                        width: 200,
                        height: 20,
                        backgroundColor: theme.palette.grey[300],
                        borderRadius: 1,
                      }}
                    />
                  }
                  secondary={
                    <Box
                      sx={{
                        width: 150,
                        height: 16,
                        backgroundColor: theme.palette.grey[200],
                        borderRadius: 1,
                        mt: 1,
                      }}
                    />
                  }
                />
              </ListItem>
            ))}
          </List>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardContent>
        <Box
          sx={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            mb: 2,
          }}
        >
          <Box sx={{ display: 'flex', alignItems: 'center' }}>
            <NotificationsIcon sx={{ mr: 1 }} />
            <Typography variant="h6" component="h3">
              Alerts & Monitoring
            </Typography>
          </Box>
          <Button
            startIcon={<AddIcon />}
            variant="outlined"
            size="small"
            onClick={() => setRuleDialogOpen(true)}
          >
            Add Rule
          </Button>
        </Box>

        <Tabs value={tabValue} onChange={handleTabChange} sx={{ mb: 2 }}>
          <Tab
            label={
              <Badge badgeContent={activeAlerts.length} color="error">
                Active
              </Badge>
            }
          />
          <Tab label={`Acknowledged (${acknowledgedAlerts.length})`} />
          <Tab label={`Rules (${alertRules.length})`} />
        </Tabs>

        {/* Active Alerts Tab */}
        {tabValue === 0 && (
          <Box>
            {activeAlerts.length === 0 ? (
              <Box
                sx={{
                  textAlign: 'center',
                  py: 4,
                  color: 'text.secondary',
                }}
              >
                <Typography>No active alerts</Typography>
              </Box>
            ) : (
              <List>
                {activeAlerts.map(alert => (
                  <ListItem key={alert.id} divider>
                    <ListItemIcon>{getAlertIcon(alert.type)}</ListItemIcon>
                    <ListItemText
                      primary={
                        <Box
                          sx={{ display: 'flex', alignItems: 'center', gap: 1 }}
                        >
                          <Typography variant="subtitle2">
                            {alert.title}
                          </Typography>
                          <Chip
                            label={alert.type.toUpperCase()}
                            color={getAlertColor(alert.type) as any}
                            size="small"
                          />
                        </Box>
                      }
                      secondary={
                        <Box>
                          <Typography variant="body2" color="text.secondary">
                            {alert.message}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            {formatTimestamp(alert.timestamp)} • {alert.source}
                          </Typography>
                        </Box>
                      }
                    />
                    <ListItemSecondaryAction>
                      <IconButton
                        edge="end"
                        onClick={() => onAcknowledgeAlert?.(alert.id)}
                        size="small"
                      >
                        <AcknowledgeIcon />
                      </IconButton>
                    </ListItemSecondaryAction>
                  </ListItem>
                ))}
              </List>
            )}
          </Box>
        )}

        {/* Acknowledged Alerts Tab */}
        {tabValue === 1 && (
          <Box>
            {acknowledgedAlerts.length === 0 ? (
              <Box
                sx={{
                  textAlign: 'center',
                  py: 4,
                  color: 'text.secondary',
                }}
              >
                <Typography>No acknowledged alerts</Typography>
              </Box>
            ) : (
              <List>
                {acknowledgedAlerts.map(alert => (
                  <ListItem key={alert.id} divider>
                    <ListItemIcon>{getAlertIcon(alert.type)}</ListItemIcon>
                    <ListItemText
                      primary={
                        <Box
                          sx={{ display: 'flex', alignItems: 'center', gap: 1 }}
                        >
                          <Typography variant="subtitle2" sx={{ opacity: 0.7 }}>
                            {alert.title}
                          </Typography>
                          <Chip
                            label="ACKNOWLEDGED"
                            color="success"
                            size="small"
                            variant="outlined"
                          />
                        </Box>
                      }
                      secondary={
                        <Box>
                          <Typography variant="body2" color="text.secondary">
                            {alert.message}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            {formatTimestamp(alert.timestamp)} • {alert.source}
                          </Typography>
                        </Box>
                      }
                    />
                  </ListItem>
                ))}
              </List>
            )}
          </Box>
        )}

        {/* Alert Rules Tab */}
        {tabValue === 2 && (
          <Box>
            {alertRules.length === 0 ? (
              <Box
                sx={{
                  textAlign: 'center',
                  py: 4,
                  color: 'text.secondary',
                }}
              >
                <Typography>No alert rules configured</Typography>
                <Button
                  startIcon={<AddIcon />}
                  variant="contained"
                  sx={{ mt: 2 }}
                  onClick={() => setRuleDialogOpen(true)}
                >
                  Create First Rule
                </Button>
              </Box>
            ) : (
              <List>
                {alertRules.map(rule => (
                  <ListItem key={rule.id} divider>
                    <ListItemIcon>
                      <SettingsIcon />
                    </ListItemIcon>
                    <ListItemText
                      primary={
                        <Box
                          sx={{ display: 'flex', alignItems: 'center', gap: 1 }}
                        >
                          <Typography variant="subtitle2">
                            {rule.name}
                          </Typography>
                          <Chip
                            label={rule.enabled ? 'ENABLED' : 'DISABLED'}
                            color={rule.enabled ? 'success' : 'default'}
                            size="small"
                            variant="outlined"
                          />
                        </Box>
                      }
                      secondary={
                        <Box>
                          <Typography variant="body2" color="text.secondary">
                            {rule.description}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            {rule.metric} {rule.condition.replace('_', ' ')}{' '}
                            {rule.threshold}
                          </Typography>
                        </Box>
                      }
                    />
                  </ListItem>
                ))}
              </List>
            )}
          </Box>
        )}

        {/* Create Rule Dialog */}
        <Dialog
          open={ruleDialogOpen}
          onClose={() => setRuleDialogOpen(false)}
          maxWidth="sm"
          fullWidth
        >
          <DialogTitle>Create Alert Rule</DialogTitle>
          <DialogContent>
            <Box
              sx={{ display: 'flex', flexDirection: 'column', gap: 2, mt: 1 }}
            >
              <TextField
                label="Rule Name"
                value={newRule.name}
                onChange={e => setNewRule({ ...newRule, name: e.target.value })}
                fullWidth
              />
              <TextField
                label="Description"
                value={newRule.description}
                onChange={e =>
                  setNewRule({ ...newRule, description: e.target.value })
                }
                fullWidth
                multiline
                rows={2}
              />
              <TextField
                label="Metric"
                value={newRule.metric}
                onChange={e =>
                  setNewRule({ ...newRule, metric: e.target.value })
                }
                fullWidth
                placeholder="e.g., cpu.usage, memory.percentage"
              />
              <FormControl fullWidth>
                <InputLabel>Condition</InputLabel>
                <Select
                  value={newRule.condition}
                  onChange={e =>
                    setNewRule({ ...newRule, condition: e.target.value as any })
                  }
                >
                  <MenuItem value="greater_than">Greater Than</MenuItem>
                  <MenuItem value="less_than">Less Than</MenuItem>
                  <MenuItem value="equals">Equals</MenuItem>
                  <MenuItem value="not_equals">Not Equals</MenuItem>
                </Select>
              </FormControl>
              <TextField
                label="Threshold"
                type="number"
                value={newRule.threshold}
                onChange={e =>
                  setNewRule({ ...newRule, threshold: Number(e.target.value) })
                }
                fullWidth
              />
              <FormControl fullWidth>
                <InputLabel>Severity</InputLabel>
                <Select
                  value={newRule.severity}
                  onChange={e =>
                    setNewRule({ ...newRule, severity: e.target.value as any })
                  }
                >
                  <MenuItem value="info">Info</MenuItem>
                  <MenuItem value="warning">Warning</MenuItem>
                  <MenuItem value="error">Error</MenuItem>
                  <MenuItem value="critical">Critical</MenuItem>
                </Select>
              </FormControl>
              <TextField
                label="Cooldown (seconds)"
                type="number"
                value={newRule.cooldown}
                onChange={e =>
                  setNewRule({ ...newRule, cooldown: Number(e.target.value) })
                }
                fullWidth
              />
            </Box>
          </DialogContent>
          <DialogActions>
            <Button onClick={() => setRuleDialogOpen(false)}>Cancel</Button>
            <Button onClick={handleCreateRule} variant="contained">
              Create Rule
            </Button>
          </DialogActions>
        </Dialog>
      </CardContent>
    </Card>
  );
};

export default AlertsPanel;
