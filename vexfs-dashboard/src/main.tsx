import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { ErrorBoundary } from './components/Common/ErrorBoundary';
import './index.css';
import App from './App.tsx';

// Global error fallback for the entire application
const GlobalErrorFallback = () => (
  <div
    style={{
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      minHeight: '100vh',
      padding: '2rem',
      textAlign: 'center',
      backgroundColor: '#f5f5f5',
    }}
  >
    <h1 style={{ color: '#d32f2f', marginBottom: '1rem' }}>
      Application Error
    </h1>
    <p style={{ color: '#666', marginBottom: '2rem', maxWidth: '500px' }}>
      The application encountered an unexpected error and cannot continue. 
      Please refresh the page or contact support if the problem persists.
    </p>
    <button
      onClick={() => window.location.reload()}
      style={{
        padding: '0.75rem 1.5rem',
        backgroundColor: '#1976d2',
        color: 'white',
        border: 'none',
        borderRadius: '4px',
        cursor: 'pointer',
        fontSize: '1rem',
      }}
    >
      Reload Application
    </button>
  </div>
);

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ErrorBoundary
      fallback={<GlobalErrorFallback />}
      onError={(error, errorInfo) => {
        // Log critical errors to console in development
        if (import.meta.env.DEV) {
          console.error('🚨 Critical Application Error:', error, errorInfo);
        }
        // In production, you might want to send to error reporting service
        // errorReportingService.captureException(error, { extra: errorInfo });
      }}
    >
      <App />
    </ErrorBoundary>
  </StrictMode>
);
