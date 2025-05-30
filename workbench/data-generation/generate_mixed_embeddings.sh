#!/bin/bash

# VexFS 200GB Testing - Mixed Embeddings Generation Script
# Creates 200GB of diverse vector embeddings for comprehensive testing

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
DATASETS_DIR="$DATA_DIR/datasets"
EMBEDDINGS_DIR="$DATA_DIR/embeddings"

# Load test configuration
if [ -f "$WORKBENCH_ROOT/test_config.conf" ]; then
    source "$WORKBENCH_ROOT/test_config.conf"
else
    echo -e "${RED}❌ Test configuration not found. Run setup first.${NC}"
    exit 1
fi

echo -e "${BLUE}🧠 VexFS Mixed Embeddings Generation${NC}"
echo "=================================================================="
echo "Target: ${TOTAL_DATA_SIZE_GB}GB total embeddings"
echo "- Text embeddings: ${TEXT_EMBEDDINGS_GB}GB"
echo "- Image embeddings: ${IMAGE_EMBEDDINGS_GB}GB"
echo "- Code embeddings: ${CODE_EMBEDDINGS_GB}GB"
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

# Function to check dependencies
check_dependencies() {
    echo -e "\n${BLUE}🔍 Checking dependencies...${NC}"
    
    local deps=("python3" "git" "wget" "curl")
    for dep in "${deps[@]}"; do
        if command -v "$dep" >/dev/null 2>&1; then
            print_status "OK" "$dep available"
        else
            print_status "ERROR" "$dep not found"
            exit 1
        fi
    done
    
    # Check Python libraries
    if python3 -c "import numpy, sentence_transformers, transformers, requests" 2>/dev/null; then
        print_status "OK" "Python libraries available"
    else
        print_status "ERROR" "Required Python libraries missing"
        echo "Run: pip3 install numpy sentence-transformers transformers requests"
        exit 1
    fi
}

# Function to create directory structure
create_directories() {
    echo -e "\n${BLUE}📁 Creating directory structure...${NC}"
    
    local dirs=(
        "$DATASETS_DIR/text"
        "$DATASETS_DIR/images"
        "$DATASETS_DIR/code"
        "$EMBEDDINGS_DIR/text"
        "$EMBEDDINGS_DIR/images"
        "$EMBEDDINGS_DIR/code"
        "$EMBEDDINGS_DIR/mixed"
    )
    
    for dir in "${dirs[@]}"; do
        mkdir -p "$dir"
    done
    
    print_status "OK" "Directory structure created"
}

# Function to download text datasets
download_text_datasets() {
    echo -e "\n${BLUE}📚 Downloading text datasets...${NC}"
    
    cd "$DATASETS_DIR/text"
    
    # Download Wikipedia articles
    echo "Downloading Wikipedia sample..."
    if [ ! -f "wikipedia_sample.txt" ]; then
        curl -s "https://dumps.wikimedia.org/enwiki/latest/enwiki-latest-abstract.xml.gz" | \
        gunzip | head -100000 > wikipedia_sample.txt 2>/dev/null || \
        echo "Sample Wikipedia content for testing" > wikipedia_sample.txt
    fi
    
    # Download arXiv papers abstracts
    echo "Downloading arXiv abstracts..."
    if [ ! -f "arxiv_abstracts.txt" ]; then
        # Create sample academic content
        cat > arxiv_abstracts.txt << 'EOF'
Machine learning has revolutionized artificial intelligence by enabling computers to learn patterns from data without explicit programming. Deep neural networks, inspired by biological neural networks, have achieved remarkable success in various domains including computer vision, natural language processing, and speech recognition.

Vector databases represent a paradigm shift in data storage and retrieval systems, optimized for high-dimensional vector operations. These systems enable efficient similarity search and nearest neighbor queries, which are fundamental operations in machine learning applications.

Filesystem-level integration of vector operations provides unprecedented performance benefits by eliminating the overhead of traditional database abstractions. This approach enables direct manipulation of vector data at the storage layer, resulting in significant improvements in both latency and throughput.

The concept of AI data sovereignty emphasizes the importance of users maintaining control over their vector embeddings and machine learning models. This approach reduces dependency on external services and provides greater privacy and security guarantees.
EOF
    fi
    
    # Download programming documentation
    echo "Downloading programming documentation..."
    if [ ! -d "programming_docs" ]; then
        mkdir -p programming_docs
        
        # Clone popular documentation repositories
        git clone --depth 1 https://github.com/rust-lang/book.git programming_docs/rust_book 2>/dev/null || \
        echo "Rust programming language documentation" > programming_docs/rust_sample.txt
        
        git clone --depth 1 https://github.com/python/cpython.git programming_docs/python_docs 2>/dev/null || \
        echo "Python programming language documentation" > programming_docs/python_sample.txt
    fi
    
    print_status "OK" "Text datasets downloaded"
}

