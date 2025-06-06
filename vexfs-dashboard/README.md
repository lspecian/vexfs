# VexFS Web UI Dashboard

A comprehensive web-based dashboard for VexFS that provides a Qdrant-style interface for browsing collections, viewing vectors, and managing the database.

## Features

- 🚀 **Modern React + TypeScript** - Built with Vite for fast development
- 🎨 **Material-UI Design** - Professional and responsive interface
- 📊 **Data Visualization** - Charts and graphs using Recharts
- 🔍 **Vector Search** - Advanced vector similarity search capabilities
- 📱 **Responsive Design** - Works on desktop and mobile devices
- 🛠️ **Developer Tools** - ESLint, Prettier, and TypeScript for code quality

## Tech Stack

- **Frontend Framework**: React 19 with TypeScript
- **Build Tool**: Vite
- **UI Library**: Material-UI (MUI) v7
- **Routing**: React Router DOM v7
- **HTTP Client**: Axios
- **Charts**: Recharts
- **Code Quality**: ESLint + Prettier
- **Type Checking**: TypeScript with strict mode

## Project Structure

```
src/
├── components/     # Reusable UI components
├── hooks/         # Custom React hooks
├── pages/         # Page components
├── services/      # API services and external integrations
├── types/         # TypeScript type definitions
├── utils/         # Utility functions
└── assets/        # Static assets
```

## Getting Started

### Prerequisites

- Node.js 18+
- npm or yarn
- VexFS server running (default: http://localhost:8080)

### Installation

1. **Install dependencies**:

   ```bash
   npm install
   ```

2. **Start development server**:

   ```bash
   npm run dev
   ```

3. **Open your browser** and navigate to `http://localhost:5173`

### Available Scripts

- `npm run dev` - Start development server
- `npm run build` - Build for production
- `npm run preview` - Preview production build
- `npm run lint` - Run ESLint
- `npm run lint:fix` - Fix ESLint issues automatically
- `npm run format` - Format code with Prettier
- `npm run format:check` - Check code formatting
- `npm run type-check` - Run TypeScript type checking

## API Integration

The dashboard connects to the VexFS server via REST API. The default server URL is `http://localhost:8080`, but this can be configured in the API service.

### Supported Operations

- **Collections**: Create, list, view, and delete vector collections
- **Points**: Add, update, delete, and search vector points
- **Search**: Perform similarity searches with various distance metrics
- **Dashboard**: View system statistics and health status

## Development

### Code Quality

This project uses strict TypeScript configuration and comprehensive linting rules:

- **TypeScript**: Strict mode enabled with comprehensive type checking
- **ESLint**: Extended rules for React, TypeScript, and code quality
- **Prettier**: Consistent code formatting
- **Import Organization**: Automatic import sorting and organization

### Custom Hooks

The project includes several custom hooks for VexFS operations:

- `useCollections()` - Manage vector collections
- `usePoints(collectionName)` - Manage points in a collection
- `useVectorSearch()` - Perform vector similarity searches
- `useDashboardStats()` - Fetch dashboard statistics
- `useServerHealth()` - Monitor server health

### API Service

The `vexfsApi` service provides a clean interface to the VexFS server:

```typescript
import { vexfsApi } from './services/api';

// Get all collections
const collections = await vexfsApi.getCollections();

// Search vectors
const results = await vexfsApi.searchPoints('my-collection', {
  vector: [0.1, 0.2, 0.3],
  limit: 10,
});
```

## Configuration

### Environment Variables

Create a `.env` file in the project root to configure the dashboard:

```env
VITE_VEXFS_API_URL=http://localhost:8080
VITE_APP_TITLE=VexFS Dashboard
```

### TypeScript Configuration

The project uses strict TypeScript settings for maximum type safety:

- Strict null checks
- No implicit any
- Unused variable detection
- Comprehensive type checking

## Building for Production

1. **Build the project**:

   ```bash
   npm run build
   ```

2. **Preview the build**:

   ```bash
   npm run preview
   ```

3. **Deploy**: The `dist/` folder contains the production build ready for deployment.

## Contributing

1. Follow the existing code style and conventions
2. Run `npm run lint` and `npm run type-check` before committing
3. Use meaningful commit messages
4. Add tests for new features

## License

This project is part of the VexFS ecosystem. See the main VexFS repository for licensing information.
