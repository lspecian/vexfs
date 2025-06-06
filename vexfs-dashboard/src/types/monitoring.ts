// Monitoring Types for VexFS Dashboard

export interface SystemMetrics {
  cpu: {
    usage: number; // percentage
    cores: number;
    loadAverage: number[];
  };
  memory: {
    used: number; // bytes
    total: number; // bytes
    available: number; // bytes
    percentage: number;
  };
  disk: {
    used: number; // bytes
    total: number; // bytes
    available: number; // bytes
    percentage: number;
  };
  network: {
    bytesIn: number;
    bytesOut: number;
    packetsIn: number;
    packetsOut: number;
  };
}

export interface PerformanceMetrics {
  queryPerformance: {
    averageResponseTime: number; // milliseconds
    p95ResponseTime: number;
    p99ResponseTime: number;
    throughput: number; // queries per second
    totalQueries: number;
  };
  vectorOperations: {
    indexingRate: number; // vectors per second
    searchRate: number; // searches per second
    totalIndexed: number;
    totalSearches: number;
  };
  storage: {
    readThroughput: number; // MB/s
    writeThroughput: number; // MB/s
    iops: number; // operations per second
  };
}

export interface HealthStatus {
  overall: 'healthy' | 'warning' | 'critical' | 'unknown';
  services: {
    vexfsCore: ServiceHealth;
    database: ServiceHealth;
    vectorIndex: ServiceHealth;
    api: ServiceHealth;
  };
  uptime: number; // seconds
  lastHealthCheck: string; // ISO timestamp
}

export interface ServiceHealth {
  status: 'healthy' | 'warning' | 'critical' | 'unknown';
  responseTime?: number; // milliseconds
  errorRate?: number; // percentage
  lastCheck: string; // ISO timestamp
  message?: string;
}

export interface Alert {
  id: string;
  type: 'info' | 'warning' | 'error' | 'critical';
  title: string;
  message: string;
  timestamp: string; // ISO timestamp
  acknowledged: boolean;
  source: string; // component that generated the alert
  metadata?: Record<string, unknown>;
}

export interface AlertRule {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  metric: string;
  condition: 'greater_than' | 'less_than' | 'equals' | 'not_equals';
  threshold: number;
  severity: 'info' | 'warning' | 'error' | 'critical';
  cooldown: number; // seconds
  createdAt: string;
  updatedAt: string;
}

export interface MonitoringConfig {
  refreshInterval: number; // seconds
  retentionPeriod: number; // days
  alertsEnabled: boolean;
  realTimeUpdates: boolean;
  exportFormat: 'json' | 'csv' | 'xlsx';
}

export interface TimeSeriesData {
  timestamp: string; // ISO timestamp
  value: number;
  label?: string;
}

export interface ChartData {
  labels: string[];
  datasets: {
    label: string;
    data: number[];
    borderColor?: string;
    backgroundColor?: string;
    fill?: boolean;
  }[];
}

export interface MetricWidget {
  id: string;
  title: string;
  type: 'gauge' | 'counter' | 'chart' | 'status';
  metric: string;
  unit?: string;
  format?: 'number' | 'percentage' | 'bytes' | 'duration';
  thresholds?: {
    warning: number;
    critical: number;
  };
  size: 'small' | 'medium' | 'large';
  position: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
}

export interface DashboardLayout {
  id: string;
  name: string;
  description?: string;
  widgets: MetricWidget[];
  isDefault: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface MonitoringStats {
  totalAlerts: number;
  activeAlerts: number;
  acknowledgedAlerts: number;
  systemHealth: 'healthy' | 'warning' | 'critical';
  lastUpdate: string;
}

export interface RealTimeUpdate {
  type: 'metrics' | 'health' | 'alert' | 'performance';
  data: SystemMetrics | HealthStatus | Alert | PerformanceMetrics;
  timestamp: string;
}

export interface ExportOptions {
  format: 'json' | 'csv' | 'xlsx' | 'pdf';
  timeRange: {
    start: string;
    end: string;
  };
  metrics: string[];
  includeAlerts: boolean;
  includeHealth: boolean;
}

// API Response types for monitoring endpoints
export interface MonitoringApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  timestamp: string;
}

export interface MetricsHistoryResponse {
  metrics: {
    [metricName: string]: TimeSeriesData[];
  };
  timeRange: {
    start: string;
    end: string;
  };
  resolution: string; // e.g., '1m', '5m', '1h'
}