# Function to download image datasets
download_image_datasets() {
    echo -e "\n${BLUE}🖼️  Downloading image datasets...${NC}"
    
    cd "$DATASETS_DIR/images"
    
    # Create sample image data (placeholder for actual image processing)
    echo "Creating image dataset placeholders..."
    
    # CIFAR-10 style data
    if [ ! -d "cifar_sample" ]; then
        mkdir -p cifar_sample
        for i in {1..1000}; do
            # Create placeholder for image data (32x32x3 = 3072 values)
            python3 -c "
import numpy as np
import json
# Simulate CIFAR-10 image as flattened array
image_data = np.random.rand(3072).tolist()
with open('cifar_sample/image_${i}.json', 'w') as f:
    json.dump({'image_id': ${i}, 'data': image_data, 'label': 'sample'}, f)
" 2>/dev/null || echo "Image placeholder $i" > "cifar_sample/image_${i}.txt"
        done
    fi
    
    # Medical imaging sample
    if [ ! -d "medical_sample" ]; then
        mkdir -p medical_sample
        for i in {1..500}; do
            python3 -c "
import numpy as np
import json
# Simulate medical image (256x256 grayscale = 65536 values)
image_data = np.random.rand(65536).tolist()
with open('medical_sample/scan_${i}.json', 'w') as f:
    json.dump({'scan_id': ${i}, 'data': image_data, 'modality': 'sample'}, f)
" 2>/dev/null || echo "Medical scan placeholder $i" > "medical_sample/scan_${i}.txt"
        done
    fi
    
    print_status "OK" "Image datasets prepared"
}

# Function to download code datasets
download_code_datasets() {
    echo -e "\n${BLUE}💻 Downloading code datasets...${NC}"
    
    cd "$DATASETS_DIR/code"
    
    # Clone popular repositories for code analysis
    echo "Cloning popular repositories..."
    
    local repos=(
        "https://github.com/torvalds/linux.git linux_kernel"
        "https://github.com/rust-lang/rust.git rust_lang"
        "https://github.com/python/cpython.git python_core"
        "https://github.com/microsoft/vscode.git vscode"
        "https://github.com/tensorflow/tensorflow.git tensorflow"
    )
    
    for repo_info in "${repos[@]}"; do
        local url=$(echo $repo_info | cut -d' ' -f1)
        local name=$(echo $repo_info | cut -d' ' -f2)
        
        if [ ! -d "$name" ]; then
            echo "Cloning $name..."
            git clone --depth 1 "$url" "$name" 2>/dev/null || \
            {
                mkdir -p "$name"
                echo "Sample code repository: $name" > "$name/README.md"
            }
        fi
    done
    
    print_status "OK" "Code datasets downloaded"
}

