import React, { useState, useEffect } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  Button,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Box,
  Typography,
  Alert,
  CircularProgress,
  Chip,
  IconButton,
  Accordion,
  AccordionSummary,
  AccordionDetails,
} from '@mui/material';
import {
  Add as AddIcon,
  Remove as RemoveIcon,
  ExpandMore as ExpandMoreIcon,
} from '@mui/icons-material';
import { useForm, Controller, useFieldArray, type FieldValues } from 'react-hook-form';
import type { 
  NodeResponse, 
  CreateNodeRequest, 
  UpdateNodeRequest, 
  NodeType, 
  PropertyType 
} from '../../types/graph';

interface NodeManagementDialogProps {
  open: boolean;
  onClose: () => void;
  onSubmit: (data: CreateNodeRequest | UpdateNodeRequest) => Promise<void>;
  node?: NodeResponse | null; // null for create, NodeResponse for edit
  loading?: boolean;
  error?: string | null;
}

interface NodeFormData {
  inode_number?: number;
  node_type: NodeType;
  properties: Array<{
    key: string;
    value: string;
    type: PropertyType;
  }>;
}

const NODE_TYPES: NodeType[] = ['File', 'Directory', 'Symlink', 'Device', 'Custom'];
const PROPERTY_TYPES: PropertyType[] = ['String', 'Integer', 'Float', 'Boolean', 'Array', 'Object'];

