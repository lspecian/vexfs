"""
Configuration Management

This module handles configuration for the VexFS Qdrant adapter,
including VexFS device settings, API configuration, and performance tuning.
"""

import os
from typing import Optional
from pydantic_settings import BaseSettings
from pydantic import Field


class VexFSConfig(BaseSettings):
    """VexFS-specific configuration"""
    device_path: str = Field(
        default="/dev/vexfs_v2_phase3",
        description="Path to VexFS v2 device file"
    )
    
    # Performance settings
    batch_size_limit: int = Field(
        default=1000,
        description="Maximum batch size for operations"
    )
    
    search_limit_max: int = Field(
        default=10000,
        description="Maximum search results limit"
    )
    
    vector_dimensions_max: int = Field(
        default=65536,
        description="Maximum vector dimensions"
    )
    
    # Timeout settings
    operation_timeout: int = Field(
        default=30,
        description="Operation timeout in seconds"
    )
    
    # Memory settings
    memory_alignment: int = Field(
        default=32,
        description="Memory alignment for vectors"
    )

    class Config:
        env_prefix = "VEXFS_"


class APIConfig(BaseSettings):
    """API server configuration"""
    host: str = Field(
        default="0.0.0.0",
        description="API server host"
    )
    
    port: int = Field(
        default=6333,
        description="REST API server port (Qdrant default)"
    )
    
    grpc_port: int = Field(
        default=6334,
        description="gRPC server port (Qdrant default)"
    )
    
    workers: int = Field(
        default=1,
        description="Number of worker processes"
    )
    
    reload: bool = Field(
        default=False,
        description="Enable auto-reload for development"
    )
    
    # gRPC specific settings
    grpc_max_message_size: int = Field(
        default=100 * 1024 * 1024,  # 100MB
        description="Maximum gRPC message size in bytes"
    )
    
    grpc_keepalive_time: int = Field(
        default=30000,
        description="gRPC keepalive time in milliseconds"
    )
    
    grpc_keepalive_timeout: int = Field(
        default=5000,
        description="gRPC keepalive timeout in milliseconds"
    )
    
    api_key: Optional[str] = Field(
        default=None,
        description="Optional API key for authentication"
    )
    
    # CORS settings
    cors_origins: list = Field(
        default=["*"],
        description="CORS allowed origins"
    )
    
    cors_methods: list = Field(
        default=["GET", "POST", "PUT", "DELETE", "OPTIONS"],
        description="CORS allowed methods"
    )
    
    # Request limits
    max_request_size: int = Field(
        default=100 * 1024 * 1024,  # 100MB
        description="Maximum request size in bytes"
    )
    
    request_timeout: int = Field(
        default=60,
        description="Request timeout in seconds"
    )

    class Config:
        env_prefix = "API_"


class LoggingConfig(BaseSettings):
    """Logging configuration"""
    level: str = Field(
        default="INFO",
        description="Logging level"
    )
    
    format: str = Field(
        default="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
        description="Log format"
    )
    
    file_path: Optional[str] = Field(
        default=None,
        description="Log file path (None for stdout)"
    )
    
    max_file_size: int = Field(
        default=10 * 1024 * 1024,  # 10MB
        description="Maximum log file size in bytes"
    )
    
    backup_count: int = Field(
        default=5,
        description="Number of backup log files"
    )
    
    # Performance logging
    log_performance: bool = Field(
        default=True,
        description="Enable performance logging"
    )
    
    log_requests: bool = Field(
        default=True,
        description="Enable request logging"
    )

    class Config:
        env_prefix = "LOG_"


class PerformanceConfig(BaseSettings):
    """Performance monitoring configuration"""
    enable_metrics: bool = Field(
        default=True,
        description="Enable performance metrics collection"
    )
    
    metrics_port: int = Field(
        default=8000,
        description="Prometheus metrics port"
    )
    
    # Performance targets (ops/sec)
    target_metadata_ops: int = Field(
        default=200000,
        description="Target metadata operations per second"
    )
    
    target_search_ops: int = Field(
        default=100000,
        description="Target search operations per second"
    )
    
    target_insert_ops: int = Field(
        default=50000,
        description="Target insert operations per second"
    )
    
    # Monitoring intervals
    stats_interval: int = Field(
        default=60,
        description="Statistics collection interval in seconds"
    )

    class Config:
        env_prefix = "PERF_"


class Config:
    """Main configuration class combining all settings"""
    
    def __init__(self):
        self.vexfs = VexFSConfig()
        self.api = APIConfig()
        self.logging = LoggingConfig()
        self.performance = PerformanceConfig()
    
    @classmethod
    def from_env(cls) -> "Config":
        """Create configuration from environment variables"""
        return cls()
    
    def validate_vexfs_device(self) -> bool:
        """Validate that VexFS device is accessible"""
        try:
            if not os.path.exists(self.vexfs.device_path):
                return False
            
            # Try to open the device (read-only test)
            with open(self.vexfs.device_path, 'rb') as f:
                pass
            
            return True
        except (OSError, PermissionError):
            return False
    
    def get_api_url(self) -> str:
        """Get the full API URL"""
        return f"http://{self.api.host}:{self.api.port}"
    
    def get_metrics_url(self) -> str:
        """Get the metrics URL"""
        return f"http://{self.api.host}:{self.performance.metrics_port}/metrics"
    
    def to_dict(self) -> dict:
        """Convert configuration to dictionary"""
        return {
            "vexfs": self.vexfs.dict(),
            "api": self.api.dict(),
            "logging": self.logging.dict(),
            "performance": self.performance.dict()
        }


# Global configuration instance
config = Config.from_env()


def get_config() -> Config:
    """Get the global configuration instance"""
    return config


def validate_environment() -> tuple[bool, list[str]]:
    """
    Validate the environment for running the VexFS Qdrant adapter.
    
    Returns:
        Tuple of (is_valid, error_messages)
    """
    errors = []
    
    # Check VexFS device
    if not config.validate_vexfs_device():
        errors.append(f"VexFS device not accessible: {config.vexfs.device_path}")
    
    # Check port availability
    import socket
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.bind((config.api.host, config.api.port))
        sock.close()
    except OSError:
        errors.append(f"Port {config.api.port} is not available")
    
    # Check log file permissions if specified
    if config.logging.file_path:
        try:
            log_dir = os.path.dirname(config.logging.file_path)
            if log_dir and not os.path.exists(log_dir):
                os.makedirs(log_dir, exist_ok=True)
            
            # Test write access
            test_file = config.logging.file_path + ".test"
            with open(test_file, 'w') as f:
                f.write("test")
            os.remove(test_file)
        except (OSError, PermissionError):
            errors.append(f"Cannot write to log file: {config.logging.file_path}")
    
    return len(errors) == 0, errors


def print_config_summary():
    """Print a summary of the current configuration"""
    print("VexFS Qdrant Adapter Configuration:")
    print(f"  VexFS Device: {config.vexfs.device_path}")
    print(f"  API Server: {config.get_api_url()}")
    print(f"  Log Level: {config.logging.level}")
    print(f"  Performance Metrics: {config.performance.enable_metrics}")
    if config.performance.enable_metrics:
        print(f"  Metrics URL: {config.get_metrics_url()}")
    print()