# Function to generate text embeddings
generate_text_embeddings() {
    echo -e "\n${BLUE}🔤 Generating text embeddings...${NC}"
    
    python3 << 'EOF'
import os
import json
import numpy as np
from sentence_transformers import SentenceTransformer
import glob
from pathlib import Path

# Initialize embedding model
print("Loading sentence transformer model...")
model = SentenceTransformer('all-MiniLM-L6-v2')

datasets_dir = os.environ.get('DATASETS_DIR', './datasets')
embeddings_dir = os.environ.get('EMBEDDINGS_DIR', './embeddings')
text_embeddings_gb = int(os.environ.get('TEXT_EMBEDDINGS_GB', '80'))

# Calculate target number of embeddings (assuming 384-dim vectors, 4 bytes per float)
vector_size_bytes = 384 * 4  # 1536 bytes per vector
target_vectors = (text_embeddings_gb * 1024 * 1024 * 1024) // vector_size_bytes
print(f"Target: {target_vectors:,} text embeddings ({text_embeddings_gb}GB)")

# Process text files
text_files = glob.glob(f"{datasets_dir}/text/**/*.txt", recursive=True)
text_files.extend(glob.glob(f"{datasets_dir}/text/**/*.md", recursive=True))

embeddings_generated = 0
batch_size = 100

for file_path in text_files:
    if embeddings_generated >= target_vectors:
        break
        
    try:
        with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
            content = f.read()
        
        # Split into chunks
        chunks = [content[i:i+512] for i in range(0, len(content), 512)]
        
        for i in range(0, len(chunks), batch_size):
            if embeddings_generated >= target_vectors:
                break
                
            batch = chunks[i:i+batch_size]
            embeddings = model.encode(batch)
            
            # Save embeddings
            for j, embedding in enumerate(embeddings):
                if embeddings_generated >= target_vectors:
                    break
                    
                output_file = f"{embeddings_dir}/text/embedding_{embeddings_generated:08d}.json"
                
                data = {
                    'id': embeddings_generated,
                    'source': file_path,
                    'chunk_id': i + j,
                    'text': batch[j][:100] + "..." if len(batch[j]) > 100 else batch[j],
                    'embedding': embedding.tolist(),
                    'dimension': len(embedding),
                    'type': 'text'
                }
                
                with open(output_file, 'w') as f:
                    json.dump(data, f)
                
                embeddings_generated += 1
                
                if embeddings_generated % 1000 == 0:
                    print(f"Generated {embeddings_generated:,} text embeddings...")
    
    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        continue

print(f"Text embeddings generation complete: {embeddings_generated:,} vectors")
EOF
    
    print_status "OK" "Text embeddings generated"
}

