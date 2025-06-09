import React, { useState } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  Box,
  Typography,
  Tabs,
  Tab,
  TextField,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  FormControlLabel,
  Switch,
  Alert,
  LinearProgress,
  Card,
  CardContent,
  Chip,
  List,
  ListItem,
  ListItemText,
  ListItemIcon,
  Divider,
} from '@mui/material';
import {
  Upload as UploadIcon,
  Download as DownloadIcon,
  CloudUpload as CloudUploadIcon,
  GetApp as GetAppIcon,
  CheckCircle as CheckIcon,
  Error as ErrorIcon,
  Warning as WarningIcon,
} from '@mui/icons-material';
import type { GraphSchema, SchemaExportOptions, SchemaImportOptions } from '../../types/schema';

interface SchemaImportExportProps {
  open: boolean;
  currentSchema: GraphSchema | null;
  onImport: (schema: GraphSchema) => void;
  onExport: (options: SchemaExportOptions) => void;
  onClose: () => void;
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
      id={`import-export-tabpanel-${index}`}
      aria-labelledby={`import-export-tab-${index}`}
      {...other}
    >
      {value === index && <Box sx={{ p: 3 }}>{children}</Box>}
    </div>
  );
}

const SchemaImportExport: React.FC<SchemaImportExportProps> = ({
  open,
  currentSchema,
  onImport,
  onExport,
  onClose,
}) => {
  const [activeTab, setActiveTab] = useState(0);
  const [isProcessing, setIsProcessing] = useState(false);
  const [importFile, setImportFile] = useState<File | null>(null);
  const [importText, setImportText] = useState('');
  const [importOptions, setImportOptions] = useState<SchemaImportOptions>({
    format: 'json',
    mergeStrategy: 'replace',
    validateBeforeImport: true,
    createBackup: true,
  });
  const [exportOptions, setExportOptions] = useState<SchemaExportOptions>({
    format: 'json',
    includeValidation: true,
    includeExamples: false,
    includeDocumentation: true,
    minify: false,
  });
  const [validationResults, setValidationResults] = useState<{
    isValid: boolean;
    errors: string[];
    warnings: string[];
  } | null>(null);

  const handleTabChange = (event: React.SyntheticEvent, newValue: number) => {
    setActiveTab(newValue);
  };

  const handleFileUpload = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      setImportFile(file);
      const reader = new FileReader();
      reader.onload = (e) => {
        const content = e.target?.result as string;
        setImportText(content);
        if (importOptions.validateBeforeImport) {
          validateImportContent(content);
        }
      };
      reader.readAsText(file);
    }
  };

  const validateImportContent = (content: string) => {
    try {
      const parsed = JSON.parse(content);
      const errors: string[] = [];
      const warnings: string[] = [];

      // Basic schema validation
      if (!parsed.node_types || !Array.isArray(parsed.node_types)) {
        errors.push('Missing or invalid node_types array');
      }
      if (!parsed.edge_types || !Array.isArray(parsed.edge_types)) {
        errors.push('Missing or invalid edge_types array');
      }
      if (!parsed.version) {
        warnings.push('Schema version not specified');
      }

      // Validate node types
      parsed.node_types?.forEach((nt: any, index: number) => {
        if (!nt.type) {
          errors.push(`Node type at index ${index} missing type field`);
        }
        if (!nt.property_types || typeof nt.property_types !== 'object') {
          warnings.push(`Node type ${nt.type} missing property_types`);
        }
      });

      // Validate edge types
      parsed.edge_types?.forEach((et: any, index: number) => {
        if (!et.type) {
          errors.push(`Edge type at index ${index} missing type field`);
        }
        if (!et.allowed_source_types || !Array.isArray(et.allowed_source_types)) {
          errors.push(`Edge type ${et.type} missing allowed_source_types`);
        }
        if (!et.allowed_target_types || !Array.isArray(et.allowed_target_types)) {
          errors.push(`Edge type ${et.type} missing allowed_target_types`);
        }
      });

      setValidationResults({
        isValid: errors.length === 0,
        errors,
        warnings,
      });
    } catch (error) {
      setValidationResults({
        isValid: false,
        errors: ['Invalid JSON format'],
        warnings: [],
      });
    }
  };

  const handleImport = async () => {
    if (!importText) return;

    setIsProcessing(true);
    try {
      const schema = JSON.parse(importText) as GraphSchema;
      
      if (importOptions.createBackup && currentSchema) {
        // Create backup (in a real implementation, this would save to storage)
        console.log('Creating backup of current schema');
      }

      onImport(schema);
      onClose();
    } catch (error) {
      console.error('Import failed:', error);
    } finally {
      setIsProcessing(false);
    }
  };

  const handleExport = async () => {
    if (!currentSchema) return;

    setIsProcessing(true);
    try {
      let exportData: any = { ...currentSchema };

      if (!exportOptions.includeValidation) {
        // Remove validation-related fields
        exportData.node_types = exportData.node_types.map((nt: any) => {
          const { constraints, ...rest } = nt;
          return rest;
        });
      }

      if (!exportOptions.includeExamples) {
        // Remove example data
        delete exportData.sampleData;
      }

      if (!exportOptions.includeDocumentation) {
        // Remove documentation fields
        exportData.node_types = exportData.node_types.map((nt: any) => {
          const { description, ...rest } = nt;
          return rest;
        });
        exportData.edge_types = exportData.edge_types.map((et: any) => {
          const { description, ...rest } = et;
          return rest;
        });
      }

      let content: string;
      let filename: string;
      let mimeType: string;

      switch (exportOptions.format) {
        case 'json':
          content = exportOptions.minify 
            ? JSON.stringify(exportData)
            : JSON.stringify(exportData, null, 2);
          filename = `schema-${new Date().toISOString().split('T')[0]}.json`;
          mimeType = 'application/json';
          break;
        case 'yaml':
          // In a real implementation, you'd use a YAML library
          content = `# VexGraph Schema Export\n# Generated: ${new Date().toISOString()}\n\n${JSON.stringify(exportData, null, 2)}`;
          filename = `schema-${new Date().toISOString().split('T')[0]}.yaml`;
          mimeType = 'text/yaml';
          break;
        case 'typescript':
          content = `// VexGraph Schema Types\n// Generated: ${new Date().toISOString()}\n\nexport const schema = ${JSON.stringify(exportData, null, 2)} as const;`;
          filename = `schema-${new Date().toISOString().split('T')[0]}.ts`;
          mimeType = 'text/typescript';
          break;
        default:
          content = JSON.stringify(exportData, null, 2);
          filename = `schema-${new Date().toISOString().split('T')[0]}.json`;
          mimeType = 'application/json';
      }

      // Create and download file
      const blob = new Blob([content], { type: mimeType });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      onExport(exportOptions);
    } catch (error) {
      console.error('Export failed:', error);
    } finally {
      setIsProcessing(false);
    }
  };

  const renderValidationResults = () => {
    if (!validationResults) return null;

    return (
      <Box sx={{ mt: 2 }}>
        <Alert 
          severity={validationResults.isValid ? 'success' : 'error'}
          sx={{ mb: 2 }}
        >
          {validationResults.isValid 
            ? 'Schema is valid and ready to import'
            : 'Schema validation failed'
          }
        </Alert>

        {validationResults.errors.length > 0 && (
          <Card sx={{ mb: 2 }}>
            <CardContent>
              <Typography variant="h6" color="error" gutterBottom>
                Errors ({validationResults.errors.length})
              </Typography>
              <List dense>
                {validationResults.errors.map((error, index) => (
                  <ListItem key={index}>
                    <ListItemIcon>
                      <ErrorIcon color="error" />
                    </ListItemIcon>
                    <ListItemText primary={error} />
                  </ListItem>
                ))}
              </List>
            </CardContent>
          </Card>
        )}

        {validationResults.warnings.length > 0 && (
          <Card>
            <CardContent>
              <Typography variant="h6" color="warning.main" gutterBottom>
                Warnings ({validationResults.warnings.length})
              </Typography>
              <List dense>
                {validationResults.warnings.map((warning, index) => (
                  <ListItem key={index}>
                    <ListItemIcon>
                      <WarningIcon color="warning" />
                    </ListItemIcon>
                    <ListItemText primary={warning} />
                  </ListItem>
                ))}
              </List>
            </CardContent>
          </Card>
        )}
      </Box>
    );
  };

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>Schema Import/Export</DialogTitle>
      <DialogContent>
        <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
          <Tabs value={activeTab} onChange={handleTabChange}>
            <Tab icon={<UploadIcon />} label="Import" />
            <Tab icon={<DownloadIcon />} label="Export" />
          </Tabs>
        </Box>

        <TabPanel value={activeTab} index={0}>
          <Typography variant="h6" gutterBottom>
            Import Schema
          </Typography>
          
          <Box sx={{ mb: 3 }}>
            <FormControl fullWidth sx={{ mb: 2 }}>
              <InputLabel>Format</InputLabel>
              <Select
                value={importOptions.format}
                label="Format"
                onChange={(e) => setImportOptions(prev => ({ 
                  ...prev, 
                  format: e.target.value as any 
                }))}
              >
                <MenuItem value="json">JSON</MenuItem>
                <MenuItem value="yaml">YAML</MenuItem>
                <MenuItem value="graphql">GraphQL</MenuItem>
              </Select>
            </FormControl>

            <FormControl fullWidth sx={{ mb: 2 }}>
              <InputLabel>Merge Strategy</InputLabel>
              <Select
                value={importOptions.mergeStrategy}
                label="Merge Strategy"
                onChange={(e) => setImportOptions(prev => ({ 
                  ...prev, 
                  mergeStrategy: e.target.value as any 
                }))}
              >
                <MenuItem value="replace">Replace Existing</MenuItem>
                <MenuItem value="merge">Merge with Existing</MenuItem>
                <MenuItem value="append">Append to Existing</MenuItem>
              </Select>
            </FormControl>

            <FormControlLabel
              control={
                <Switch
                  checked={importOptions.validateBeforeImport}
                  onChange={(e) => setImportOptions(prev => ({ 
                    ...prev, 
                    validateBeforeImport: e.target.checked 
                  }))}
                />
              }
              label="Validate before import"
            />

            <FormControlLabel
              control={
                <Switch
                  checked={importOptions.createBackup}
                  onChange={(e) => setImportOptions(prev => ({ 
                    ...prev, 
                    createBackup: e.target.checked 
                  }))}
                />
              }
              label="Create backup of current schema"
            />
          </Box>

          <Box sx={{ mb: 3 }}>
            <Button
              variant="outlined"
              component="label"
              startIcon={<CloudUploadIcon />}
              fullWidth
              sx={{ mb: 2 }}
            >
              Upload Schema File
              <input
                type="file"
                hidden
                accept=".json,.yaml,.yml"
                onChange={handleFileUpload}
              />
            </Button>

            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              Or paste schema content directly:
            </Typography>

            <TextField
              fullWidth
              multiline
              rows={8}
              value={importText}
              onChange={(e) => {
                setImportText(e.target.value);
                if (importOptions.validateBeforeImport && e.target.value) {
                  validateImportContent(e.target.value);
                }
              }}
              placeholder="Paste your schema JSON here..."
              variant="outlined"
            />
          </Box>

          {renderValidationResults()}
        </TabPanel>

        <TabPanel value={activeTab} index={1}>
          <Typography variant="h6" gutterBottom>
            Export Schema
          </Typography>

          {!currentSchema && (
            <Alert severity="warning" sx={{ mb: 2 }}>
              No schema available to export. Please load a schema first.
            </Alert>
          )}

          <Box sx={{ mb: 3 }}>
            <FormControl fullWidth sx={{ mb: 2 }}>
              <InputLabel>Format</InputLabel>
              <Select
                value={exportOptions.format}
                label="Format"
                onChange={(e) => setExportOptions(prev => ({ 
                  ...prev, 
                  format: e.target.value as any 
                }))}
              >
                <MenuItem value="json">JSON</MenuItem>
                <MenuItem value="yaml">YAML</MenuItem>
                <MenuItem value="typescript">TypeScript</MenuItem>
                <MenuItem value="graphql">GraphQL Schema</MenuItem>
              </Select>
            </FormControl>

            <FormControlLabel
              control={
                <Switch
                  checked={exportOptions.includeValidation}
                  onChange={(e) => setExportOptions(prev => ({ 
                    ...prev, 
                    includeValidation: e.target.checked 
                  }))}
                />
              }
              label="Include validation rules"
            />

            <FormControlLabel
              control={
                <Switch
                  checked={exportOptions.includeExamples}
                  onChange={(e) => setExportOptions(prev => ({ 
                    ...prev, 
                    includeExamples: e.target.checked 
                  }))}
                />
              }
              label="Include example data"
            />

            <FormControlLabel
              control={
                <Switch
                  checked={exportOptions.includeDocumentation}
                  onChange={(e) => setExportOptions(prev => ({ 
                    ...prev, 
                    includeDocumentation: e.target.checked 
                  }))}
                />
              }
              label="Include documentation"
            />

            <FormControlLabel
              control={
                <Switch
                  checked={exportOptions.minify}
                  onChange={(e) => setExportOptions(prev => ({ 
                    ...prev, 
                    minify: e.target.checked 
                  }))}
                />
              }
              label="Minify output"
            />
          </Box>

          {currentSchema && (
            <Card>
              <CardContent>
                <Typography variant="subtitle1" gutterBottom>
                  Schema Summary
                </Typography>
                <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap' }}>
                  <Chip 
                    label={`${currentSchema.node_types.length} Node Types`} 
                    color="primary" 
                    variant="outlined" 
                  />
                  <Chip 
                    label={`${currentSchema.edge_types.length} Edge Types`} 
                    color="secondary" 
                    variant="outlined" 
                  />
                  <Chip 
                    label={`Version ${currentSchema.version}`} 
                    variant="outlined" 
                  />
                </Box>
              </CardContent>
            </Card>
          )}
        </TabPanel>

        {isProcessing && (
          <Box sx={{ mt: 2 }}>
            <LinearProgress />
            <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
              {activeTab === 0 ? 'Importing schema...' : 'Exporting schema...'}
            </Typography>
          </Box>
        )}
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Cancel</Button>
        {activeTab === 0 ? (
          <Button
            onClick={handleImport}
            variant="contained"
            disabled={!importText || isProcessing || (validationResults ? !validationResults.isValid : false)}
            startIcon={<UploadIcon />}
          >
            Import
          </Button>
        ) : (
          <Button
            onClick={handleExport}
            variant="contained"
            disabled={!currentSchema || isProcessing}
            startIcon={<GetAppIcon />}
          >
            Export
          </Button>
        )}
      </DialogActions>
    </Dialog>
  );
};

export default SchemaImportExport;