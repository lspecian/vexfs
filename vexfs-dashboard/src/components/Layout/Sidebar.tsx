import React from 'react';
import {
  Drawer,
  List,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  Toolbar,
  Box,
  Typography,
  Divider,
  useTheme,
  useMediaQuery,
} from '@mui/material';
import {
  Dashboard as DashboardIcon,
  Storage as CollectionsIcon,
  Search as SearchIcon,
  Monitor as MonitoringIcon,
  Settings as SettingsIcon,
} from '@mui/icons-material';
import { useLocation, useNavigate } from 'react-router-dom';
import type { NavigationItem } from '../../types';

interface SidebarProps {
  drawerWidth: number;
  mobileOpen: boolean;
  onMobileClose: () => void;
}

const navigationItems: NavigationItem[] = [
  {
    id: 'dashboard',
    label: 'Dashboard',
    path: '/',
    icon: 'dashboard',
  },
  {
    id: 'collections',
    label: 'Collections',
    path: '/collections',
    icon: 'collections',
  },
  {
    id: 'search',
    label: 'Vector Search',
    path: '/search',
    icon: 'search',
  },
  {
    id: 'monitoring',
    label: 'Monitoring',
    path: '/monitoring',
    icon: 'monitoring',
  },
  {
    id: 'settings',
    label: 'Settings',
    path: '/settings',
    icon: 'settings',
  },
];

const getIcon = (iconName: string) => {
  switch (iconName) {
    case 'dashboard':
      return <DashboardIcon />;
    case 'collections':
      return <CollectionsIcon />;
    case 'search':
      return <SearchIcon />;
    case 'monitoring':
      return <MonitoringIcon />;
    case 'settings':
      return <SettingsIcon />;
    default:
      return <DashboardIcon />;
  }
};

const Sidebar: React.FC<SidebarProps> = ({
  drawerWidth,
  mobileOpen,
  onMobileClose,
}) => {
  const theme = useTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down('md'));
  const location = useLocation();
  const navigate = useNavigate();

  const handleNavigation = (path: string) => {
    navigate(path);
    if (isMobile) {
      onMobileClose();
    }
  };

  const drawerContent = (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Logo/Brand area */}
      <Toolbar
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          px: 3,
          minHeight: '64px !important',
        }}
      >
        <Typography
          variant="h5"
          component="div"
          sx={{
            fontWeight: 700,
            color: theme.palette.primary.main,
            textAlign: 'center',
          }}
        >
          VexFS
        </Typography>
      </Toolbar>

      <Divider />

      {/* Navigation Menu */}
      <Box sx={{ flexGrow: 1, py: 1 }}>
        <List>
          {navigationItems.map(item => {
            const isActive = location.pathname === item.path;
            return (
              <ListItem key={item.id} disablePadding sx={{ px: 2 }}>
                <ListItemButton
                  onClick={() => handleNavigation(item.path)}
                  sx={{
                    borderRadius: 2,
                    mb: 0.5,
                    backgroundColor: isActive
                      ? theme.palette.primary.main
                      : 'transparent',
                    color: isActive
                      ? theme.palette.primary.contrastText
                      : theme.palette.text.primary,
                    '&:hover': {
                      backgroundColor: isActive
                        ? theme.palette.primary.dark
                        : theme.palette.action.hover,
                    },
                    '& .MuiListItemIcon-root': {
                      color: isActive
                        ? theme.palette.primary.contrastText
                        : theme.palette.text.secondary,
                    },
                  }}
                >
                  <ListItemIcon sx={{ minWidth: 40 }}>
                    {getIcon(item.icon)}
                  </ListItemIcon>
                  <ListItemText
                    primary={item.label}
                    primaryTypographyProps={{
                      fontSize: '0.875rem',
                      fontWeight: isActive ? 600 : 400,
                    }}
                  />
                </ListItemButton>
              </ListItem>
            );
          })}
        </List>
      </Box>

      {/* Footer area */}
      <Box sx={{ p: 2, mt: 'auto' }}>
        <Divider sx={{ mb: 2 }} />
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{ textAlign: 'center', display: 'block' }}
        >
          VexFS Dashboard v1.0.0
        </Typography>
      </Box>
    </Box>
  );

  return (
    <Box
      component="nav"
      sx={{ width: { md: drawerWidth }, flexShrink: { md: 0 } }}
    >
      {/* Mobile drawer */}
      <Drawer
        variant="temporary"
        open={mobileOpen}
        onClose={onMobileClose}
        ModalProps={{
          keepMounted: true, // Better open performance on mobile
        }}
        sx={{
          display: { xs: 'block', md: 'none' },
          '& .MuiDrawer-paper': {
            boxSizing: 'border-box',
            width: drawerWidth,
            backgroundColor: theme.palette.background.paper,
            borderRight: `1px solid ${theme.palette.divider}`,
          },
        }}
      >
        {drawerContent}
      </Drawer>

      {/* Desktop drawer */}
      <Drawer
        variant="permanent"
        sx={{
          display: { xs: 'none', md: 'block' },
          '& .MuiDrawer-paper': {
            boxSizing: 'border-box',
            width: drawerWidth,
            backgroundColor: theme.palette.background.paper,
            borderRight: `1px solid ${theme.palette.divider}`,
          },
        }}
        open
      >
        {drawerContent}
      </Drawer>
    </Box>
  );
};

export default Sidebar;