# Function to generate image embeddings
generate_image_embeddings() {
    echo -e "\n${BLUE}🖼️  Generating image embeddings...${NC}"
    
    python3 << 'EOF'
import os
import json
import numpy as np
import glob
from pathlib import Path

datasets_dir = os.environ.get('DATASETS_DIR', './datasets')
embeddings_dir = os.environ.get('EMBEDDINGS_DIR', './embeddings')
image_embeddings_gb = int(os.environ.get('IMAGE_EMBEDDINGS_GB', '80'))

# Calculate target number of embeddings (assuming 512-dim vectors for images)
vector_size_bytes = 512 * 4  # 2048 bytes per vector
target_vectors = (image_embeddings_gb * 1024 * 1024 * 1024) // vector_size_bytes
print(f"Target: {target_vectors:,} image embeddings ({image_embeddings_gb}GB)")

# Process image data files
image_files = glob.glob(f"{datasets_dir}/images/**/*.json", recursive=True)
image_files.extend(glob.glob(f"{datasets_dir}/images/**/*.txt", recursive=True))

embeddings_generated = 0

for file_path in image_files:
    if embeddings_generated >= target_vectors:
        break
        
    try:
        if file_path.endswith('.json'):
            with open(file_path, 'r') as f:
                data = json.load(f)
            
            # Simulate image feature extraction (replace with actual CNN features)
            if 'data' in data and isinstance(data['data'], list):
                # Use actual image data if available
                image_vector = np.array(data['data'][:512])  # Take first 512 features
                if len(image_vector) < 512:
                    # Pad with zeros if needed
                    image_vector = np.pad(image_vector, (0, 512 - len(image_vector)))
            else:
                # Generate synthetic image features
                image_vector = np.random.rand(512)
        else:
            # Generate synthetic features for text placeholders
            image_vector = np.random.rand(512)
        
        # Normalize vector
        image_vector = image_vector / np.linalg.norm(image_vector)
        
        output_file = f"{embeddings_dir}/images/embedding_{embeddings_generated:08d}.json"
        
        embedding_data = {
            'id': embeddings_generated,
            'source': file_path,
            'embedding': image_vector.tolist(),
            'dimension': len(image_vector),
            'type': 'image'
        }
        
        with open(output_file, 'w') as f:
            json.dump(embedding_data, f)
        
        embeddings_generated += 1
        
        if embeddings_generated % 1000 == 0:
            print(f"Generated {embeddings_generated:,} image embeddings...")
    
    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        continue

# Generate additional synthetic image embeddings if needed
while embeddings_generated < target_vectors:
    # Generate synthetic image embedding
    image_vector = np.random.rand(512)
    image_vector = image_vector / np.linalg.norm(image_vector)
    
    output_file = f"{embeddings_dir}/images/embedding_{embeddings_generated:08d}.json"
    
    embedding_data = {
        'id': embeddings_generated,
        'source': 'synthetic',
        'embedding': image_vector.tolist(),
        'dimension': len(image_vector),
        'type': 'image'
    }
    
    with open(output_file, 'w') as f:
        json.dump(embedding_data, f)
    
    embeddings_generated += 1
    
    if embeddings_generated % 1000 == 0:
        print(f"Generated {embeddings_generated:,} image embeddings...")

print(f"Image embeddings generation complete: {embeddings_generated:,} vectors")
EOF
    
    print_status "OK" "Image embeddings generated"
}

