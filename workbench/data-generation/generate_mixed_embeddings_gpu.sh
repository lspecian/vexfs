#!/bin/bash

# VexFS 200GB Testing Workbench - GPU-Enhanced Mixed Embeddings Generation
# Integrates with existing workbench structure and adds GPU acceleration

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
WORKBENCH_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EMBEDDINGS_DIR="$DATA_DIR/embeddings"

# GPU Configuration
USE_GPU=${USE_GPU:-true}
FORCE_CPU=${FORCE_CPU:-false}
BATCH_SIZE=${BATCH_SIZE:-64}
TARGET_SIZE_GB=${TARGET_SIZE_GB:-200}

echo -e "${BLUE}🚀 VexFS GPU-Enhanced Mixed Embeddings Generation${NC}"
echo "=================================================================="
echo "Target: ${TARGET_SIZE_GB}GB total embeddings"
echo "GPU Acceleration: ${USE_GPU}"
echo "Batch Size: ${BATCH_SIZE}"
echo "=================================================================="

# Function to print status
print_status() {
    local status=$1
    local message=$2
    if [ "$status" = "OK" ]; then
        echo -e "${GREEN}✅ $message${NC}"
    elif [ "$status" = "WARNING" ]; then
        echo -e "${YELLOW}⚠️  $message${NC}"
    else
        echo -e "${RED}❌ $message${NC}"
    fi
}

# Check GPU availability
check_gpu() {
    echo -e "\n${BLUE}🔍 Checking GPU availability...${NC}"
    
    if command -v nvidia-smi &> /dev/null; then
        if nvidia-smi > /dev/null 2>&1; then
            GPU_INFO=$(nvidia-smi --query-gpu=name,memory.total --format=csv,noheader,nounits | head -1)
            print_status "OK" "GPU detected: ${GPU_INFO}"
            return 0
        else
            print_status "WARNING" "NVIDIA GPU detected but drivers not working"
            return 1
        fi
    else
        print_status "WARNING" "No NVIDIA GPU detected"
        return 1
    fi
}

# Install GPU dependencies
install_gpu_dependencies() {
    echo -e "\n${BLUE}📦 Installing GPU dependencies...${NC}"
    
    # Create virtual environment if it doesn't exist
    if [ ! -d "$WORKBENCH_ROOT/workbench_env" ]; then
        echo "Creating Python virtual environment..."
        python3 -m venv "$WORKBENCH_ROOT/workbench_env"
    fi
    
    # Activate virtual environment
    source "$WORKBENCH_ROOT/workbench_env/bin/activate"
    
    # Upgrade pip
    pip install --upgrade pip > /dev/null 2>&1
    
    # Check if GPU is available and install appropriate packages
    if check_gpu && [ "$FORCE_CPU" = false ]; then
        print_status "OK" "Installing GPU-accelerated packages..."
        
        # Install PyTorch with CUDA support
        pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu118 > /dev/null 2>&1
        
        # Install sentence-transformers with GPU support
        pip install sentence-transformers[gpu] > /dev/null 2>&1
        
        # Install additional GPU packages
        pip install cupy-cuda11x > /dev/null 2>&1 || echo "CuPy installation skipped (optional)"
        
        print_status "OK" "GPU packages installed"
    else
        print_status "WARNING" "Installing CPU-only packages..."
        
        # Install CPU versions
        pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cpu > /dev/null 2>&1
        pip install sentence-transformers > /dev/null 2>&1
        
        print_status "OK" "CPU packages installed"
    fi
    
    # Install common dependencies
    pip install numpy scikit-learn tqdm opencv-python > /dev/null 2>&1
    
    print_status "OK" "All dependencies installed"
}

# Generate embeddings using GPU-accelerated script
generate_gpu_embeddings() {
    echo -e "\n${BLUE}🧠 Generating GPU-accelerated embeddings...${NC}"
    
    # Activate virtual environment
    source "$WORKBENCH_ROOT/workbench_env/bin/activate"
    
    # Create output directory
    mkdir -p "$EMBEDDINGS_DIR"
    
    # Set GPU options
    local gpu_args=""
    if [ "$FORCE_CPU" = true ]; then
        gpu_args="--cpu-only"
    fi
    
    # Run the GPU embedding generation script
    python3 "$DATA_DIR/generate_gpu_embeddings.py" \
        --size "$TARGET_SIZE_GB" \
        --output "$EMBEDDINGS_DIR" \
        --batch-size "$BATCH_SIZE" \
        $gpu_args
    
    if [ $? -eq 0 ]; then
        print_status "OK" "GPU embedding generation completed"
        
        # Display results
        if [ -f "$EMBEDDINGS_DIR/generation_report.json" ]; then
            echo -e "\n${BLUE}📊 Generation Report:${NC}"
            python3 -c "
import json
with open('$EMBEDDINGS_DIR/generation_report.json', 'r') as f:
    report = json.load(f)
print(f'Total embeddings: {report[\"total_embeddings\"]:,}')
print(f'Estimated size: {report[\"estimated_size_gb\"]:.2f} GB')
print(f'Generation time: {report[\"generation_time\"]:.1f} seconds')
print(f'Rate: {report[\"embeddings_per_second\"]:.0f} embeddings/second')
print(f'GPU used: {\"✅\" if report[\"gpu_used\"] else \"❌\"}')
if report[\"gpu_used\"]:
    print(f'GPU: {report[\"gpu_name\"]}')
"
        fi
    else
        print_status "ERROR" "GPU embedding generation failed"
        return 1
    fi
}

# Verify embeddings
verify_embeddings() {
    echo -e "\n${BLUE}🔍 Verifying generated embeddings...${NC}"
    
    local files=("text_embeddings.npz" "image_embeddings.npz" "code_embeddings.npz")
    local total_size=0
    
    for file in "${files[@]}"; do
        if [ -f "$EMBEDDINGS_DIR/$file" ]; then
            local size=$(du -m "$EMBEDDINGS_DIR/$file" | cut -f1)
            total_size=$((total_size + size))
            print_status "OK" "$file: ${size}MB"
        else
            print_status "ERROR" "$file not found"
        fi
    done
    
    local total_size_gb=$((total_size / 1024))
    echo -e "\n${GREEN}📊 Total embeddings size: ${total_size}MB (~${total_size_gb}GB)${NC}"
    
    if [ $total_size_gb -ge $((TARGET_SIZE_GB * 80 / 100)) ]; then
        print_status "OK" "Target size achieved (${total_size_gb}GB >= ${TARGET_SIZE_GB}GB)"
    else
        print_status "WARNING" "Target size not fully achieved (${total_size_gb}GB < ${TARGET_SIZE_GB}GB)"
    fi
}

# Main execution
main() {
    echo -e "${BLUE}🚀 Starting GPU-enhanced embedding generation...${NC}"
    
    # Check dependencies
    if ! command -v python3 &> /dev/null; then
        print_status "ERROR" "Python 3 is required but not installed"
        exit 1
    fi
    
    # Install dependencies
    install_gpu_dependencies
    
    # Generate embeddings
    generate_gpu_embeddings
    
    # Verify results
    verify_embeddings
    
    echo -e "\n${GREEN}🎉 GPU-enhanced embedding generation complete!${NC}"
    echo -e "${BLUE}📁 Embeddings saved to: ${EMBEDDINGS_DIR}${NC}"
    echo -e "${BLUE}📊 Ready for VexFS testing with 200GB of mixed embeddings${NC}"
}

# Run main function
main "$@"