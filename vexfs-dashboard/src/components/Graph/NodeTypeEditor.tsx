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
} from '@mui/material';
import {
  Add as AddIcon,
  Delete as DeleteIcon,
  ExpandMore as ExpandMoreIcon,
  ColorLens as ColorIcon,
} from '@mui/icons-material';
import type { NodeTypeDefinition, PropertyDefinition, PropertyConstraint } from '../../types/schema';

interface NodeTypeEditorProps {
  open: boolean;
  nodeType: NodeTypeDefinition | null;
  existingNodeTypes: NodeTypeDefinition[];
  onSave: (nodeType: NodeTypeDefinition) => void;
  onCancel: () => void;
}

const NodeTypeEditor: React.FC<NodeTypeEditorProps> = ({
  open,
  nodeType,
  existingNodeTypes,
  onSave,
  onCancel,
}) => {
  const [formData, setFormData] = useState<NodeTypeDefinition>({
    id: '',
    name: '',
    displayName: '',
    description: '',
    icon: '',
    color: '#1976d2',
    properties: [],
    inheritFrom: [],
    validationRules: [],
    indexHints: [],
  });

  const [errors, setErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    if (nodeType) {
      setFormData(nodeType);
    } else {
      setFormData({
        id: '',
        name: '',
        displayName: '',
        description: '',
        icon: '',
        color: '#1976d2',
        properties: [],
        inheritFrom: [],
        validationRules: [],
        indexHints: [],
      });
    }
    setErrors({});
  }, [nodeType, open]);

  const validateForm = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!formData.name.trim()) {
      newErrors.name = 'Name is required';
    } else if (existingNodeTypes.some(nt => nt.name === formData.name && nt.id !== formData.id)) {
      newErrors.name = 'Name already exists';
    }

    if (!formData.displayName.trim()) {
      newErrors.displayName = 'Display name is required';
    }

    if (!formData.description.trim()) {
      newErrors.description = 'Description is required';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSave = () => {
    if (!validateForm()) return;

    const nodeTypeToSave: NodeTypeDefinition = {
      ...formData,
      id: formData.id || `node_${Date.now()}`,
    };

    onSave(nodeTypeToSave);
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

  const handleAddConstraint = (propertyIndex: number) => {
    const newConstraint: PropertyConstraint = {
      type: 'optional',
    };

    setFormData(prev => ({
      ...prev,
      properties: prev.properties.map((p, i) => 
        i === propertyIndex 
          ? { ...p, constraints: [...p.constraints, newConstraint] }
          : p
      ),
    }));
  };

  const handleUpdateConstraint = (
    propertyIndex: number, 
    constraintIndex: number, 
    constraint: PropertyConstraint
  ) => {
    setFormData(prev => ({
      ...prev,
      properties: prev.properties.map((p, i) => 
        i === propertyIndex 
          ? {
              ...p,
              constraints: p.constraints.map((c, ci) => 
                ci === constraintIndex ? constraint : c
              ),
            }
          : p
      ),
    }));
  };

  const handleRemoveConstraint = (propertyIndex: number, constraintIndex: number) => {
    setFormData(prev => ({
      ...prev,
      properties: prev.properties.map((p, i) => 
        i === propertyIndex 
          ? {
              ...p,
              constraints: p.constraints.filter((_, ci) => ci !== constraintIndex),
            }
          : p
      ),
    }));
  };

  return (
    <Dialog open={open} onClose={onCancel} maxWidth="md" fullWidth>
      <DialogTitle>
        {nodeType ? 'Edit Node Type' : 'Create Node Type'}
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
              <TextField
                fullWidth
                label="Icon"
                value={formData.icon}
                onChange={(e) => setFormData(prev => ({ ...prev, icon: e.target.value }))}
                placeholder="e.g., folder, file, person"
              />
            </Grid>
            <Grid item xs={12} md={6}>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <TextField
                  fullWidth
                  label="Color"
                  type="color"
                  value={formData.color}
                  onChange={(e) => setFormData(prev => ({ ...prev, color: e.target.value }))}
                />
                <ColorIcon sx={{ color: formData.color }} />
              </Box>
            </Grid>
          </Grid>

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
                    <Grid item xs={12}>
                      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 1 }}>
                        <Typography variant="subtitle2">Constraints</Typography>
                        <Button
                          size="small"
                          onClick={() => handleAddConstraint(propertyIndex)}
                        >
                          Add Constraint
                        </Button>
                      </Box>
                      {property.constraints.map((constraint, constraintIndex) => (
                        <Box key={constraintIndex} sx={{ display: 'flex', gap: 1, mb: 1, alignItems: 'center' }}>
                          <FormControl size="small" sx={{ minWidth: 120 }}>
                            <Select
                              value={constraint.type}
                              onChange={(e) => handleUpdateConstraint(propertyIndex, constraintIndex, {
                                ...constraint,
                                type: e.target.value as any,
                              })}
                            >
                              <MenuItem value="required">Required</MenuItem>
                              <MenuItem value="optional">Optional</MenuItem>
                              <MenuItem value="unique">Unique</MenuItem>
                              <MenuItem value="indexed">Indexed</MenuItem>
                            </Select>
                          </FormControl>
                          {constraint.type === 'required' && (
                            <TextField
                              size="small"
                              label="Default Value"
                              value={constraint.defaultValue || ''}
                              onChange={(e) => handleUpdateConstraint(propertyIndex, constraintIndex, {
                                ...constraint,
                                defaultValue: e.target.value,
                              })}
                            />
                          )}
                          <IconButton
                            size="small"
                            onClick={() => handleRemoveConstraint(propertyIndex, constraintIndex)}
                          >
                            <DeleteIcon />
                          </IconButton>
                        </Box>
                      ))}
                    </Grid>
                  </Grid>
                </AccordionDetails>
              </Accordion>
            ))}

            {formData.properties.length === 0 && (
              <Alert severity="info">
                No properties defined. Click "Add Property" to define the structure of this node type.
              </Alert>
            )}
          </Box>
        </Box>
      </DialogContent>
      <DialogActions>
        <Button onClick={onCancel}>Cancel</Button>
        <Button onClick={handleSave} variant="contained">
          {nodeType ? 'Update' : 'Create'}
        </Button>
      </DialogActions>
    </Dialog>
  );
};

export default NodeTypeEditor;