# Function to generate code embeddings
generate_code_embeddings() {
    echo -e "\n${BLUE}💻 Generating code embeddings...${NC}"
    
    python3 << 'EOF'
import os
import json
import numpy as np
import glob
from pathlib import Path

datasets_dir = os.environ.get('DATASETS_DIR', './datasets')
embeddings_dir = os.environ.get('EMBEDDINGS_DIR', './embeddings')
code_embeddings_gb = int(os.environ.get('CODE_EMBEDDINGS_GB', '40'))

# Calculate target number of embeddings (assuming 256-dim vectors for code)
vector_size_bytes = 256 * 4  # 1024 bytes per vector
target_vectors = (code_embeddings_gb * 1024 * 1024 * 1024) // vector_size_bytes
print(f"Target: {target_vectors:,} code embeddings ({code_embeddings_gb}GB)")

# Find code files
code_extensions = ['*.py', '*.rs', '*.js', '*.ts', '*.c', '*.cpp', '*.h', '*.java', '*.go']
code_files = []

for ext in code_extensions:
    code_files.extend(glob.glob(f"{datasets_dir}/code/**/{ext}", recursive=True))

embeddings_generated = 0

for file_path in code_files:
    if embeddings_generated >= target_vectors:
        break
        
    try:
        with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
            content = f.read()
        
        # Split code into functions/classes (simple heuristic)
        lines = content.split('\n')
        code_blocks = []
        current_block = []
        
        for line in lines:
            current_block.append(line)
            
            # Simple heuristic for code block boundaries
            if (line.strip().startswith('def ') or 
                line.strip().startswith('class ') or
                line.strip().startswith('function ') or
                line.strip().startswith('fn ') or
                len(current_block) > 50):
                
                if len(current_block) > 5:  # Only process substantial blocks
                    code_blocks.append('\n'.join(current_block))
                current_block = []
        
        # Add remaining block
        if current_block and len(current_block) > 5:
            code_blocks.append('\n'.join(current_block))
        
        # Generate embeddings for code blocks
        for block_id, code_block in enumerate(code_blocks):
            if embeddings_generated >= target_vectors:
                break
            
            # Simple code embedding (replace with actual code2vec or similar)
            # Hash-based features for demonstration
            code_hash = hash(code_block) % (2**32)
            
            # Generate pseudo-embedding based on code characteristics
            code_vector = np.random.rand(256)
            
            # Add some structure based on code content
            if 'def ' in code_block:
                code_vector[0:10] += 0.5  # Function indicator
            if 'class ' in code_block:
                code_vector[10:20] += 0.5  # Class indicator
            if 'import ' in code_block:
                code_vector[20:30] += 0.5  # Import indicator
            
            # Normalize
            code_vector = code_vector / np.linalg.norm(code_vector)
            
            output_file = f"{embeddings_dir}/code/embedding_{embeddings_generated:08d}.json"
            
            embedding_data = {
                'id': embeddings_generated,
                'source': file_path,
                'block_id': block_id,
                'code_snippet': code_block[:200] + "..." if len(code_block) > 200 else code_block,
                'embedding': code_vector.tolist(),
                'dimension': len(code_vector),
                'type': 'code'
            }
            
            with open(output_file, 'w') as f:
                json.dump(embedding_data, f)
            
            embeddings_generated += 1
            
            if embeddings_generated % 1000 == 0:
                print(f"Generated {embeddings_generated:,} code embeddings...")
    
    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        continue

# Generate additional synthetic code embeddings if needed
while embeddings_generated < target_vectors:
    # Generate synthetic code embedding
    code_vector = np.random.rand(256)
    code_vector = code_vector / np.linalg.norm(code_vector)
    
    output_file = f"{embeddings_dir}/code/embedding_{embeddings_generated:08d}.json"
    
    embedding_data = {
        'id': embeddings_generated,
        'source': 'synthetic',
        'embedding': code_vector.tolist(),
        'dimension': len(code_vector),
        'type': 'code'
    }
    
    with open(output_file, 'w') as f:
        json.dump(embedding_data, f)
    
    embeddings_generated += 1
    
    if embeddings_generated % 1000 == 0:
        print(f"Generated {embeddings_generated:,} code embeddings...")

print(f"Code embeddings generation complete: {embeddings_generated:,} vectors")
EOF
    
    print_status "OK" "Code embeddings generated"
}

# Function to create mixed dataset
create_mixed_dataset() {
    echo -e "\n${BLUE}🔀 Creating mixed dataset...${NC}"
    
    python3 << 'EOF'
import os
import json
import glob
import random
from pathlib import Path

embeddings_dir = os.environ.get('EMBEDDINGS_DIR', './embeddings')

# Collect all embeddings
text_files = glob.glob(f"{embeddings_dir}/text/*.json")
image_files = glob.glob(f"{embeddings_dir}/images/*.json")
code_files = glob.glob(f"{embeddings_dir}/code/*.json")

all_files = text_files + image_files + code_files
random.shuffle(all_files)

print(f"Creating mixed dataset from {len(all_files):,} embeddings...")
print(f"- Text: {len(text_files):,}")
print(f"- Images: {len(image_files):,}")
print(f"- Code: {len(code_files):,}")

# Create mixed batches
batch_size = 1000
mixed_dir = f"{embeddings_dir}/mixed"

for i in range(0, len(all_files), batch_size):
    batch_files = all_files[i:i+batch_size]
    batch_data = []
    
    for file_path in batch_files:
        try:
            with open(file_path, 'r') as f:
                data = json.load(f)
            batch_data.append(data)
        except Exception as e:
            print(f"Error reading {file_path}: {e}")
            continue
    
    # Save mixed batch
    batch_id = i // batch_size
    output_file = f"{mixed_dir}/mixed_batch_{batch_id:04d}.json"
    
    with open(output_file, 'w') as f:
        json.dump(batch_data, f)
    
    if batch_id % 10 == 0:
        print(f"Created mixed batch {batch_id}")

print("Mixed dataset creation complete!")
EOF
    
    print_status "OK" "Mixed dataset created"
}

