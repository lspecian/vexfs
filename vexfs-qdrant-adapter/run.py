#!/usr/bin/env python3
"""
VexFS v2 Qdrant Adapter Startup Script

This script provides a convenient way to start the VexFS Qdrant adapter
with proper environment validation and configuration.
"""

import sys
import os
import subprocess
import argparse
from pathlib import Path

# Add src to Python path
sys.path.insert(0, str(Path(__file__).parent / "src"))

from src.utils.config import get_config, validate_environment, print_config_summary
from src.utils.logging import setup_logging, get_logger

def check_vexfs_module():
    """Check if VexFS v2 kernel module is loaded"""
    try:
        result = subprocess.run(['lsmod'], capture_output=True, text=True)
        return 'vexfs' in result.stdout
    except:
        return False

def check_vexfs_device():
    """Check if VexFS v2 device exists"""
    config = get_config()
    return os.path.exists(config.vexfs.device_path)

def load_vexfs_module():
    """Attempt to load VexFS v2 kernel module"""
    print("Attempting to load VexFS v2 kernel module...")
    
    # Look for kernel module in common locations
    module_paths = [
        "kernel/vexfs_v2_build/vexfs_v2_phase3.ko",
        "../kernel/vexfs_v2_build/vexfs_v2_phase3.ko",
        "/lib/modules/$(uname -r)/extra/vexfs_v2_phase3.ko"
    ]
    
    for module_path in module_paths:
        if os.path.exists(module_path):
            try:
                subprocess.run(['sudo', 'insmod', module_path], check=True)
                print(f"Successfully loaded VexFS module from {module_path}")
                return True
            except subprocess.CalledProcessError as e:
                print(f"Failed to load module from {module_path}: {e}")
                continue
    
    print("Could not find or load VexFS v2 kernel module")
    return False

def main():
    """Main entry point"""
    parser = argparse.ArgumentParser(description="VexFS v2 Qdrant Adapter")
    parser.add_argument("--host", default="0.0.0.0", help="Host to bind to")
    parser.add_argument("--port", type=int, default=6333, help="Port to bind to")
    parser.add_argument("--workers", type=int, default=1, help="Number of workers")
    parser.add_argument("--reload", action="store_true", help="Enable auto-reload")
    parser.add_argument("--log-level", default="INFO", help="Log level")
    parser.add_argument("--device", default="/dev/vexfs_v2_phase3", help="VexFS device path")
    parser.add_argument("--check-only", action="store_true", help="Only check environment")
    parser.add_argument("--load-module", action="store_true", help="Attempt to load VexFS module")
    
    args = parser.parse_args()
    
    # Setup logging
    setup_logging()
    logger = get_logger(__name__)
    
    print("=" * 60)
    print("VexFS v2 Qdrant Adapter")
    print("=" * 60)
    
    # Override config with command line arguments
    config = get_config()
    config.api.host = args.host
    config.api.port = args.port
    config.api.workers = args.workers
    config.api.reload = args.reload
    config.logging.level = args.log_level
    config.vexfs.device_path = args.device
    
    # Check VexFS module
    if not check_vexfs_module():
        print("⚠️  VexFS v2 kernel module not loaded")
        if args.load_module:
            if not load_vexfs_module():
                print("❌ Failed to load VexFS module")
                sys.exit(1)
        else:
            print("💡 Use --load-module to attempt automatic loading")
            print("💡 Or manually load with: sudo insmod /path/to/vexfs_v2_phase3.ko")
    else:
        print("✅ VexFS v2 kernel module is loaded")
    
    # Check VexFS device
    if not check_vexfs_device():
        print(f"❌ VexFS device not found: {config.vexfs.device_path}")
        print("💡 Ensure the VexFS kernel module is loaded and device is created")
        if not args.check_only:
            sys.exit(1)
    else:
        print(f"✅ VexFS device found: {config.vexfs.device_path}")
    
    # Validate environment
    is_valid, errors = validate_environment()
    if not is_valid:
        print("❌ Environment validation failed:")
        for error in errors:
            print(f"   - {error}")
        if not args.check_only:
            sys.exit(1)
    else:
        print("✅ Environment validation passed")
    
    if args.check_only:
        print("\n✅ Environment check complete")
        return
    
    # Print configuration summary
    print("\n" + "=" * 60)
    print_config_summary()
    
    # Start the server
    print("🚀 Starting VexFS v2 Qdrant Adapter...")
    
    try:
        import uvicorn
        from src.main import app
        
        uvicorn.run(
            app,
            host=config.api.host,
            port=config.api.port,
            workers=config.api.workers,
            reload=config.api.reload,
            log_level=config.logging.level.lower(),
            access_log=config.logging.log_requests
        )
    except KeyboardInterrupt:
        print("\n🛑 Shutting down VexFS v2 Qdrant Adapter")
    except Exception as e:
        logger.error(f"Failed to start server: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()