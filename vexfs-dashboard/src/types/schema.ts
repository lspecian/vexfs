// Schema Management Types for VexGraph
import type { GraphSchema, NodeTypeSchema, EdgeTypeSchema } from './graph';

export interface PropertyConstraint {
  type: 'required' | 'optional' | 'unique' | 'indexed';
  minLength?: number;
  maxLength?: number;
  minValue?: number;
  maxValue?: number;
  pattern?: string;
  enum?: string[];
  defaultValue?: any;
}

export interface PropertyDefinition {
  name: string;
  type: 'String' | 'Number' | 'Boolean' | 'Date' | 'Array' | 'Object' | 'Reference';
  constraints: PropertyConstraint[];
  description?: string;
  examples?: any[];
}

export interface NodeTypeDefinition {
  id: string;
  name: string;
  displayName: string;
  description: string;
  icon?: string;
  color?: string;
  properties: PropertyDefinition[];
  inheritFrom?: string[];
  validationRules?: ValidationRule[];
  indexHints?: string[];
}

export interface EdgeTypeDefinition {
  id: string;
  name: string;
  displayName: string;
  description: string;
  directionality: 'directed' | 'undirected' | 'bidirectional';
  allowedSourceTypes: string[];
  allowedTargetTypes: string[];
  properties: PropertyDefinition[];
  cardinality: 'one-to-one' | 'one-to-many' | 'many-to-many';
  weightConstraints?: {
    required: boolean;
    min?: number;
    max?: number;
    defaultValue?: number;
  };
  validationRules?: ValidationRule[];
}

export interface ValidationRule {
  id: string;
  name: string;
  description: string;
  type: 'property' | 'relationship' | 'custom';
  expression: string;
  errorMessage: string;
  severity: 'error' | 'warning' | 'info';
}

export interface SchemaVersion {
  version: string;
  timestamp: string;
  description: string;
  changes: SchemaChange[];
  author?: string;
}

export interface SchemaChange {
  type: 'add' | 'modify' | 'remove';
  target: 'node_type' | 'edge_type' | 'property' | 'validation_rule';
  targetId: string;
  description: string;
  migrationScript?: string;
}

export interface SchemaTemplate {
  id: string;
  name: string;
  description: string;
  category: string;
  nodeTypes: NodeTypeDefinition[];
  edgeTypes: EdgeTypeDefinition[];
  sampleData?: {
    nodes: any[];
    edges: any[];
  };
}

export interface SchemaValidationResult {
  isValid: boolean;
  errors: SchemaValidationError[];
  warnings: SchemaValidationWarning[];
  statistics: {
    totalNodes: number;
    totalEdges: number;
    validNodes: number;
    validEdges: number;
    invalidNodes: number;
    invalidEdges: number;
  };
}

export interface SchemaValidationError {
  id: string;
  type: 'node' | 'edge' | 'property' | 'relationship';
  targetId: string;
  rule: string;
  message: string;
  severity: 'error' | 'warning';
  suggestedFix?: string;
}

export interface SchemaValidationWarning {
  id: string;
  type: 'node' | 'edge' | 'property' | 'relationship';
  targetId: string;
  message: string;
  recommendation?: string;
}

export interface SchemaMigration {
  id: string;
  fromVersion: string;
  toVersion: string;
  description: string;
  steps: MigrationStep[];
  rollbackSteps: MigrationStep[];
  estimatedDuration?: number;
  riskLevel: 'low' | 'medium' | 'high';
}

export interface MigrationStep {
  id: string;
  type: 'add_property' | 'remove_property' | 'modify_property' | 'add_type' | 'remove_type' | 'custom';
  description: string;
  script: string;
  rollbackScript?: string;
  dependencies?: string[];
}

export interface SchemaStatistics {
  nodeTypes: {
    total: number;
    withProperties: number;
    withValidation: number;
    inheritance: number;
  };
  edgeTypes: {
    total: number;
    directed: number;
    undirected: number;
    bidirectional: number;
    withConstraints: number;
  };
  properties: {
    total: number;
    required: number;
    optional: number;
    indexed: number;
    withDefaults: number;
  };
  validationRules: {
    total: number;
    errors: number;
    warnings: number;
    custom: number;
  };
  complexity: {
    score: number;
    factors: string[];
    recommendations: string[];
  };
}

export interface SchemaExportOptions {
  format: 'json' | 'yaml' | 'graphql' | 'typescript';
  includeValidation: boolean;
  includeExamples: boolean;
  includeDocumentation: boolean;
  minify: boolean;
}

export interface SchemaImportOptions {
  format: 'json' | 'yaml' | 'graphql';
  mergeStrategy: 'replace' | 'merge' | 'append';
  validateBeforeImport: boolean;
  createBackup: boolean;
}

// UI State Types
export interface SchemaManagerState {
  currentSchema: GraphSchema | null;
  nodeTypes: NodeTypeDefinition[];
  edgeTypes: EdgeTypeDefinition[];
  validationResults: SchemaValidationResult | null;
  isLoading: boolean;
  error: string | null;
  selectedNodeType: string | null;
  selectedEdgeType: string | null;
  showValidation: boolean;
  showMigration: boolean;
}

export interface SchemaEditorState {
  mode: 'create' | 'edit' | 'view';
  target: 'node' | 'edge' | null;
  targetId: string | null;
  isDirty: boolean;
  validationErrors: string[];
}

// Re-export from graph types for convenience
export type { GraphSchema, NodeTypeSchema, EdgeTypeSchema };