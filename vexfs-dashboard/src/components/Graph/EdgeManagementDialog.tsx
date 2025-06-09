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
  Autocomplete,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  IconButton,
} from '@mui/material';
import {
  Add as AddIcon,
  Remove as RemoveIcon,
  ExpandMore as ExpandMoreIcon,
} from '@mui/icons-material';
import { useForm, Controller, useFieldArray, type FieldValues } from 'react-hook-form';
import type { 
  EdgeResponse, 
  CreateEdgeRequest, 
  UpdateEdgeRequest, 
  EdgeType, 
  PropertyType,
  NodeResponse,
  NodeId 
} from '../../types/graph';

interface EdgeManagementDialogProps {
  open: boolean;
  onClose: () => void;
  onSubmit: (data: CreateEdgeRequest | UpdateEdgeRequest) => Promise<void>;
  edge?: EdgeResponse | null; // null for create, EdgeResponse for edit
  nodes: NodeResponse[]; // Available nodes for source/target selection
  loading?: boolean;
  error?: string | null;
}

interface EdgeFormData {
  source_id: NodeId;
  target_id: NodeId;
  edge_type: EdgeType;
  weight: number;
  properties: Array<{
    key: string;
    value: string;
    type: PropertyType;
  }>;
}

const EDGE_TYPES: EdgeType[] = ['Contains', 'References', 'DependsOn', 'SimilarTo', 'Custom'];
const PROPERTY_TYPES: PropertyType[] = ['String', 'Integer', 'Float', 'Boolean', 'Array', 'Object'];

