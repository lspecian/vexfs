import React, { useState } from 'react';
import {
  Box,
  Typography,
  Card,
  CardContent,
  CardActions,
  Button,
  TextField,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Chip,
  IconButton,
  Grid,
  Switch,
  FormControlLabel,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  Alert,
  Tooltip,
  Divider,
} from '@mui/material';
import {
  Add as AddIcon,
  Delete as DeleteIcon,
  Edit as EditIcon,
  ExpandMore as ExpandMoreIcon,
  Save as SaveIcon,
  Cancel as CancelIcon,
  Code as CodeIcon,
  Visibility as PreviewIcon,
} from '@mui/icons-material';
import type { PropertyDefinition, PropertyConstraint } from '../../types/schema';

interface PropertySchemaBuilderProps {
  properties: PropertyDefinition[];
  onChange: (properties: PropertyDefinition[]) => void;
  readonly?: boolean;
}

const PropertySchemaBuilder: React.FC<PropertySchemaBuilderProps> = ({
  properties,
  onChange,
  readonly = false,
}) => {
  const [editingProperty, setEditingProperty] = useState<number | null>(null);
  const [previewMode, setPreviewMode] = useState(false);

  const handleAddProperty = () => {
    const newProperty: PropertyDefinition = {
      name: '',
      type: 'String',
      constraints: [{ type: 'optional' }],
      description: '',
      examples: [],
    };

    onChange([...properties, newProperty]);
    setEditingProperty(properties.length);
  };

  const handleUpdateProperty = (index: number, property: PropertyDefinition) => {
    const updatedProperties = properties.map((p, i) => i === index ? property : p);
    onChange(updatedProperties);
  };

  const handleRemoveProperty = (index: number) => {
    const updatedProperties = properties.filter((_, i) => i !== index);
    onChange(updatedProperties);
    if (editingProperty === index) {
      setEditingProperty(null);
    }
  };

  const handleAddConstraint = (propertyIndex: number) => {
    const property = properties[propertyIndex];
    const newConstraint: PropertyConstraint = {
      type: 'optional',
    };

    handleUpdateProperty(propertyIndex, {
      ...property,
      constraints: [...property.constraints, newConstraint],
    });
  };

  const handleUpdateConstraint = (
    propertyIndex: number,
    constraintIndex: number,
    constraint: PropertyConstraint
  ) => {
    const property = properties[propertyIndex];
    const updatedConstraints = property.constraints.map((c, i) =>
      i === constraintIndex ? constraint : c
    );

    handleUpdateProperty(propertyIndex, {
      ...property,
      constraints: updatedConstraints,
    });
  };

  const handleRemoveConstraint = (propertyIndex: number, constraintIndex: number) => {
    const property = properties[propertyIndex];
    const updatedConstraints = property.constraints.filter((_, i) => i !== constraintIndex);

    handleUpdateProperty(propertyIndex, {
      ...property,
      constraints: updatedConstraints,
    });
  };

  const getTypeIcon = (type: string) => {
    switch (type) {
      case 'String':
        return '📝';
      case 'Number':
        return '🔢';
      case 'Boolean':
        return '✅';
      case 'Date':
        return '📅';
      case 'Array':
        return '📋';
      case 'Object':
        return '📦';
      case 'Reference':
        return '🔗';
      default:
        return '❓';
    }
  };

  const getConstraintColor = (type: string) => {
    switch (type) {
      case 'required':
        return 'error';
      case 'unique':
        return 'warning';
      case 'indexed':
        return 'info';
      default:
        return 'default';
    }
  };

  const generateJsonSchema = () => {
    const schema = {
      type: 'object',
      properties: {},
      required: [] as string[],
    };

    properties.forEach(prop => {
      const propSchema: any = {};
      
      switch (prop.type) {
        case 'String':
          propSchema.type = 'string';
          break;
        case 'Number':
          propSchema.type = 'number';
          break;
        case 'Boolean':
          propSchema.type = 'boolean';
          break;
        case 'Date':
          propSchema.type = 'string';
          propSchema.format = 'date-time';
          break;
        case 'Array':
          propSchema.type = 'array';
          break;
        case 'Object':
          propSchema.type = 'object';
          break;
        case 'Reference':
          propSchema.type = 'string';
          propSchema.format = 'uuid';
          break;
      }

      if (prop.description) {
        propSchema.description = prop.description;
      }

      if (prop.examples && prop.examples.length > 0) {
        propSchema.examples = prop.examples;
      }

      // Apply constraints
      prop.constraints.forEach(constraint => {
        switch (constraint.type) {
          case 'required':
            (schema.required as string[]).push(prop.name);
            if (constraint.defaultValue !== undefined) {
              propSchema.default = constraint.defaultValue;
            }
            break;
          case 'unique':
            propSchema.uniqueItems = true;
            break;
          case 'indexed':
            propSchema['x-index'] = true;
            break;
        }

        if (constraint.minLength !== undefined) {
          propSchema.minLength = constraint.minLength;
        }
        if (constraint.maxLength !== undefined) {
          propSchema.maxLength = constraint.maxLength;
        }
        if (constraint.minValue !== undefined) {
          propSchema.minimum = constraint.minValue;
        }
        if (constraint.maxValue !== undefined) {
          propSchema.maximum = constraint.maxValue;
        }
        if (constraint.pattern) {
          propSchema.pattern = constraint.pattern;
        }
        if (constraint.enum) {
          propSchema.enum = constraint.enum;
        }
      });

      (schema.properties as any)[prop.name] = propSchema;
    });

    return schema;
  };

  const renderPropertyEditor = (property: PropertyDefinition, index: number) => {
    const isEditing = editingProperty === index;

    return (
      <Accordion key={index} expanded={isEditing}>
        <AccordionSummary
          expandIcon={<ExpandMoreIcon />}
          onClick={() => setEditingProperty(isEditing ? null : index)}
        >
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, width: '100%' }}>
            <Typography sx={{ fontSize: '1.2em' }}>
              {getTypeIcon(property.type)}
            </Typography>
            <Typography variant="subtitle1">
              {property.name || `Property ${index + 1}`}
            </Typography>
            <Chip label={property.type} size="small" />
            {property.constraints.map((constraint, ci) => (
              <Chip
                key={ci}
                label={constraint.type}
                size="small"
                color={getConstraintColor(constraint.type) as any}
                variant="outlined"
              />
            ))}
            <Box sx={{ flexGrow: 1 }} />
            {!readonly && (
              <IconButton
                size="small"
                onClick={(e) => {
                  e.stopPropagation();
                  handleRemoveProperty(index);
                }}
              >
                <DeleteIcon />
              </IconButton>
            )}
          </Box>
        </AccordionSummary>
        <AccordionDetails>
          <Grid container spacing={2}>
            <Grid item xs={12} md={6}>
              <TextField
                fullWidth
                label="Property Name"
                value={property.name}
                onChange={(e) => handleUpdateProperty(index, {
                  ...property,
                  name: e.target.value,
                })}
                disabled={readonly}
                required
              />
            </Grid>
            <Grid item xs={12} md={6}>
              <FormControl fullWidth>
                <InputLabel>Type</InputLabel>
                <Select
                  value={property.type}
                  label="Type"
                  onChange={(e) => handleUpdateProperty(index, {
                    ...property,
                    type: e.target.value as any,
                  })}
                  disabled={readonly}
                >
                  <MenuItem value="String">
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      📝 String
                    </Box>
                  </MenuItem>
                  <MenuItem value="Number">
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      🔢 Number
                    </Box>
                  </MenuItem>
                  <MenuItem value="Boolean">
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      ✅ Boolean
                    </Box>
                  </MenuItem>
                  <MenuItem value="Date">
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      📅 Date
                    </Box>
                  </MenuItem>
                  <MenuItem value="Array">
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      📋 Array
                    </Box>
                  </MenuItem>
                  <MenuItem value="Object">
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      📦 Object
                    </Box>
                  </MenuItem>
                  <MenuItem value="Reference">
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      🔗 Reference
                    </Box>
                  </MenuItem>
                </Select>
              </FormControl>
            </Grid>
            <Grid item xs={12}>
              <TextField
                fullWidth
                label="Description"
                value={property.description}
                onChange={(e) => handleUpdateProperty(index, {
                  ...property,
                  description: e.target.value,
                })}
                disabled={readonly}
                multiline
                rows={2}
              />
            </Grid>
            <Grid item xs={12}>
              <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 1 }}>
                <Typography variant="subtitle2">Constraints</Typography>
                {!readonly && (
                  <Button
                    size="small"
                    onClick={() => handleAddConstraint(index)}
                    startIcon={<AddIcon />}
                  >
                    Add Constraint
                  </Button>
                )}
              </Box>
              {property.constraints.map((constraint, constraintIndex) => (
                <Card key={constraintIndex} variant="outlined" sx={{ mb: 1, p: 1 }}>
                  <Grid container spacing={1} alignItems="center">
                    <Grid item xs={12} md={3}>
                      <FormControl fullWidth size="small">
                        <InputLabel>Type</InputLabel>
                        <Select
                          value={constraint.type}
                          label="Type"
                          onChange={(e) => handleUpdateConstraint(index, constraintIndex, {
                            ...constraint,
                            type: e.target.value as any,
                          })}
                          disabled={readonly}
                        >
                          <MenuItem value="required">Required</MenuItem>
                          <MenuItem value="optional">Optional</MenuItem>
                          <MenuItem value="unique">Unique</MenuItem>
                          <MenuItem value="indexed">Indexed</MenuItem>
                        </Select>
                      </FormControl>
                    </Grid>
                    {(constraint.type === 'required' || property.type === 'String') && (
                      <>
                        <Grid item xs={12} md={3}>
                          <TextField
                            fullWidth
                            size="small"
                            label="Min Length"
                            type="number"
                            value={constraint.minLength || ''}
                            onChange={(e) => handleUpdateConstraint(index, constraintIndex, {
                              ...constraint,
                              minLength: e.target.value ? parseInt(e.target.value) : undefined,
                            })}
                            disabled={readonly}
                          />
                        </Grid>
                        <Grid item xs={12} md={3}>
                          <TextField
                            fullWidth
                            size="small"
                            label="Max Length"
                            type="number"
                            value={constraint.maxLength || ''}
                            onChange={(e) => handleUpdateConstraint(index, constraintIndex, {
                              ...constraint,
                              maxLength: e.target.value ? parseInt(e.target.value) : undefined,
                            })}
                            disabled={readonly}
                          />
                        </Grid>
                      </>
                    )}
                    {property.type === 'Number' && (
                      <>
                        <Grid item xs={12} md={3}>
                          <TextField
                            fullWidth
                            size="small"
                            label="Min Value"
                            type="number"
                            value={constraint.minValue || ''}
                            onChange={(e) => handleUpdateConstraint(index, constraintIndex, {
                              ...constraint,
                              minValue: e.target.value ? parseFloat(e.target.value) : undefined,
                            })}
                            disabled={readonly}
                          />
                        </Grid>
                        <Grid item xs={12} md={3}>
                          <TextField
                            fullWidth
                            size="small"
                            label="Max Value"
                            type="number"
                            value={constraint.maxValue || ''}
                            onChange={(e) => handleUpdateConstraint(index, constraintIndex, {
                              ...constraint,
                              maxValue: e.target.value ? parseFloat(e.target.value) : undefined,
                            })}
                            disabled={readonly}
                          />
                        </Grid>
                      </>
                    )}
                    <Grid item xs={12} md={3}>
                      <TextField
                        fullWidth
                        size="small"
                        label="Default Value"
                        value={constraint.defaultValue || ''}
                        onChange={(e) => handleUpdateConstraint(index, constraintIndex, {
                          ...constraint,
                          defaultValue: e.target.value,
                        })}
                        disabled={readonly}
                      />
                    </Grid>
                    {!readonly && (
                      <Grid item xs={12} md={1}>
                        <IconButton
                          size="small"
                          onClick={() => handleRemoveConstraint(index, constraintIndex)}
                        >
                          <DeleteIcon />
                        </IconButton>
                      </Grid>
                    )}
                  </Grid>
                </Card>
              ))}
            </Grid>
          </Grid>
        </AccordionDetails>
      </Accordion>
    );
  };

  return (
    <Box>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
        <Typography variant="h6">Property Schema Builder</Typography>
        <Box sx={{ display: 'flex', gap: 1 }}>
          <FormControlLabel
            control={
              <Switch
                checked={previewMode}
                onChange={(e) => setPreviewMode(e.target.checked)}
                icon={<CodeIcon />}
                checkedIcon={<PreviewIcon />}
              />
            }
            label="JSON Schema"
          />
          {!readonly && (
            <Button
              variant="contained"
              startIcon={<AddIcon />}
              onClick={handleAddProperty}
            >
              Add Property
            </Button>
          )}
        </Box>
      </Box>

      {previewMode ? (
        <Card>
          <CardContent>
            <Typography variant="h6" gutterBottom>
              Generated JSON Schema
            </Typography>
            <Box
              component="pre"
              sx={{
                backgroundColor: '#f5f5f5',
                p: 2,
                borderRadius: 1,
                overflow: 'auto',
                fontSize: '0.875rem',
                fontFamily: 'monospace',
              }}
            >
              {JSON.stringify(generateJsonSchema(), null, 2)}
            </Box>
          </CardContent>
        </Card>
      ) : (
        <Box>
          {properties.length === 0 ? (
            <Alert severity="info">
              No properties defined. Click "Add Property" to start building your schema.
            </Alert>
          ) : (
            properties.map((property, index) => renderPropertyEditor(property, index))
          )}
        </Box>
      )}
    </Box>
  );
};

export default PropertySchemaBuilder;