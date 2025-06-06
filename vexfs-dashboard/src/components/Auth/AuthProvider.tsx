/**
 * Authentication Provider
 * Provides authentication context to the entire application
 */

import React, { createContext, useContext, type ReactNode } from 'react';
import { useAuth, type UseAuthReturn } from '../../hooks/useAuth';

interface AuthProviderProps {
  children: ReactNode;
}

const AuthContext = createContext<UseAuthReturn | undefined>(undefined);

export const AuthProvider: React.FC<AuthProviderProps> = ({ children }) => {
  const auth = useAuth();

  return <AuthContext.Provider value={auth}>{children}</AuthContext.Provider>;
};

export const useAuthContext = (): UseAuthReturn => {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuthContext must be used within an AuthProvider');
  }
  return context;
};

export default AuthProvider;
