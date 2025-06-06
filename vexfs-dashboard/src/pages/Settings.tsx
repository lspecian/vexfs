import React from 'react';
import {
  Box,
  Typography,
  Card,
  CardContent,
  Switch,
  FormControlLabel,
  TextField,
  Button,
  Divider,
  Grid,
} from '@mui/material';
import { Save as SaveIcon } from '@mui/icons-material';

const Settings: React.FC = () => {
  return (
    <Box>
      <Typography variant="h4" component="h1" sx={{ mb: 3, fontWeight: 600 }}>
        Settings
      </Typography>

      <Grid container spacing={3}>
        {/* General Settings */}
        <Grid item xs={12} md={6}>
          <Card>
            <CardContent>
              <Typography variant="h6" component="h2" sx={{ mb: 3 }}>
                General Settings
              </Typography>

              <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
                <FormControlLabel
                  control={<Switch defaultChecked />}
                  label="Enable dark mode"
                />
                <FormControlLabel
                  control={<Switch defaultChecked />}
                  label="Show notifications"
                />
                <FormControlLabel
                  control={<Switch />}
                  label="Auto-refresh dashboard"
                />
                <TextField
                  label="Refresh interval (seconds)"
                  type="number"
                  defaultValue={30}
                  size="small"
                />
              </Box>
            </CardContent>
          </Card>
        </Grid>

        {/* VexFS Configuration */}
        <Grid item xs={12} md={6}>
          <Card>
            <CardContent>
              <Typography variant="h6" component="h2" sx={{ mb: 3 }}>
                VexFS Configuration
              </Typography>

              <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
                <TextField
                  label="Server URL"
                  defaultValue="http://localhost:8080"
                  size="small"
                  fullWidth
                />
                <TextField
                  label="API Timeout (ms)"
                  type="number"
                  defaultValue={10000}
                  size="small"
                />
                <TextField
                  label="Default vector size"
                  type="number"
                  defaultValue={384}
                  size="small"
                />
                <FormControlLabel
                  control={<Switch defaultChecked />}
                  label="Enable API logging"
                />
              </Box>
            </CardContent>
          </Card>
        </Grid>

        {/* Performance Settings */}
        <Grid item xs={12}>
          <Card>
            <CardContent>
              <Typography variant="h6" component="h2" sx={{ mb: 3 }}>
                Performance Settings
              </Typography>

              <Grid container spacing={3}>
                <Grid item xs={12} sm={6} md={3}>
                  <TextField
                    label="Search limit"
                    type="number"
                    defaultValue={100}
                    size="small"
                    fullWidth
                  />
                </Grid>
                <Grid item xs={12} sm={6} md={3}>
                  <TextField
                    label="Batch size"
                    type="number"
                    defaultValue={1000}
                    size="small"
                    fullWidth
                  />
                </Grid>
                <Grid item xs={12} sm={6} md={3}>
                  <TextField
                    label="Cache size (MB)"
                    type="number"
                    defaultValue={512}
                    size="small"
                    fullWidth
                  />
                </Grid>
                <Grid item xs={12} sm={6} md={3}>
                  <TextField
                    label="Max connections"
                    type="number"
                    defaultValue={10}
                    size="small"
                    fullWidth
                  />
                </Grid>
              </Grid>

              <Box sx={{ mt: 3 }}>
                <FormControlLabel
                  control={<Switch defaultChecked />}
                  label="Enable query optimization"
                />
                <FormControlLabel
                  control={<Switch />}
                  label="Enable compression"
                  sx={{ ml: 3 }}
                />
              </Box>
            </CardContent>
          </Card>
        </Grid>

        {/* About */}
        <Grid item xs={12}>
          <Card>
            <CardContent>
              <Typography variant="h6" component="h2" sx={{ mb: 2 }}>
                About VexFS Dashboard
              </Typography>
              <Typography variant="body1" color="text.secondary" sx={{ mb: 2 }}>
                VexFS Dashboard v1.0.0
              </Typography>
              <Typography variant="body2" color="text.secondary">
                A modern web interface for managing VexFS vector filesystem
                operations. Built with React, TypeScript, and Material-UI.
              </Typography>

              <Divider sx={{ my: 3 }} />

              <Box sx={{ display: 'flex', gap: 2 }}>
                <Button
                  variant="contained"
                  startIcon={<SaveIcon />}
                  sx={{ borderRadius: 2 }}
                >
                  Save Settings
                </Button>
                <Button variant="outlined" sx={{ borderRadius: 2 }}>
                  Reset to Defaults
                </Button>
              </Box>
            </CardContent>
          </Card>
        </Grid>
      </Grid>
    </Box>
  );
};

export default Settings;