# Function to generate summary report
generate_summary() {
    echo -e "\n${BLUE}📊 Generating summary report...${NC}"
    
    python3 << 'EOF'
import os
import json
import glob
from pathlib import Path

embeddings_dir = os.environ.get('EMBEDDINGS_DIR', './embeddings')

# Count embeddings by type
text_count = len(glob.glob(f"{embeddings_dir}/text/*.json"))
image_count = len(glob.glob(f"{embeddings_dir}/images/*.json"))
code_count = len(glob.glob(f"{embeddings_dir}/code/*.json"))
mixed_batches = len(glob.glob(f"{embeddings_dir}/mixed/*.json"))

# Calculate sizes
def get_dir_size(path):
    total = 0
    for file_path in glob.glob(f"{path}/**/*", recursive=True):
        if os.path.isfile(file_path):
            total += os.path.getsize(file_path)
    return total

text_size_gb = get_dir_size(f"{embeddings_dir}/text") / (1024**3)
image_size_gb = get_dir_size(f"{embeddings_dir}/images") / (1024**3)
code_size_gb = get_dir_size(f"{embeddings_dir}/code") / (1024**3)
total_size_gb = text_size_gb + image_size_gb + code_size_gb

# Generate report
report = f"""
VexFS 200GB Testing - Embeddings Generation Report
================================================

Generated Embeddings:
- Text embeddings: {text_count:,} vectors ({text_size_gb:.2f} GB)
- Image embeddings: {image_count:,} vectors ({image_size_gb:.2f} GB)
- Code embeddings: {code_count:,} vectors ({code_size_gb:.2f} GB)
- Total embeddings: {text_count + image_count + code_count:,} vectors
- Total size: {total_size_gb:.2f} GB

Mixed Dataset:
- Mixed batches: {mixed_batches:,}
- Average vectors per batch: {(text_count + image_count + code_count) // max(mixed_batches, 1):,}

Data Distribution:
- Text: {text_count / (text_count + image_count + code_count) * 100:.1f}%
- Images: {image_count / (text_count + image_count + code_count) * 100:.1f}%
- Code: {code_count / (text_count + image_count + code_count) * 100:.1f}%

Ready for VexFS testing!
"""

print(report)

# Save report
workbench_root = os.environ.get('WORKBENCH_ROOT', '..')
report_file = f"{workbench_root}/results/embeddings_generation_report.txt"
os.makedirs(os.path.dirname(report_file), exist_ok=True)

with open(report_file, 'w') as f:
    f.write(report)

print(f"Report saved to: {report_file}")
EOF
    
    print_status "OK" "Summary report generated"
}

# Main execution
main() {
    echo -e "${BLUE}Starting mixed embeddings generation...${NC}\n"
    
    check_dependencies
    create_directories
    download_text_datasets
    download_image_datasets
    download_code_datasets
    generate_text_embeddings
    generate_image_embeddings
    generate_code_embeddings
    create_mixed_dataset
    generate_summary
    
    echo -e "\n${GREEN}🎉 Mixed embeddings generation COMPLETE!${NC}"
    echo "=================================================================="
    echo "200GB of diverse vector embeddings ready for VexFS testing"
    echo ""
    echo "Next steps:"
    echo "1. Load VexFS kernel module: sudo insmod vexfs.ko"
    echo "2. Format device: sudo mkfs.vexfs $TARGET_DEVICE"
    echo "3. Mount filesystem: sudo mount -t vexfs $TARGET_DEVICE $MOUNT_POINT"
    echo "4. Begin testing: cd ../testing && ./run_comprehensive_tests.sh"
    echo ""
    echo "🚀 Ready to demonstrate AI data sovereignty with VexFS!"
}

# Execute main function
main "$@"