const NodeManagementDialog: React.FC<NodeManagementDialogProps> = ({
  open,
  onClose,
  onSubmit,
  node = null,
  loading = false,
  error = null,
}) => {
  const [submitError, setSubmitError] = useState<string | null>(null);
  const isEditing = !!node;

  // Convert node properties to form format
  const convertPropertiesToForm = (properties: Record<string, any>) => {
    return Object.entries(properties).map(([key, value]) => ({
      key,
      value: typeof value === 'object' ? JSON.stringify(value) : String(value),
      type: typeof value === 'number'
        ? (Number.isInteger(value) ? 'Integer' : 'Float')
        : typeof value === 'boolean'
        ? 'Boolean'
        : Array.isArray(value)
        ? 'Array'
        : typeof value === 'object'
        ? 'Object'
        : 'String' as PropertyType,
    }));
  };

  const {
    control,
    handleSubmit,
    reset,
    watch,
    formState: { errors, isValid },
  } = useForm<NodeFormData>({
    defaultValues: {
      inode_number: node?.inode_number || undefined,
      node_type: node?.node_type || 'File',
      properties: node ? convertPropertiesToForm(node.properties) : [],
    },
    mode: 'onChange',
  });

  const { fields, append, remove } = useFieldArray({
    control,
    name: 'properties',
  });

  // Reset form when node changes
  useEffect(() => {
    if (open) {
      reset({
        inode_number: node?.inode_number || undefined,
        node_type: node?.node_type || 'File',
        properties: node ? convertPropertiesToForm(node.properties) : [],
      });
      setSubmitError(null);
    }
  }, [node, open, reset]);

  const handleFormSubmit = async (data: NodeFormData) => {
    try {
      setSubmitError(null);

      // Convert properties back to API format
      const properties: Record<string, any> = {};
      data.properties.forEach(prop => {
        if (prop.key.trim()) {
          let value: any;
          switch (prop.type) {
            case 'Integer':
              value = parseInt(prop.value, 10);
              break;
            case 'Float':
              value = parseFloat(prop.value);
              break;
            case 'Boolean':
              value = prop.value.toLowerCase() === 'true';
              break;
            case 'Array':
              try {
                value = JSON.parse(prop.value);
              } catch {
                value = prop.value.split(',').map(s => s.trim());
              }
              break;
            case 'Object':
              try {
                value = JSON.parse(prop.value);
              } catch {
                value = prop.value;
              }
              break;
            default:
              value = prop.value;
          }
          properties[prop.key] = value;
        }
      });

      if (isEditing) {
        // Update existing node
        const updateData: UpdateNodeRequest = { properties };
        await onSubmit(updateData);
      } else {
        // Create new node
        const createData: CreateNodeRequest = {
          inode_number: data.inode_number || 0,
          node_type: data.node_type,
          properties,
        };
        await onSubmit(createData);
      }

      handleClose();
    } catch (err) {
      setSubmitError(
        err instanceof Error ? err.message : `Failed to ${isEditing ? 'update' : 'create'} node`
      );
    }
  };

  const handleClose = () => {
    reset();
    setSubmitError(null);
    onClose();
  };

  const addProperty = () => {
    append({ key: '', value: '', type: 'String' });
  };

  const validateInodeNumber = (value: number | undefined) => {
    if (!isEditing && (value === undefined || value < 0)) {
      return 'Inode number must be a non-negative integer';
    }
    return true;
  };

  const validatePropertyKey = (value: string, index: number) => {
    if (!value.trim()) {
      return 'Property key is required';
    }
    const properties = watch('properties');
    const duplicateIndex = properties.findIndex((prop, i) => i !== index && prop.key === value);
    if (duplicateIndex !== -1) {
      return 'Property key must be unique';
    }
    return true;
  };

  return (
    <Dialog
      open={open}
      onClose={handleClose}
      maxWidth="md"
      fullWidth
      PaperProps={{
        sx: { borderRadius: 2 },
      }}
    >
      <DialogTitle>
        <Typography variant="h6" component="h2" sx={{ fontWeight: 600 }}>
          {isEditing ? 'Edit Node' : 'Create New Node'}
        </Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
          {isEditing 
            ? `Modify properties for node ${node?.id}`
            : 'Create a new node in the VexGraph with custom properties.'
          }
        </Typography>
      </DialogTitle>

      <form onSubmit={handleSubmit(handleFormSubmit)}>
        <DialogContent>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
            {(error || submitError) && (
              <Alert severity="error">{error || submitError}</Alert>
            )}

            {/* Basic Node Information */}
            <Box sx={{ display: 'flex', gap: 2 }}>
              {/* Inode Number - only for creation */}
              {!isEditing && (
                <Controller
                  name="inode_number"
                  control={control}
                  rules={{ validate: validateInodeNumber }}
                  render={({ field }: { field: FieldValues }) => (
                    <TextField
                      {...field}
                      label="Inode Number"
                      type="number"
                      placeholder="0"
                      error={!!errors.inode_number}
                      helperText={
                        errors.inode_number?.message ||
                        'Unique filesystem inode identifier'
                      }
                      required
                      sx={{ flex: 1 }}
                      onChange={e => field.onChange(parseInt(e.target.value, 10))}
                    />
                  )}
                />
              )}

              {/* Node Type */}
              <Controller
                name="node_type"
                control={control}
                render={({ field }: { field: FieldValues }) => (
                  <FormControl sx={{ flex: 1 }} disabled={isEditing}>
                    <InputLabel>Node Type</InputLabel>
                    <Select {...field} label="Node Type">
                      {NODE_TYPES.map(type => (
                        <MenuItem key={type} value={type}>
                          <Box>
                            <Typography variant="body2" sx={{ fontWeight: 500 }}>
                              {type}
                            </Typography>
                            <Typography variant="caption" color="text.secondary">
                              {type === 'File' && 'Regular file node'}
                              {type === 'Directory' && 'Directory container node'}
                              {type === 'Symlink' && 'Symbolic link node'}
                              {type === 'Device' && 'Device file node'}
                              {type === 'Custom' && 'Custom node type'}
                            </Typography>
                          </Box>
                        </MenuItem>
                      ))}
                    </Select>
                  </FormControl>
                )}
              />
            </Box>

            {/* Properties Section */}
            <Accordion defaultExpanded>
              <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                <Typography variant="h6">
                  Properties ({fields.length})
                </Typography>
              </AccordionSummary>
              <AccordionDetails>
                <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                  {fields.map((field, index) => (
                    <Box key={field.id} sx={{ display: 'flex', gap: 1, alignItems: 'flex-start' }}>
                      <Controller
                        name={`properties.${index}.key`}
                        control={control}
                        rules={{ validate: (value) => validatePropertyKey(value, index) }}
                        render={({ field: keyField }: { field: FieldValues }) => (
                          <TextField
                            {...keyField}
                            label="Key"
                            placeholder="property_name"
                            error={!!errors.properties?.[index]?.key}
                            helperText={errors.properties?.[index]?.key?.message}
                            size="small"
                            sx={{ flex: 1 }}
                          />
                        )}
                      />

                      <Controller
                        name={`properties.${index}.type`}
                        control={control}
                        render={({ field: typeField }: { field: FieldValues }) => (
                          <FormControl size="small" sx={{ minWidth: 120 }}>
                            <InputLabel>Type</InputLabel>
                            <Select {...typeField} label="Type">
                              {PROPERTY_TYPES.map(type => (
                                <MenuItem key={type} value={type}>
                                  {type}
                                </MenuItem>
                              ))}
                            </Select>
                          </FormControl>
                        )}
                      />

                      <Controller
                        name={`properties.${index}.value`}
                        control={control}
                        render={({ field: valueField }: { field: FieldValues }) => (
                          <TextField
                            {...valueField}
                            label="Value"
                            placeholder="property_value"
                            size="small"
                            sx={{ flex: 2 }}
                            multiline={watch(`properties.${index}.type`) === 'Object'}
                            rows={watch(`properties.${index}.type`) === 'Object' ? 2 : 1}
                          />
                        )}
                      />

                      <IconButton
                        onClick={() => remove(index)}
                        size="small"
                        color="error"
                        sx={{ mt: 0.5 }}
                      >
                        <RemoveIcon />
                      </IconButton>
                    </Box>
                  ))}

                  <Button
                    startIcon={<AddIcon />}
                    onClick={addProperty}
                    variant="outlined"
                    size="small"
                    sx={{ alignSelf: 'flex-start' }}
                  >
                    Add Property
                  </Button>

                  {fields.length === 0 && (
                    <Typography variant="body2" color="text.secondary" sx={{ textAlign: 'center', py: 2 }}>
                      No properties defined. Click "Add Property" to add custom properties.
                    </Typography>
                  )}
                </Box>
              </AccordionDetails>
            </Accordion>

            {/* Info Box */}
            <Alert severity="info">
              <Typography variant="body2">
                <strong>Property Types:</strong> String (text), Integer (whole numbers), 
                Float (decimals), Boolean (true/false), Array (JSON array), Object (JSON object).
                {isEditing && ' Node type and inode number cannot be changed after creation.'}
              </Typography>
            </Alert>
          </Box>
        </DialogContent>

        <DialogActions sx={{ px: 3, pb: 3 }}>
          <Button onClick={handleClose} disabled={loading}>
            Cancel
          </Button>
          <Button
            type="submit"
            variant="contained"
            disabled={!isValid || loading}
            startIcon={loading ? <CircularProgress size={16} /> : null}
            sx={{ borderRadius: 2 }}
          >
            {loading 
              ? (isEditing ? 'Updating...' : 'Creating...') 
              : (isEditing ? 'Update Node' : 'Create Node')
            }
          </Button>
        </DialogActions>
      </form>
    </Dialog>
  );
};

export default NodeManagementDialog;