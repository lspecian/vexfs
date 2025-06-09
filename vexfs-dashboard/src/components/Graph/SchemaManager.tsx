import React, { useState, useEffect, useCallback } from 'react';
import {
  Box,
  Paper,
  Typography,
  Tabs,
  Tab,
  Button,
  IconButton,
  Tooltip,
  Alert,
  Snackbar,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Grid,
  Card,
  CardContent,
  CardActions,
  Chip,
  LinearProgress,
} from '@mui/material';
import {
  Schema as SchemaIcon,
  Add as AddIcon,
  Edit as EditIcon,
  Delete as DeleteIcon,
  Visibility as ViewIcon,
  Download as ExportIcon,
  Upload as ImportIcon,
  Refresh as RefreshIcon,
  CheckCircle as ValidIcon,
  Warning as WarningIcon,
  Error as ErrorIcon,
  Timeline as MigrationIcon,
  ViewModule as TemplateIcon,
} from '@mui/icons-material';
import { vexfsApi } from '../../services/api';
import type {
  GraphSchema,
  NodeTypeDefinition,
  EdgeTypeDefinition,
  SchemaValidationResult,
  SchemaTemplate,
  SchemaManagerState,
} from '../../types/schema';
import NodeTypeEditor from './NodeTypeEditor';
import EdgeTypeEditor from './EdgeTypeEditor';
import SchemaValidator from './SchemaValidator';
import SchemaVisualizer from './SchemaVisualizer';
import SchemaImportExport from './SchemaImportExport';
import PropertySchemaBuilder from './PropertySchemaBuilder';

interface SchemaManagerProps {
  onSchemaChange?: (schema: GraphSchema) => void;
  onValidationChange?: (results: SchemaValidationResult) => void;
}

interface TabPanelProps {
  children?: React.ReactNode;
  index: number;
  value: number;
}

function TabPanel(props: TabPanelProps) {
  const { children, value, index, ...other } = props;
  return (
    <div
      role="tabpanel"
      hidden={value !== index}
      id={`schema-tabpanel-${index}`}
      aria-labelledby={`schema-tab-${index}`}
      {...other}
    >
      {value === index && <Box sx={{ p: 3 }}>{children}</Box>}
    </div>
  );
}

