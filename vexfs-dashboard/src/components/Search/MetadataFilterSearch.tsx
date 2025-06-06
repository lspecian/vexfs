import React, { useState, useCallback } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  TextField,
  Button,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  IconButton,
  Chip,
  Alert,
  Autocomplete,
  Switch,
  FormControlLabel,
  Divider,
  Paper,
} from '@mui/material';
import {
  Add as AddIcon,
  Delete as DeleteIcon,
  Search as SearchIcon,
  Clear as ClearIcon,
  FilterList as FilterIcon,
  Code as CodeIcon,
} from '@mui/icons-material';
import { useForm, Controller, useFieldArray } from 'react-hook-form';
import type {
  MetadataFilterQuery,
  FilterCondition,
  FilterOperator,
  CollectionSchema,
  VexFSCollection,
} from '../../types';

interface MetadataFilterSearchProps {
  collections: VexFSCollection[];
  selectedCollection: string | null;
  onCollectionChange: (collectionId: string) => void;
  onSearch: (query: MetadataFilterQuery) => void;
  schema?: CollectionSchema;
  loading?: boolean;
}

const FILTER_OPERATORS: { value: FilterOperator; label: string }[] = [
  { value: 'equals', label: 'Equals' },
  { value: 'not_equals', label: 'Not Equals' },
  { value: 'contains', label: 'Contains' },
  { value: 'not_contains', label: 'Does Not Contain' },
  { value: 'starts_with', label: 'Starts With' },
  { value: 'ends_with', label: 'Ends With' },
  { value: 'greater_than', label: 'Greater Than' },
  { value: 'less_than', label: 'Less Than' },
  { value: 'greater_equal', label: 'Greater or Equal' },
  { value: 'less_equal', label: 'Less or Equal' },
  { value: 'in', label: 'In Array' },
  { value: 'not_in', label: 'Not In Array' },
  { value: 'exists', label: 'Field Exists' },
  { value: 'not_exists', label: 'Field Does Not Exist' },
  { value: 'regex', label: 'Regex Match' },
];

const LOGICAL_OPERATORS = [
  { value: 'AND', label: 'AND' },
  { value: 'OR', label: 'OR' },
  { value: 'NOT', label: 'NOT' },
];