const EdgeManagementDialog: React.FC<EdgeManagementDialogProps> = ({
  open,
  onClose,
  onSubmit,
  edge = null,
  nodes,
  loading = false,
  error = null,
}) => {
  const [submitError, setSubmitError] = useState<string | null>(null);
  const isEditing = !!edge;

  // Convert edge properties to form format
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
  } = useForm<EdgeFormData>({
    defaultValues: {
      source_id: edge?.source_id || '',
      target_id: edge?.target_id || '',
      edge_type: edge?.edge_type || 'Contains',
      weight: edge?.weight || 1.0,
      properties: edge ? convertPropertiesToForm(edge.properties) : [],
    },
    mode: 'onChange',
  });

  const { fields, append, remove } = useFieldArray({
    control,
    name: 'properties',
  });

  // Reset form when edge changes
  useEffect(() => {
    if (open) {
      reset({
        source_id: edge?.source_id || '',
        target_id: edge?.target_id || '',
        edge_type: edge?.edge_type || 'Contains',
        weight: edge?.weight || 1.0,
        properties: edge ? convertPropertiesToForm(edge.properties) : [],
      });
      setSubmitError(null);
    }
  }, [edge, open, reset]);

  const handleFormSubmit = async (data: EdgeFormData) => {
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
        // Update existing edge
        const updateData: UpdateEdgeRequest = { 
          weight: data.weight,
          properties 
        };
        await onSubmit(updateData);
      } else {
        // Create new edge
        const createData: CreateEdgeRequest = {
          source_id: data.source_id,
          target_id: data.target_id,
          edge_type: data.edge_type,
          weight: data.weight,
          properties,
        };
        await onSubmit(createData);
      }

      handleClose();
    } catch (err) {
      setSubmitError(
        err instanceof Error ? err.message : `Failed to ${isEditing ? 'update' : 'create'} edge`
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

  const validateWeight = (value: number) => {
    if (isNaN(value) || value < 0) {
      return 'Weight must be a non-negative number';
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

  const validateNodeSelection = (value: string) => {
    if (!value) {
      return 'Node selection is required';
    }
    return true;
  };

  const validateDifferentNodes = (targetId: string) => {
    const sourceId = watch('source_id');
    if (sourceId && targetId && sourceId === targetId) {
      return 'Source and target nodes must be different';
    }
    return true;
  };

  // Create node options for autocomplete
  const nodeOptions = nodes.map(node => ({
    id: node.id,
    label: `${node.id} (${node.node_type}) - ${node.properties?.name || node.properties?.path || 'No name'}`,
    node,
  }));

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
          {isEditing ? 'Edit Edge' : 'Create New Edge'}
        </Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
          {isEditing 
            ? `Modify properties for edge ${edge?.id}`
            : 'Create a new edge connecting two nodes in the VexGraph.'
          }
        </Typography>
      </DialogTitle>

      <form onSubmit={handleSubmit(handleFormSubmit)}>
        <DialogContent>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
            {(error || submitError) && (
              <Alert severity="error">{error || submitError}</Alert>
            )}

            {/* Node Selection - only for creation */}
            {!isEditing && (
              <Box sx={{ display: 'flex', gap: 2 }}>
                {/* Source Node */}
                <Controller
                  name="source_id"
                  control={control}
                  rules={{ validate: validateNodeSelection }}
                  render={({ field }: { field: FieldValues }) => (
                    <Autocomplete
                      {...field}
                      options={nodeOptions}
                      getOptionLabel={(option) => option.label}
                      isOptionEqualToValue={(option, value) => option.id === value.id}
                      onChange={(_, value) => field.onChange(value?.id || '')}
                      value={nodeOptions.find(opt => opt.id === field.value) || null}
                      renderInput={(params) => (
                        <TextField
                          {...params}
                          label="Source Node"
                          placeholder="Select source node"
                          error={!!errors.source_id}
                          helperText={errors.source_id?.message || 'Node where the edge starts'}
                          required
                        />
                      )}
                      sx={{ flex: 1 }}
                    />
                  )}
                />

                {/* Target Node */}
                <Controller
                  name="target_id"
                  control={control}
                  rules={{ 
                    validate: (value) => validateNodeSelection(value) || validateDifferentNodes(value)
                  }}
                  render={({ field }: { field: FieldValues }) => (
                    <Autocomplete
                      {...field}
                      options={nodeOptions}
                      getOptionLabel={(option) => option.label}
                      isOptionEqualToValue={(option, value) => option.id === value.id}
                      onChange={(_, value) => field.onChange(value?.id || '')}
                      value={nodeOptions.find(opt => opt.id === field.value) || null}
                      renderInput={(params) => (
                        <TextField
                          {...params}
                          label="Target Node"
                          placeholder="Select target node"
                          error={!!errors.target_id}
                          helperText={errors.target_id?.message || 'Node where the edge ends'}
                          required
                        />
                      )}
                      sx={{ flex: 1 }}
                    />
                  )}
                />
              </Box>
            )}

            {/* Edge Information */}
            <Box sx={{ display: 'flex', gap: 2 }}>
              {/* Edge Type */}
              <Controller
                name="edge_type"
                control={control}
                render={({ field }: { field: FieldValues }) => (
                  <FormControl sx={{ flex: 1 }} disabled={isEditing}>
                    <InputLabel>Edge Type</InputLabel>
                    <Select {...field} label="Edge Type">
                      {EDGE_TYPES.map(type => (
                        <MenuItem key={type} value={type}>
                          <Box>
                            <Typography variant="body2" sx={{ fontWeight: 500 }}>
                              {type}
                            </Typography>
                            <Typography variant="caption" color="text.secondary">
                              {type === 'Contains' && 'Parent-child relationship'}
                              {type === 'References' && 'Reference or link relationship'}
                              {type === 'DependsOn' && 'Dependency relationship'}
                              {type === 'SimilarTo' && 'Similarity relationship'}
                              {type === 'Custom' && 'Custom relationship type'}
                            </Typography>
                          </Box>
                        </MenuItem>
                      ))}
                    </Select>
                  </FormControl>
                )}
              />

              {/* Weight */}
              <Controller
                name="weight"
                control={control}
                rules={{ validate: validateWeight }}
                render={({ field }: { field: FieldValues }) => (
                  <TextField
                    {...field}
                    label="Weight"
                    type="number"
                    placeholder="1.0"
                    error={!!errors.weight}
                    helperText={
                      errors.weight?.message ||
                      'Edge weight for algorithms (default: 1.0)'
                    }
                    sx={{ flex: 1 }}
                    inputProps={{ step: 0.1, min: 0 }}
                    onChange={e => field.onChange(parseFloat(e.target.value))}
                  />
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
                <strong>Edge Properties:</strong> Edges connect two nodes and can have custom properties 
                and weights for graph algorithms.
                {isEditing && ' Source, target, and edge type cannot be changed after creation.'}
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
              : (isEditing ? 'Update Edge' : 'Create Edge')
            }
          </Button>
        </DialogActions>
      </form>
    </Dialog>
  );
};

export default EdgeManagementDialog;