# VexFS FUSE Stack Overflow Test Scenarios

This directory contains comprehensive test scenarios designed to reproduce and analyze stack overflow issues in the VexFS FUSE implementation, specifically targeting the VectorStorageManager and VectorSearchEngine initialization problems identified in Task 23.1.

## Test Scenario Categories

### 1. Large Vector Operations Test Scenarios
- **Purpose**: Test stack behavior with progressively larger vector datasets
- **Target Components**: VectorStorageManager, vector allocation, bulk operations
- **Files**: `large_vector_operations/`

### 2. Deep HNSW Graph Traversal Test Scenarios  
- **Purpose**: Test stack usage during HNSW graph construction and traversal
- **Target Components**: VectorSearchEngine, HNSW graph algorithms
- **Files**: `hnsw_graph_traversal/`

### 3. Component Initialization Test Scenarios
- **Purpose**: Isolate and test individual component initialization patterns
- **Target Components**: VectorStorageManager, VectorSearchEngine initialization
- **Files**: `component_initialization/`

### 4. Stress Test Scenarios
- **Purpose**: Test under extreme conditions and memory pressure
- **Target Components**: All VexFS components under stress
- **Files**: `stress_testing/`

### 5. Baseline Comparison Scenarios
- **Purpose**: Compare FUSE vs kernel module performance and stack usage
- **Target Components**: Cross-implementation comparison
- **Files**: `baseline_comparison/`

## Execution Framework

Each test scenario includes:
- **Parameterizable test configurations**
- **Profiling infrastructure integration**
- **Measurable stack usage data collection**
- **Reproducible execution scripts**
- **Expected outcomes and success criteria**
- **Both automated and manual execution support**

## Usage

1. **Setup**: Run the profiling environment setup first
2. **Individual Tests**: Execute specific scenario categories
3. **Full Suite**: Run all scenarios with comprehensive analysis
4. **Analysis**: Use collected data for stack optimization

See individual scenario directories for detailed execution instructions.