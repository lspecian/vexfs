import React, { useState } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  TextField,
  Box,
  Typography,
  Tabs,
  Tab,
  Alert,
  LinearProgress,
  Chip,
  FormControlLabel,
  Switch,
} from '@mui/material';
import {
  Upload as UploadIcon,
  Add as AddIcon,
  Close as CloseIcon,
} from '@mui/icons-material';
import { useForm, Controller } from 'react-hook-form';
import { generateRandomVector } from '../../utils';
import type { VexFSPoint } from '../../types';

interface AddVectorFormProps {
  open: boolean;
  collectionName: string;
  vectorSize: number;
  onClose: () => void;
  onSubmit: (vectors: VexFSPoint[]) => Promise<boolean>;
}

interface VectorFormData {
  vectorId: string;
  vectorData: string;
  metadata: string;
  generateRandom: boolean;
}

interface TabPanelProps {
  children?: React.ReactNode;
  index: number;
  value: number;
}

const TabPanel: React.FC<TabPanelProps> = ({ children, value, index }) => {
  return (
    <div hidden={value !== index}>
      {value === index && <Box sx={{ py: 2 }}>{children}</Box>}
    </div>
  );
};

const AddVectorForm: React.FC<AddVectorFormProps> = ({
  open,
  collectionName,
  vectorSize,
  onClose,
  onSubmit,
}) => {
  const [activeTab, setActiveTab] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [uploadedFile, setUploadedFile] = useState<File | null>(null);

  const {
    control,
    handleSubmit,
    reset,
    watch,
    setValue,
    formState: { errors },
  } = useForm<VectorFormData>({
    defaultValues: {
      vectorId: '',
      vectorData: '',
      metadata: '{}',
      generateRandom: false,
    },
  });

  const generateRandom = watch('generateRandom');

  const handleTabChange = (_event: React.SyntheticEvent, newValue: number) => {
    setActiveTab(newValue);
  };

  const handleClose = () => {
    reset();
    setError(null);
    setUploadedFile(null);
    setActiveTab(0);
    onClose();
  };

  const handleGenerateRandom = () => {
    const randomVector = generateRandomVector(vectorSize);
    setValue('vectorData', JSON.stringify(randomVector));
  };

  const validateVectorData = (data: string): number[] | null => {
    try {
      const parsed = JSON.parse(data);
      if (!Array.isArray(parsed)) {
        throw new Error('Vector data must be an array');
      }
      if (parsed.length !== vectorSize) {
        throw new Error(
          `Vector must have exactly ${vectorSize} dimensions, got ${parsed.length}`
        );
      }
      if (!parsed.every(val => typeof val === 'number')) {
        throw new Error('All vector values must be numbers');
      }
      return parsed;
    } catch {
      return null;
    }
  };

  const validateMetadata = (data: string): Record<string, unknown> | null => {
    try {
      const parsed = JSON.parse(data);
      if (
        typeof parsed !== 'object' ||
        parsed === null ||
        Array.isArray(parsed)
      ) {
        throw new Error('Metadata must be a JSON object');
      }
      return parsed;
    } catch {
      return null;
    }
  };

  const onFormSubmit = async (data: VectorFormData) => {
    setLoading(true);
    setError(null);

    try {
      let vectorData: number[];

      if (data.generateRandom) {
        vectorData = generateRandomVector(vectorSize);
      } else {
        const parsedVector = validateVectorData(data.vectorData);
        if (!parsedVector) {
          throw new Error('Invalid vector data format');
        }
        vectorData = parsedVector;
      }

      const metadata = validateMetadata(data.metadata);
      if (!metadata) {
        throw new Error('Invalid metadata format');
      }

      const vector: VexFSPoint = {
        id: data.vectorId || `vector_${Date.now()}`,
        vector: vectorData,
        payload: metadata,
      };

      const success = await onSubmit([vector]);
      if (success) {
        handleClose();
      } else {
        setError('Failed to add vector');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error occurred');
    } finally {
      setLoading(false);
    }
  };

  const handleFileUpload = async (
    event: React.ChangeEvent<HTMLInputElement>
  ) => {
    const file = event.target.files?.[0];
    if (!file) return;

    setUploadedFile(file);
    setLoading(true);
    setError(null);

    try {
      const text = await file.text();
      let vectors: VexFSPoint[] = [];

      if (file.name.endsWith('.json')) {
        const parsed = JSON.parse(text);
        if (Array.isArray(parsed)) {
          vectors = parsed;
        } else {
          vectors = [parsed];
        }
      } else if (file.name.endsWith('.csv')) {
        // Simple CSV parsing for vectors
        const lines = text.split('\n').filter(line => line.trim());
        vectors = lines.map((line, index) => {
          const values = line.split(',').map(val => parseFloat(val.trim()));
          if (values.length !== vectorSize) {
            throw new Error(
              `Line ${index + 1}: Expected ${vectorSize} values, got ${values.length}`
            );
          }
          return {
            id: `vector_${index + 1}`,
            vector: values,
            payload: {},
          };
        });
      } else {
        throw new Error('Unsupported file format. Use JSON or CSV.');
      }

      // Validate all vectors
      for (const vector of vectors) {
        if (!vector.vector || vector.vector.length !== vectorSize) {
          throw new Error(
            `Vector ${vector.id}: Expected ${vectorSize} dimensions`
          );
        }
      }

      const success = await onSubmit(vectors);
      if (success) {
        handleClose();
      } else {
        setError('Failed to upload vectors');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to process file');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onClose={handleClose} maxWidth="md" fullWidth>
      <DialogTitle>
        <Box
          sx={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
          }}
        >
          Add Vectors to {collectionName}
          <Button onClick={handleClose} size="small">
            <CloseIcon />
          </Button>
        </Box>
      </DialogTitle>

      <DialogContent>
        {loading && <LinearProgress sx={{ mb: 2 }} />}
        {error && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {error}
          </Alert>
        )}

        <Tabs value={activeTab} onChange={handleTabChange} sx={{ mb: 2 }}>
          <Tab label="Single Vector" />
          <Tab label="Batch Upload" />
        </Tabs>

        {/* Single Vector Tab */}
        <TabPanel value={activeTab} index={0}>
          <form onSubmit={handleSubmit(onFormSubmit)}>
            <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
              <Controller
                name="vectorId"
                control={control}
                render={({ field }) => (
                  <TextField
                    {...field}
                    label="Vector ID (optional)"
                    placeholder="Leave empty to auto-generate"
                    fullWidth
                    size="small"
                  />
                )}
              />

              <Controller
                name="generateRandom"
                control={control}
                render={({ field }) => (
                  <FormControlLabel
                    control={
                      <Switch
                        checked={field.value}
                        onChange={e => {
                          field.onChange(e.target.checked);
                          if (e.target.checked) {
                            handleGenerateRandom();
                          }
                        }}
                      />
                    }
                    label={`Generate random ${vectorSize}-dimensional vector`}
                  />
                )}
              />

              <Controller
                name="vectorData"
                control={control}
                rules={{
                  required: !generateRandom && 'Vector data is required',
                  validate: value => {
                    if (generateRandom) return true;
                    const parsed = validateVectorData(value);
                    return parsed !== null || 'Invalid vector format';
                  },
                }}
                render={({ field }) => (
                  <TextField
                    {...field}
                    label="Vector Data"
                    placeholder={`[${Array(Math.min(vectorSize, 5))
                      .fill('0.0')
                      .join(', ')}${vectorSize > 5 ? ', ...' : ''}]`}
                    multiline
                    rows={4}
                    fullWidth
                    disabled={generateRandom}
                    error={!!errors.vectorData}
                    helperText={
                      errors.vectorData?.message ||
                      `JSON array with exactly ${vectorSize} numbers`
                    }
                  />
                )}
              />

              <Controller
                name="metadata"
                control={control}
                rules={{
                  validate: value => {
                    const parsed = validateMetadata(value);
                    return parsed !== null || 'Invalid JSON format';
                  },
                }}
                render={({ field }) => (
                  <TextField
                    {...field}
                    label="Metadata (JSON)"
                    placeholder='{"key": "value", "category": "example"}'
                    multiline
                    rows={3}
                    fullWidth
                    error={!!errors.metadata}
                    helperText={
                      errors.metadata?.message || 'JSON object with metadata'
                    }
                  />
                )}
              />

              <Box sx={{ display: 'flex', gap: 1 }}>
                <Chip
                  label={`${vectorSize} dimensions required`}
                  size="small"
                  color="primary"
                  variant="outlined"
                />
                {generateRandom && (
                  <Chip
                    label="Random vector will be generated"
                    size="small"
                    color="success"
                    variant="outlined"
                  />
                )}
              </Box>
            </Box>
          </form>
        </TabPanel>

        {/* Batch Upload Tab */}
        <TabPanel value={activeTab} index={1}>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
            <Typography variant="body2" color="text.secondary">
              Upload multiple vectors from a JSON or CSV file. Each vector must
              have exactly {vectorSize} dimensions.
            </Typography>

            <Box
              sx={{
                border: '2px dashed',
                borderColor: 'grey.300',
                borderRadius: 2,
                p: 4,
                textAlign: 'center',
                cursor: 'pointer',
                '&:hover': {
                  borderColor: 'primary.main',
                  bgcolor: 'action.hover',
                },
              }}
              component="label"
            >
              <input
                type="file"
                accept=".json,.csv"
                onChange={handleFileUpload}
                style={{ display: 'none' }}
              />
              <UploadIcon sx={{ fontSize: 48, color: 'grey.400', mb: 2 }} />
              <Typography variant="h6" gutterBottom>
                Choose file to upload
              </Typography>
              <Typography variant="body2" color="text.secondary">
                Supports JSON and CSV formats
              </Typography>
              {uploadedFile && (
                <Chip
                  label={uploadedFile.name}
                  sx={{ mt: 2 }}
                  onDelete={() => setUploadedFile(null)}
                />
              )}
            </Box>

            <Alert severity="info">
              <Typography variant="body2">
                <strong>JSON format:</strong> Array of objects with id, vector,
                and payload fields
                <br />
                <strong>CSV format:</strong> Each row contains {vectorSize}{' '}
                comma-separated numbers
              </Typography>
            </Alert>
          </Box>
        </TabPanel>
      </DialogContent>

      <DialogActions>
        <Button onClick={handleClose} disabled={loading}>
          Cancel
        </Button>
        {activeTab === 0 && (
          <Button
            onClick={handleSubmit(onFormSubmit)}
            variant="contained"
            startIcon={<AddIcon />}
            disabled={loading}
          >
            Add Vector
          </Button>
        )}
      </DialogActions>
    </Dialog>
  );
};

export default AddVectorForm;
