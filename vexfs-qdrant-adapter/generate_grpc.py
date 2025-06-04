#!/usr/bin/env python3
"""
Generate gRPC Python stubs from protobuf definitions

This script generates the Python gRPC stubs required for the VexFS v2 Qdrant adapter
gRPC server implementation.
"""

import subprocess
import sys
import os
from pathlib import Path

def generate_grpc_stubs():
    """Generate Python gRPC stubs from protobuf definitions"""
    
    # Get the project root directory
    project_root = Path(__file__).parent
    proto_dir = project_root / "src" / "proto"
    
    # Ensure the proto directory exists
    if not proto_dir.exists():
        print(f"Error: Proto directory {proto_dir} does not exist")
        return False
    
    # Check if protoc is available
    try:
        subprocess.run(["protoc", "--version"], check=True, capture_output=True)
    except (subprocess.CalledProcessError, FileNotFoundError):
        print("Error: protoc (Protocol Buffer Compiler) not found")
        print("Please install protobuf compiler:")
        print("  Ubuntu/Debian: sudo apt-get install protobuf-compiler")
        print("  macOS: brew install protobuf")
        print("  Or install via pip: pip install grpcio-tools")
        return False
    
    # Generate Python stubs
    proto_file = proto_dir / "qdrant.proto"
    
    if not proto_file.exists():
        print(f"Error: Proto file {proto_file} does not exist")
        return False
    
    # Command to generate Python protobuf and gRPC stubs
    cmd = [
        "python", "-m", "grpc_tools.protoc",
        f"--proto_path={proto_dir}",
        f"--python_out={proto_dir}",
        f"--grpc_python_out={proto_dir}",
        str(proto_file)
    ]
    
    try:
        print("Generating gRPC stubs...")
        print(f"Command: {' '.join(cmd)}")
        
        result = subprocess.run(cmd, check=True, capture_output=True, text=True)
        
        print("✅ Successfully generated gRPC stubs:")
        print(f"  - {proto_dir}/qdrant_pb2.py")
        print(f"  - {proto_dir}/qdrant_pb2_grpc.py")
        
        return True
        
    except subprocess.CalledProcessError as e:
        print(f"Error generating gRPC stubs: {e}")
        print(f"stdout: {e.stdout}")
        print(f"stderr: {e.stderr}")
        return False

def fix_imports():
    """Fix relative imports in generated files"""
    proto_dir = Path(__file__).parent / "src" / "proto"
    
    # Fix imports in qdrant_pb2_grpc.py
    grpc_file = proto_dir / "qdrant_pb2_grpc.py"
    if grpc_file.exists():
        content = grpc_file.read_text()
        # Replace absolute import with relative import
        content = content.replace("import qdrant_pb2", "from . import qdrant_pb2")
        grpc_file.write_text(content)
        print("✅ Fixed imports in qdrant_pb2_grpc.py")

if __name__ == "__main__":
    print("VexFS v2 Qdrant Adapter - gRPC Stub Generator")
    print("=" * 50)
    
    if generate_grpc_stubs():
        fix_imports()
        print("\n🎉 gRPC stub generation completed successfully!")
        print("\nNext steps:")
        print("1. Install dependencies: pip install -r requirements.txt")
        print("2. Run the gRPC server: python -m src.main")
    else:
        print("\n❌ gRPC stub generation failed!")
        sys.exit(1)