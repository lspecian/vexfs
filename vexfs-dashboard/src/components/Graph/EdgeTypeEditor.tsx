import React, { useState, useEffect } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  TextField,
  Box,
  Typography,
  IconButton,
  Chip,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Switch,
  FormControlLabel,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  Grid,
  Alert,
  Autocomplete,
  Checkbox,
  ListItemText,
} from '@mui/material';
import {
  Add as AddIcon,
  Delete as DeleteIcon,
  ExpandMore as ExpandMoreIcon,
  ArrowForward as DirectedIcon,
  SwapHoriz as UndirectedIcon,
  CompareArrows as BidirectionalIcon,
} from '@mui/icons-material';
import type { 
  EdgeTypeDefinition, 
  NodeTypeDefinition, 
  PropertyDefinition, 
  PropertyConstraint 
} from '../../types/schema';

interface EdgeTypeEditorProps {
  open: boolean;
  edgeType: EdgeTypeDefinition | null;
  existingEdgeTypes: EdgeTypeDefinition[];
  availableNodeTypes: NodeTypeDefinition[];
  onSave: (edgeType: EdgeTypeDefinition) => void;
  onCancel: () => void;
}

const EdgeTypeEditor: React.FC<EdgeTypeEditorProps> = ({
  open,
  edgeType,
  existingEdgeTypes,
  availableNodeTypes,
  onSave,
  onCancel,
}) => {
  const [formData, setFormData] = useState<EdgeTypeDefinition>({
    id: '',
    name: '',
    displayName: '',
    description: '',
    directionality: 'directed',
    allowedSourceTypes: [],
    allowedTargetTypes: [],
    properties: [],
    cardinality: 'many-to-many',
    weightConstraints: {
      required: false,
      min: 0,
      max: 1,
      defaultValue: 1,
    },
    validationRules: [],
  });

  const [errors, setErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    if (edgeType) {
      setFormData(edgeType);
    } else {
      setFormData({
        id: '',
        name: '',
        displayName: '',
        description: '',
        directionality: 'directed',
        allowedSourceTypes: [],
        allowedTargetTypes: [],
        properties: [],
        cardinality: 'many-to-many',
        weightConstraints: {
          required: false,
          min: 0,
          max: 1,
          defaultValue: 1,
        },
        validationRules: [],
      });
    }
    setErrors({});
  }, [edgeType, open]);

  const validateForm = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!formData.name.trim()) {
      newErrors.name = 'Name is required';
    } else if (existingEdgeTypes.some(et => et.name === formData.name && et.id !== formData.id)) {
      newErrors.name = 'Name already exists';
    }

    if (!formData.displayName.trim()) {
      newErrors.displayName = 'Display name is required';
    }

    if (!formData.description.trim()) {
      newErrors.description = 'Description is required';
    }

    if (formData.allowedSourceTypes.length === 0) {
      newErrors.allowedSourceTypes = 'At least one source type is required';
    }

    if (formData.allowedTargetTypes.length === 0) {
      newErrors.allowedTargetTypes = 'At least one target type is required';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSave = () => {
    if (!validateForm()) return;

    const edgeTypeToSave: EdgeTypeDefinition = {
      ...formData,
      id: formData.id || `edge_${Date.now()}`,
    };

    onSave(edgeTypeToSave);
  };

  const handleAddProperty = () => {
    const newProperty: PropertyDefinition = {
      name: '',
      type: 'String',
      constraints: [{ type: 'optional' }],
      description: '',
      examples: [],
    };

    setFormData(prev => ({
      ...prev,
      properties: [...prev.properties, newProperty],
    }));
  };

  const handleUpdateProperty = (index: number, property: PropertyDefinition) => {
    setFormData(prev => ({
      ...prev,
      properties: prev.properties.map((p, i) => i === index ? property : p),
    }));
  };

  const handleRemoveProperty = (index: number) => {
    setFormData(prev => ({
      ...prev,
      properties: prev.properties.filter((_, i) => i !== index),
    }));
  };

  const getDirectionalityIcon = (directionality: string) => {
    switch (directionality) {
      case 'directed':
        return <DirectedIcon />;
      case 'undirected':
        return <UndirectedIcon />;
      case 'bidirectional':
        return <BidirectionalIcon />;
      default:
        return <DirectedIcon />;
    }
  };

  const nodeTypeOptions = availableNodeTypes.map(nt => nt.name);

  return (
    <Dialog open={open} onClose={onCancel} maxWidth="md" fullWidth>
      <DialogTitle>
        {edgeType ? 'Edit Edge Type' : 'Create Edge Type'}
      </DialogTitle>
      <DialogContent>
        <Box sx={{ mt: 2 }}>
          <Grid container spacing={2}>
            <Grid item xs={12} md={6}>
              <TextField
                fullWidth
                label="Name"
                value={formData.name}
                onChange={(e) => setFormData(prev => ({ ...prev, name: e.target.value }))}
                error={!!errors.name}
                helperText={errors.name}
                required
              />
            </Grid>
            <Grid item xs={12} md={6}>
              <TextField
                fullWidth
                label="Display Name"
                value={formData.displayName}
                onChange={(e) => setFormData(prev => ({ ...prev, displayName: e.target.value }))}
                error={!!errors.displayName}
                helperText={errors.displayName}
                required
              />
            </Grid>
            <Grid item xs={12}>
              <TextField
                fullWidth
                label="Description"
                value={formData.description}
                onChange={(e) => setFormData(prev => ({ ...prev, description: e.target.value }))}
                error={!!errors.description}
                helperText={errors.description}
                multiline
                rows={2}
                required
              />
            </Grid>
            <Grid item xs={12} md={6}>
              <FormControl fullWidth>
                <InputLabel>Directionality</InputLabel>
                <Select
                  value={formData.directionality}
                  label="Directionality"
                  onChange={(e) => setFormData(prev => ({ 
                    ...prev, 
                    directionality: e.target.value as any 
                  }))}
                  startAdornment={getDirectionalityIcon(formData.directionality)}
                >
                  <MenuItem value="directed">
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      <DirectedIcon />
                      Directed
                    </Box>
                  </MenuItem>
                  <MenuItem value="undirected">
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      <UndirectedIcon />
                      Undirected
                    </Box>
                  </MenuItem>
                  <MenuItem value="bidirectional">
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      <BidirectionalIcon />
                      Bidirectional
                    </Box>
                  </MenuItem>
                </Select>
              </FormControl>
            </Grid>
            <Grid item xs={12} md={6}>
              <FormControl fullWidth>
                <InputLabel>Cardinality</InputLabel>
                <Select
                  value={formData.cardinality}
                  label="Cardinality"
                  onChange={(e) => setFormData(prev => ({ 
                    ...prev, 
                    cardinality: e.target.value as any 
                  }))}
                >
                  <MenuItem value="one-to-one">One-to-One</MenuItem>
                  <MenuItem value="one-to-many">One-to-Many</MenuItem>
                  <MenuItem value="many-to-many">Many-to-Many</MenuItem>
                </Select>
              </FormControl>
            </Grid>
            <Grid item xs={12} md={6}>
              <Autocomplete
                multiple
                options={nodeTypeOptions}
                value={formData.allowedSourceTypes}
                onChange={(_, newValue) => setFormData(prev => ({ 
                  ...prev, 
                  allowedSourceTypes: newValue 
                }))}
                renderInput={(params) => (
                  <TextField
                    {...params}
                    label="Allowed Source Types"
                    error={!!errors.allowedSourceTypes}
                    helperText={errors.allowedSourceTypes}
                    required
                  />
                )}
                renderTags={(value, getTagProps) =>
                  value.map((option, index) => (
                    <Chip
                      variant="outlined"
                      label={option}
                      {...getTagProps({ index })}
                      key={option}
                    />
                  ))
                }
              />
            </Grid>
            <Grid item xs={12} md={6}>
              <Autocomplete
                multiple
                options={nodeTypeOptions}
                value={formData.allowedTargetTypes}
                onChange={(_, newValue) => setFormData(prev => ({ 
                  ...prev, 
                  allowedTargetTypes: newValue 
                }))}
                renderInput={(params) => (
                  <TextField
                    {...params}
                    label="Allowed Target Types"
                    error={!!errors.allowedTargetTypes}
                    helperText={errors.allowedTargetTypes}
                    required
                  />
                )}
                renderTags={(value, getTagProps) =>
                  value.map((option, index) => (
                    <Chip
                      variant="outlined"
                      label={option}
                      {...getTagProps({ index })}
                      key={option}
                    />
                  ))
                }
              />
            </Grid>
          </Grid>

          {/* Weight Constraints */}
          <Box sx={{ mt: 3 }}>
            <Typography variant="h6" gutterBottom>
              Weight Constraints
            </Typography>
            <Grid container spacing={2}>
              <Grid item xs={12}>
                <FormControlLabel
                  control={
                    <Switch
                      checked={formData.weightConstraints?.required || false}
                      onChange={(e) => setFormData(prev => ({
                        ...prev,
                        weightConstraints: {
                          ...prev.weightConstraints,
                          required: e.target.checked,
                        },
                      }))}
                    />
                  }
                  label="Weight Required"
                />
              </Grid>
              {formData.weightConstraints?.required && (
                <>
                  <Grid item xs={12} md={4}>
                    <TextField
                      fullWidth
                      label="Minimum Weight"
                      type="number"
                      value={formData.weightConstraints?.min || 0}
                      onChange={(e) => setFormData(prev => ({
                        ...prev,
                        weightConstraints: {
                          required: prev.weightConstraints?.required || false,
                          min: parseFloat(e.target.value),
                          max: prev.weightConstraints?.max,
                          defaultValue: prev.weightConstraints?.defaultValue,
                        },
                      }))}
                    />
                  </Grid>
                  <Grid item xs={12} md={4}>
                    <TextField
                      fullWidth
                      label="Maximum Weight"
                      type="number"
                      value={formData.weightConstraints?.max || 1}
                      onChange={(e) => setFormData(prev => ({
                        ...prev,
                        weightConstraints: {
                          required: prev.weightConstraints?.required || false,
                          min: prev.weightConstraints?.min,
                          max: parseFloat(e.target.value),
                          defaultValue: prev.weightConstraints?.defaultValue,
                        },
                      }))}
                    />
                  </Grid>
                  <Grid item xs={12} md={4}>
                    <TextField
                      fullWidth
                      label="Default Weight"
                      type="number"
                      value={formData.weightConstraints?.defaultValue || 1}
                      onChange={(e) => setFormData(prev => ({
                        ...prev,
                        weightConstraints: {
                          required: prev.weightConstraints?.required || false,
                          min: prev.weightConstraints?.min,
                          max: prev.weightConstraints?.max,
                          defaultValue: parseFloat(e.target.value),
                        },
                      }))}
                    />
                  </Grid>
                </>
              )}
            </Grid>
          </Box>

          {/* Properties */}
          <Box sx={{ mt: 3 }}>
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
              <Typography variant="h6">Properties</Typography>
              <Button
                startIcon={<AddIcon />}
                onClick={handleAddProperty}
                variant="outlined"
                size="small"
              >
                Add Property
              </Button>
            </Box>

            {formData.properties.map((property, propertyIndex) => (
              <Accordion key={propertyIndex} sx={{ mb: 1 }}>
                <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, width: '100%' }}>
                    <Typography>
                      {property.name || `Property ${propertyIndex + 1}`}
                    </Typography>
                    <Chip label={property.type} size="small" />
                    {property.constraints.some(c => c.type === 'required') && (
                      <Chip label="Required" size="small" color="primary" />
                    )}
                    <Box sx={{ flexGrow: 1 }} />
                    <IconButton
                      size="small"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleRemoveProperty(propertyIndex);
                      }}
                    >
                      <DeleteIcon />
                    </IconButton>
                  </Box>
                </AccordionSummary>
                <AccordionDetails>
                  <Grid container spacing={2}>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        label="Property Name"
                        value={property.name}
                        onChange={(e) => handleUpdateProperty(propertyIndex, {
                          ...property,
                          name: e.target.value,
                        })}
                        required
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <FormControl fullWidth>
                        <InputLabel>Type</InputLabel>
                        <Select
                          value={property.type}
                          label="Type"
                          onChange={(e) => handleUpdateProperty(propertyIndex, {
                            ...property,
                            type: e.target.value as any,
                          })}
                        >
                          <MenuItem value="String">String</MenuItem>
                          <MenuItem value="Number">Number</MenuItem>
                          <MenuItem value="Boolean">Boolean</MenuItem>
                          <MenuItem value="Date">Date</MenuItem>
                          <MenuItem value="Array">Array</MenuItem>
                          <MenuItem value="Object">Object</MenuItem>
                          <MenuItem value="Reference">Reference</MenuItem>
                        </Select>
                      </FormControl>
                    </Grid>
                    <Grid item xs={12}>
                      <TextField
                        fullWidth
                        label="Description"
                        value={property.description}
                        onChange={(e) => handleUpdateProperty(propertyIndex, {
                          ...property,
                          description: e.target.value,
                        })}
                        multiline
                        rows={2}
                      />
                    </Grid>
                  </Grid>
                </AccordionDetails>
              </Accordion>
            ))}

            {formData.properties.length === 0 && (
              <Alert severity="info">
                No properties defined. Click "Add Property" to define additional data for this edge type.
              </Alert>
            )}
          </Box>
        </Box>
      </DialogContent>
      <DialogActions>
        <Button onClick={onCancel}>Cancel</Button>
        <Button onClick={handleSave} variant="contained">
          {edgeType ? 'Update' : 'Create'}
        </Button>
      </DialogActions>
    </Dialog>
  );
};

export default EdgeTypeEditor;