import React from 'react';
import {
  AppBar,
  Toolbar,
  Typography,
  IconButton,
  Box,
  Chip,
  Breadcrumbs,
  Link,
  useTheme,
  useMediaQuery,
} from '@mui/material';
import {
  Menu as MenuIcon,
  Settings as SettingsIcon,
  Help as HelpIcon,
  Brightness4 as DarkModeIcon,
  Brightness7 as LightModeIcon,
} from '@mui/icons-material';
import { useLocation } from 'react-router-dom';
import { useServerHealth } from '../../hooks/useVexFS';

interface HeaderProps {
  onMenuToggle: () => void;
  drawerWidth: number;
}

const Header: React.FC<HeaderProps> = ({ onMenuToggle, drawerWidth }) => {
  const theme = useTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down('md'));
  const location = useLocation();
  const { isHealthy } = useServerHealth();

  // Generate breadcrumbs from current path
  const generateBreadcrumbs = () => {
    const pathnames = location.pathname.split('/').filter(x => x);
    const breadcrumbs = [
      { label: 'Dashboard', path: '/' },
      ...pathnames.map((name, index) => {
        const path = `/${pathnames.slice(0, index + 1).join('/')}`;
        const label = name.charAt(0).toUpperCase() + name.slice(1);
        return { label, path };
      }),
    ];
    return breadcrumbs;
  };

  const breadcrumbs = generateBreadcrumbs();

  return (
    <AppBar
      position="fixed"
      sx={{
        width: { md: `calc(100% - ${drawerWidth}px)` },
        ml: { md: `${drawerWidth}px` },
        zIndex: theme.zIndex.drawer + 1,
        backgroundColor: theme.palette.background.paper,
        color: theme.palette.text.primary,
        borderBottom: `1px solid ${theme.palette.divider}`,
      }}
      elevation={0}
    >
      <Toolbar sx={{ minHeight: '64px !important' }}>
        {/* Mobile menu button */}
        {isMobile && (
          <IconButton
            color="inherit"
            aria-label="open drawer"
            edge="start"
            onClick={onMenuToggle}
            sx={{ mr: 2 }}
          >
            <MenuIcon />
          </IconButton>
        )}

        {/* VexFS Logo and Title */}
        <Box sx={{ display: 'flex', alignItems: 'center', flexGrow: 1 }}>
          <Typography
            variant="h6"
            noWrap
            component="div"
            sx={{
              fontWeight: 600,
              color: theme.palette.primary.main,
              mr: 2,
            }}
          >
            VexFS
          </Typography>

          {/* Server Status Indicator */}
          <Chip
            label={isHealthy ? 'Online' : 'Offline'}
            color={isHealthy ? 'success' : 'error'}
            size="small"
            sx={{ mr: 2 }}
          />

          {/* Breadcrumbs - Hidden on mobile */}
          {!isMobile && (
            <Breadcrumbs
              aria-label="breadcrumb"
              sx={{ color: theme.palette.text.secondary }}
            >
              {breadcrumbs.map((crumb, index) => {
                const isLast = index === breadcrumbs.length - 1;
                return isLast ? (
                  <Typography
                    key={crumb.path}
                    color="text.primary"
                    variant="body2"
                  >
                    {crumb.label}
                  </Typography>
                ) : (
                  <Link
                    key={crumb.path}
                    underline="hover"
                    color="inherit"
                    href={crumb.path}
                    variant="body2"
                  >
                    {crumb.label}
                  </Link>
                );
              })}
            </Breadcrumbs>
          )}
        </Box>

        {/* Action buttons */}
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          {/* Theme toggle - will be implemented later */}
          <IconButton
            color="inherit"
            aria-label="toggle theme"
            title="Toggle theme"
          >
            {theme.palette.mode === 'dark' ? (
              <LightModeIcon />
            ) : (
              <DarkModeIcon />
            )}
          </IconButton>

          {/* Help button */}
          <IconButton color="inherit" aria-label="help" title="Help">
            <HelpIcon />
          </IconButton>

          {/* Settings button */}
          <IconButton color="inherit" aria-label="settings" title="Settings">
            <SettingsIcon />
          </IconButton>
        </Box>
      </Toolbar>
    </AppBar>
  );
};

export default Header;