const SchemaManager: React.FC<SchemaManagerProps> = ({
  onSchemaChange,
  onValidationChange,
}) => {
  const api = vexfsApi;
  const [state, setState] = useState<SchemaManagerState>({
    currentSchema: null,
    nodeTypes: [],
    edgeTypes: [],
    validationResults: null,
    isLoading: false,
    error: null,
    selectedNodeType: null,
    selectedEdgeType: null,
    showValidation: false,
    showMigration: false,
  });

  const [activeTab, setActiveTab] = useState(0);
  const [showNodeEditor, setShowNodeEditor] = useState(false);
  const [showEdgeEditor, setShowEdgeEditor] = useState(false);
  const [showImportExport, setShowImportExport] = useState(false);
  const [showTemplates, setShowTemplates] = useState(false);
  const [editingNodeType, setEditingNodeType] = useState<NodeTypeDefinition | null>(null);
  const [editingEdgeType, setEditingEdgeType] = useState<EdgeTypeDefinition | null>(null);
  const [snackbar, setSnackbar] = useState<{
    open: boolean;
    message: string;
    severity: 'success' | 'error' | 'warning' | 'info';
  }>({
    open: false,
    message: '',
    severity: 'info',
  });

  // Load current schema
  const loadSchema = useCallback(async () => {
    setState(prev => ({ ...prev, isLoading: true, error: null }));
    try {
      const schema = await api.getGraphSchema();
      const nodeTypes = schema.node_types.map((nt: any) => ({
        id: nt.type,
        name: nt.type,
        displayName: nt.type,
        description: `Node type: ${nt.type}`,
        properties: Object.entries(nt.property_types).map(([name, type]) => ({
          name,
          type: type as any,
          constraints: nt.required_properties.includes(name) 
            ? [{ type: 'required' as const }] 
            : [{ type: 'optional' as const }],
        })),
      })) as NodeTypeDefinition[];

      const edgeTypes = schema.edge_types.map((et: any) => ({
        id: et.type,
        name: et.type,
        displayName: et.type,
        description: `Edge type: ${et.type}`,
        directionality: 'directed' as const,
        allowedSourceTypes: et.allowed_source_types,
        allowedTargetTypes: et.allowed_target_types,
        cardinality: 'many-to-many' as const,
        properties: Object.entries(et.property_types).map(([name, type]) => ({
          name,
          type: type as any,
          constraints: et.required_properties.includes(name) 
            ? [{ type: 'required' as const }] 
            : [{ type: 'optional' as const }],
        })),
      })) as EdgeTypeDefinition[];

      setState(prev => ({
        ...prev,
        currentSchema: schema,
        nodeTypes,
        edgeTypes,
        isLoading: false,
      }));

      onSchemaChange?.(schema);
    } catch (error) {
      setState(prev => ({
        ...prev,
        error: error instanceof Error ? error.message : 'Failed to load schema',
        isLoading: false,
      }));
    }
  }, [api, onSchemaChange]);

  // Validate schema
  const validateSchema = useCallback(async () => {
    if (!state.currentSchema) return;

    setState(prev => ({ ...prev, isLoading: true }));
    try {
      // This would call a validation API endpoint
      // For now, we'll simulate validation
      const validationResults: SchemaValidationResult = {
        isValid: true,
        errors: [],
        warnings: [],
        statistics: {
          totalNodes: state.nodeTypes.length,
          totalEdges: state.edgeTypes.length,
          validNodes: state.nodeTypes.length,
          validEdges: state.edgeTypes.length,
          invalidNodes: 0,
          invalidEdges: 0,
        },
      };

      setState(prev => ({
        ...prev,
        validationResults,
        isLoading: false,
      }));

      onValidationChange?.(validationResults);
      setSnackbar({
        open: true,
        message: 'Schema validation completed successfully',
        severity: 'success',
      });
    } catch (error) {
      setState(prev => ({
        ...prev,
        error: error instanceof Error ? error.message : 'Validation failed',
        isLoading: false,
      }));
    }
  }, [state.currentSchema, state.nodeTypes, state.edgeTypes, onValidationChange]);

  // Handle node type operations
  const handleCreateNodeType = () => {
    setEditingNodeType(null);
    setShowNodeEditor(true);
  };

  const handleEditNodeType = (nodeType: NodeTypeDefinition) => {
    setEditingNodeType(nodeType);
    setShowNodeEditor(true);
  };

  const handleDeleteNodeType = async (nodeTypeId: string) => {
    try {
      // This would call the API to delete the node type
      setState(prev => ({
        ...prev,
        nodeTypes: prev.nodeTypes.filter(nt => nt.id !== nodeTypeId),
      }));
      setSnackbar({
        open: true,
        message: 'Node type deleted successfully',
        severity: 'success',
      });
    } catch (error) {
      setSnackbar({
        open: true,
        message: 'Failed to delete node type',
        severity: 'error',
      });
    }
  };

  const handleSaveNodeType = async (nodeType: NodeTypeDefinition) => {
    try {
      // This would call the API to save the node type
      setState(prev => ({
        ...prev,
        nodeTypes: editingNodeType
          ? prev.nodeTypes.map(nt => nt.id === nodeType.id ? nodeType : nt)
          : [...prev.nodeTypes, nodeType],
      }));
      setShowNodeEditor(false);
      setEditingNodeType(null);
      setSnackbar({
        open: true,
        message: `Node type ${editingNodeType ? 'updated' : 'created'} successfully`,
        severity: 'success',
      });
    } catch (error) {
      setSnackbar({
        open: true,
        message: 'Failed to save node type',
        severity: 'error',
      });
    }
  };

  // Handle edge type operations
  const handleCreateEdgeType = () => {
    setEditingEdgeType(null);
    setShowEdgeEditor(true);
  };

  const handleEditEdgeType = (edgeType: EdgeTypeDefinition) => {
    setEditingEdgeType(edgeType);
    setShowEdgeEditor(true);
  };

  const handleDeleteEdgeType = async (edgeTypeId: string) => {
    try {
      // This would call the API to delete the edge type
      setState(prev => ({
        ...prev,
        edgeTypes: prev.edgeTypes.filter(et => et.id !== edgeTypeId),
      }));
      setSnackbar({
        open: true,
        message: 'Edge type deleted successfully',
        severity: 'success',
      });
    } catch (error) {
      setSnackbar({
        open: true,
        message: 'Failed to delete edge type',
        severity: 'error',
      });
    }
  };

  const handleSaveEdgeType = async (edgeType: EdgeTypeDefinition) => {
    try {
      // This would call the API to save the edge type
      setState(prev => ({
        ...prev,
        edgeTypes: editingEdgeType
          ? prev.edgeTypes.map(et => et.id === edgeType.id ? edgeType : et)
          : [...prev.edgeTypes, edgeType],
      }));
      setShowEdgeEditor(false);
      setEditingEdgeType(null);
      setSnackbar({
        open: true,
        message: `Edge type ${editingEdgeType ? 'updated' : 'created'} successfully`,
        severity: 'success',
      });
    } catch (error) {
      setSnackbar({
        open: true,
        message: 'Failed to save edge type',
        severity: 'error',
      });
    }
  };

  // Load schema on component mount
  useEffect(() => {
    loadSchema();
  }, [loadSchema]);

  const handleTabChange = (event: React.SyntheticEvent, newValue: number) => {
    setActiveTab(newValue);
  };

  const getValidationIcon = (results: SchemaValidationResult | null) => {
    if (!results) return null;
    if (results.errors.length > 0) return <ErrorIcon color="error" />;
    if (results.warnings.length > 0) return <WarningIcon color="warning" />;
    return <ValidIcon color="success" />;
  };

  return (
    <Box sx={{ width: '100%', height: '100%' }}>
      <Paper sx={{ mb: 2, p: 2 }}>
        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
          <Typography variant="h5" component="h2" sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <SchemaIcon />
            Schema Manager
            {getValidationIcon(state.validationResults)}
          </Typography>
          <Box sx={{ display: 'flex', gap: 1 }}>
            <Tooltip title="Refresh Schema">
              <IconButton onClick={loadSchema} disabled={state.isLoading}>
                <RefreshIcon />
              </IconButton>
            </Tooltip>
            <Tooltip title="Validate Schema">
              <IconButton onClick={validateSchema} disabled={state.isLoading}>
                <ValidIcon />
              </IconButton>
            </Tooltip>
            <Tooltip title="Import/Export">
              <IconButton onClick={() => setShowImportExport(true)}>
                <ImportIcon />
              </IconButton>
            </Tooltip>
            <Tooltip title="Schema Templates">
              <IconButton onClick={() => setShowTemplates(true)}>
                <TemplateIcon />
              </IconButton>
            </Tooltip>
          </Box>
        </Box>

        {state.isLoading && <LinearProgress sx={{ mb: 2 }} />}

        {state.error && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {state.error}
          </Alert>
        )}

        <Tabs value={activeTab} onChange={handleTabChange} aria-label="schema management tabs">
          <Tab label={`Node Types (${state.nodeTypes.length})`} />
          <Tab label={`Edge Types (${state.edgeTypes.length})`} />
          <Tab label="Schema Visualization" />
          <Tab label="Validation Results" />
          <Tab label="Migration Tools" />
        </Tabs>
      </Paper>

      <TabPanel value={activeTab} index={0}>
        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
          <Typography variant="h6">Node Types</Typography>
          <Button
            variant="contained"
            startIcon={<AddIcon />}
            onClick={handleCreateNodeType}
          >
            Create Node Type
          </Button>
        </Box>
        <Grid container spacing={2}>
          {state.nodeTypes.map((nodeType) => (
            <Grid item xs={12} md={6} lg={4} key={nodeType.id}>
              <Card>
                <CardContent>
                  <Typography variant="h6" gutterBottom>
                    {nodeType.displayName}
                  </Typography>
                  <Typography variant="body2" color="text.secondary" gutterBottom>
                    {nodeType.description}
                  </Typography>
                  <Box sx={{ mt: 1 }}>
                    <Chip
                      label={`${nodeType.properties.length} properties`}
                      size="small"
                      variant="outlined"
                    />
                    {nodeType.inheritFrom && nodeType.inheritFrom.length > 0 && (
                      <Chip
                        label={`Inherits from ${nodeType.inheritFrom.length}`}
                        size="small"
                        variant="outlined"
                        sx={{ ml: 1 }}
                      />
                    )}
                  </Box>
                </CardContent>
                <CardActions>
                  <IconButton
                    size="small"
                    onClick={() => handleEditNodeType(nodeType)}
                    title="Edit"
                  >
                    <EditIcon />
                  </IconButton>
                  <IconButton
                    size="small"
                    onClick={() => handleDeleteNodeType(nodeType.id)}
                    title="Delete"
                  >
                    <DeleteIcon />
                  </IconButton>
                </CardActions>
              </Card>
            </Grid>
          ))}
        </Grid>
      </TabPanel>

      <TabPanel value={activeTab} index={1}>
        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
          <Typography variant="h6">Edge Types</Typography>
          <Button
            variant="contained"
            startIcon={<AddIcon />}
            onClick={handleCreateEdgeType}
          >
            Create Edge Type
          </Button>
        </Box>
        <Grid container spacing={2}>
          {state.edgeTypes.map((edgeType) => (
            <Grid item xs={12} md={6} lg={4} key={edgeType.id}>
              <Card>
                <CardContent>
                  <Typography variant="h6" gutterBottom>
                    {edgeType.displayName}
                  </Typography>
                  <Typography variant="body2" color="text.secondary" gutterBottom>
                    {edgeType.description}
                  </Typography>
                  <Box sx={{ mt: 1 }}>
                    <Chip
                      label={edgeType.directionality}
                      size="small"
                      variant="outlined"
                    />
                    <Chip
                      label={edgeType.cardinality}
                      size="small"
                      variant="outlined"
                      sx={{ ml: 1 }}
                    />
                    <Chip
                      label={`${edgeType.properties.length} properties`}
                      size="small"
                      variant="outlined"
                      sx={{ ml: 1 }}
                    />
                  </Box>
                </CardContent>
                <CardActions>
                  <IconButton
                    size="small"
                    onClick={() => handleEditEdgeType(edgeType)}
                    title="Edit"
                  >
                    <EditIcon />
                  </IconButton>
                  <IconButton
                    size="small"
                    onClick={() => handleDeleteEdgeType(edgeType.id)}
                    title="Delete"
                  >
                    <DeleteIcon />
                  </IconButton>
                </CardActions>
              </Card>
            </Grid>
          ))}
        </Grid>
      </TabPanel>

      <TabPanel value={activeTab} index={2}>
        <SchemaVisualizer
          nodeTypes={state.nodeTypes}
          edgeTypes={state.edgeTypes}
          onNodeTypeSelect={(nodeTypeId: string) => setState(prev => ({ ...prev, selectedNodeType: nodeTypeId }))}
          onEdgeTypeSelect={(edgeTypeId: string) => setState(prev => ({ ...prev, selectedEdgeType: edgeTypeId }))}
        />
      </TabPanel>

      <TabPanel value={activeTab} index={3}>
        <SchemaValidator
          schema={state.currentSchema}
          validationResults={state.validationResults}
          onValidate={validateSchema}
          onFixIssue={(issueId: string) => {
            // Handle fixing validation issues
            console.log('Fix issue:', issueId);
          }}
        />
      </TabPanel>

      <TabPanel value={activeTab} index={4}>
        <Box>
          <Typography variant="h6" gutterBottom>
            Schema Migration Tools
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Migration tools will be available in a future version.
          </Typography>
        </Box>
      </TabPanel>

      {/* Node Type Editor Dialog */}
      <NodeTypeEditor
        open={showNodeEditor}
        nodeType={editingNodeType}
        existingNodeTypes={state.nodeTypes}
        onSave={handleSaveNodeType}
        onCancel={() => {
          setShowNodeEditor(false);
          setEditingNodeType(null);
        }}
      />

      {/* Edge Type Editor Dialog */}
      <EdgeTypeEditor
        open={showEdgeEditor}
        edgeType={editingEdgeType}
        existingEdgeTypes={state.edgeTypes}
        availableNodeTypes={state.nodeTypes}
        onSave={handleSaveEdgeType}
        onCancel={() => {
          setShowEdgeEditor(false);
          setEditingEdgeType(null);
        }}
      />

      {/* Import/Export Dialog */}
      <SchemaImportExport
        open={showImportExport}
        currentSchema={state.currentSchema}
        onImport={(schema: any) => {
          // Handle schema import
          console.log('Import schema:', schema);
          setShowImportExport(false);
        }}
        onExport={(options: any) => {
          // Handle schema export
          console.log('Export schema:', options);
        }}
        onClose={() => setShowImportExport(false)}
      />

      {/* Snackbar for notifications */}
      <Snackbar
        open={snackbar.open}
        autoHideDuration={6000}
        onClose={() => setSnackbar(prev => ({ ...prev, open: false }))}
      >
        <Alert
          onClose={() => setSnackbar(prev => ({ ...prev, open: false }))}
          severity={snackbar.severity}
          sx={{ width: '100%' }}
        >
          {snackbar.message}
        </Alert>
      </Snackbar>
    </Box>
  );
};

export default SchemaManager;