const MetadataFilterSearch: React.FC<MetadataFilterSearchProps> = ({
  collections,
  selectedCollection,
  onCollectionChange,
  onSearch,
  schema,
  loading = false,
}) => {
  const [showJsonView, setShowJsonView] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const { control, handleSubmit, watch, setValue, reset } =
    useForm<MetadataFilterQuery>({
      defaultValues: {
        conditions: [
          {
            id: '1',
            field: '',
            operator: 'equals',
            value: '',
            logicalOperator: 'AND',
          },
        ],
        logicalOperator: 'AND',
      },
    });

  const { fields, append, remove } = useFieldArray({
    control,
    name: 'conditions',
  });

  const watchedConditions = watch('conditions');
  const watchedLogicalOperator = watch('logicalOperator');

  const getFieldType = (fieldName: string): string => {
    const field = schema?.fields.find(f => f.name === fieldName);
    return field?.type || 'string';
  };

  const getOperatorsForField = (fieldName: string): FilterOperator[] => {
    const fieldType = getFieldType(fieldName);

    switch (fieldType) {
      case 'number':
        return [
          'equals',
          'not_equals',
          'greater_than',
          'less_than',
          'greater_equal',
          'less_equal',
          'in',
          'not_in',
          'exists',
          'not_exists',
        ];
      case 'boolean':
        return ['equals', 'not_equals', 'exists', 'not_exists'];
      case 'array':
        return [
          'contains',
          'not_contains',
          'in',
          'not_in',
          'exists',
          'not_exists',
        ];
      case 'date':
        return [
          'equals',
          'not_equals',
          'greater_than',
          'less_than',
          'greater_equal',
          'less_equal',
          'exists',
          'not_exists',
        ];
      default: // string
        return [
          'equals',
          'not_equals',
          'contains',
          'not_contains',
          'starts_with',
          'ends_with',
          'in',
          'not_in',
          'regex',
          'exists',
          'not_exists',
        ];
    }
  };

  const renderValueInput = (
    condition: FilterCondition,
    index: number
  ): React.ReactNode => {
    const fieldType = getFieldType(condition.field);
    const needsValue = !['exists', 'not_exists'].includes(condition.operator);

    if (!needsValue) {
      return null;
    }

    const commonProps = {
      size: 'small' as const,
      fullWidth: true,
      label: 'Value',
    };

    switch (fieldType) {
      case 'number':
        return (
          <Controller
            name={`conditions.${index}.value`}
            control={control}
            render={({ field }) => (
              <TextField
                {...field}
                {...commonProps}
                type="number"
                value={field.value || ''}
              />
            )}
          />
        );

      case 'boolean':
        return (
          <Controller
            name={`conditions.${index}.value`}
            control={control}
            render={({ field }) => (
              <FormControl {...commonProps}>
                <InputLabel>Value</InputLabel>
                <Select {...field} label="Value" value={field.value || ''}>
                  <MenuItem value="true">True</MenuItem>
                  <MenuItem value="false">False</MenuItem>
                </Select>
              </FormControl>
            )}
          />
        );

      case 'date':
        return (
          <Controller
            name={`conditions.${index}.value`}
            control={control}
            render={({ field }) => (
              <TextField
                {...field}
                {...commonProps}
                type="datetime-local"
                value={field.value || ''}
              />
            )}
          />
        );

      case 'array':
        if (['in', 'not_in'].includes(condition.operator)) {
          return (
            <Controller
              name={`conditions.${index}.value`}
              control={control}
              render={({ field }) => (
                <Autocomplete
                  {...field}
                  multiple
                  freeSolo
                  options={[]}
                  renderInput={params => (
                    <TextField {...params} {...commonProps} />
                  )}
                  onChange={(_, value) => field.onChange(value)}
                  value={Array.isArray(field.value) ? field.value : []}
                />
              )}
            />
          );
        }
      // Fall through to default string input

      default:
        if (['in', 'not_in'].includes(condition.operator)) {
          return (
            <Controller
              name={`conditions.${index}.value`}
              control={control}
              render={({ field }) => (
                <TextField
                  {...field}
                  {...commonProps}
                  placeholder="value1,value2,value3"
                  helperText="Comma-separated values"
                  value={field.value || ''}
                />
              )}
            />
          );
        }

        return (
          <Controller
            name={`conditions.${index}.value`}
            control={control}
            render={({ field }) => (
              <TextField
                {...field}
                {...commonProps}
                multiline={condition.operator === 'regex'}
                rows={condition.operator === 'regex' ? 2 : 1}
                value={field.value || ''}
                placeholder={
                  condition.operator === 'regex'
                    ? '^pattern.*$'
                    : 'Enter value...'
                }
              />
            )}
          />
        );
    }
  };

  const addCondition = () => {
    append({
      id: Date.now().toString(),
      field: '',
      operator: 'equals',
      value: '',
      logicalOperator: 'AND',
    });
  };

  const generateJsonQuery = (): object => {
    const conditions = watchedConditions.filter(c => c.field && c.operator);

    if (conditions.length === 0) return {};

    if (conditions.length === 1) {
      const condition = conditions[0];
      return {
        [condition.field]: {
          [condition.operator]: condition.value,
        },
      };
    }

    // Multiple conditions
    const operator = watchedLogicalOperator?.toLowerCase() || 'and';
    return {
      [`$${operator}`]: conditions.map(condition => ({
        [condition.field]: {
          [condition.operator]: condition.value,
        },
      })),
    };
  };

  const onSubmit = useCallback(
    (data: MetadataFilterQuery) => {
      if (!selectedCollection) {
        setError('Please select a collection');
        return;
      }

      const validConditions = data.conditions.filter(
        c => c.field && c.operator
      );

      if (validConditions.length === 0) {
        setError('Please add at least one filter condition');
        return;
      }

      setError(null);
      onSearch({
        ...data,
        conditions: validConditions,
      });
    },
    [selectedCollection, onSearch]
  );

  const clearForm = () => {
    reset();
    setError(null);
  };

  return (
    <Card>
      <CardContent>
        <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
          <FilterIcon sx={{ mr: 1 }} />
          <Typography variant="h6" sx={{ flexGrow: 1, fontWeight: 600 }}>
            Metadata Filter Search
          </Typography>
          <FormControlLabel
            control={
              <Switch
                checked={showJsonView}
                onChange={e => setShowJsonView(e.target.checked)}
                size="small"
              />
            }
            label="JSON View"
          />
        </Box>

        {/* Collection Selection */}
        <FormControl fullWidth sx={{ mb: 3 }}>
          <InputLabel>Collection</InputLabel>
          <Select
            value={selectedCollection || ''}
            onChange={e => onCollectionChange(e.target.value)}
            label="Collection"
          >
            {collections.map(collection => (
              <MenuItem key={collection.id} value={collection.id}>
                {collection.name} ({collection.pointsCount} vectors)
              </MenuItem>
            ))}
          </Select>
        </FormControl>

        {error && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {error}
          </Alert>
        )}

        <form onSubmit={handleSubmit(onSubmit)}>
          {showJsonView ? (
            /* JSON View */
            <Box sx={{ mb: 3 }}>
              <Typography variant="subtitle2" sx={{ mb: 1, fontWeight: 600 }}>
                Generated Query (Read-only)
              </Typography>
              <Paper
                sx={{
                  p: 2,
                  bgcolor: 'grey.50',
                  fontFamily: 'monospace',
                  fontSize: '0.875rem',
                  maxHeight: 300,
                  overflow: 'auto',
                }}
              >
                <pre>{JSON.stringify(generateJsonQuery(), null, 2)}</pre>
              </Paper>
            </Box>
          ) : (
            /* Visual Query Builder */
            <Box sx={{ mb: 3 }}>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
                <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
                  Filter Conditions
                </Typography>
                <Box sx={{ flexGrow: 1 }} />
                <Controller
                  name="logicalOperator"
                  control={control}
                  render={({ field }) => (
                    <FormControl size="small" sx={{ minWidth: 100 }}>
                      <InputLabel>Logic</InputLabel>
                      <Select {...field} label="Logic">
                        {LOGICAL_OPERATORS.map(op => (
                          <MenuItem key={op.value} value={op.value}>
                            {op.label}
                          </MenuItem>
                        ))}
                      </Select>
                    </FormControl>
                  )}
                />
              </Box>

              {fields.map((field, index) => (
                <Paper
                  key={field.id}
                  sx={{
                    p: 2,
                    mb: 2,
                    border: '1px solid',
                    borderColor: 'divider',
                  }}
                >
                  <Box
                    sx={{
                      display: 'grid',
                      gridTemplateColumns: '2fr 1.5fr 2fr auto',
                      gap: 2,
                      alignItems: 'start',
                    }}
                  >
                    {/* Field Selection */}
                    <Controller
                      name={`conditions.${index}.field`}
                      control={control}
                      render={({ field: fieldProps }) => (
                        <Autocomplete
                          {...fieldProps}
                          options={schema?.fields.map(f => f.name) || []}
                          freeSolo
                          renderInput={params => (
                            <TextField
                              {...params}
                              label="Field"
                              size="small"
                              placeholder="metadata.field"
                            />
                          )}
                          onChange={(_, value) => fieldProps.onChange(value)}
                          value={fieldProps.value || ''}
                        />
                      )}
                    />

                    {/* Operator Selection */}
                    <Controller
                      name={`conditions.${index}.operator`}
                      control={control}
                      render={({ field: fieldProps }) => (
                        <FormControl size="small" fullWidth>
                          <InputLabel>Operator</InputLabel>
                          <Select {...fieldProps} label="Operator">
                            {getOperatorsForField(
                              watchedConditions[index]?.field || ''
                            ).map(op => {
                              const operatorInfo = FILTER_OPERATORS.find(
                                o => o.value === op
                              );
                              return (
                                <MenuItem key={op} value={op}>
                                  {operatorInfo?.label || op}
                                </MenuItem>
                              );
                            })}
                          </Select>
                        </FormControl>
                      )}
                    />

                    {/* Value Input */}
                    <Box>
                      {renderValueInput(watchedConditions[index], index)}
                    </Box>

                    {/* Remove Button */}
                    <IconButton
                      onClick={() => remove(index)}
                      disabled={fields.length === 1}
                      size="small"
                      color="error"
                    >
                      <DeleteIcon />
                    </IconButton>
                  </Box>

                  {index < fields.length - 1 && (
                    <Box sx={{ textAlign: 'center', mt: 1 }}>
                      <Chip
                        label={watchedLogicalOperator || 'AND'}
                        size="small"
                        variant="outlined"
                      />
                    </Box>
                  )}
                </Paper>
              ))}

              <Button
                variant="outlined"
                startIcon={<AddIcon />}
                onClick={addCondition}
                sx={{ mb: 2 }}
              >
                Add Condition
              </Button>
            </Box>
          )}

          <Divider sx={{ my: 3 }} />

          {/* Action Buttons */}
          <Box sx={{ display: 'flex', gap: 2 }}>
            <Button
              type="submit"
              variant="contained"
              startIcon={<SearchIcon />}
              disabled={loading || !selectedCollection}
              sx={{ borderRadius: 2 }}
            >
              {loading ? 'Searching...' : 'Search Metadata'}
            </Button>
            <Button
              variant="outlined"
              startIcon={<ClearIcon />}
              onClick={clearForm}
              disabled={loading}
            >
              Clear
            </Button>
            <Button
              variant="outlined"
              startIcon={<CodeIcon />}
              onClick={() => setShowJsonView(!showJsonView)}
            >
              {showJsonView ? 'Visual' : 'JSON'} View
            </Button>
          </Box>
        </form>
      </CardContent>
    </Card>
  );
};

export default MetadataFilterSearch;
