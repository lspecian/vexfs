//! Compression Benchmark Tool for VexFS Vector Storage
//! 
//! This tool benchmarks different compression strategies for vector data,
//! measuring compression ratios, speed, and integration with the optimization framework.

use std::time::Instant;
use std::sync::Arc;

// Import from the current crate
use vexfs::vector_storage::{
    VectorStorageManager, VectorDataType, CompressionType, VectorCompressionStrategy,
    CompressionBenchmark, VectorCompression
};
use vexfs::vector_optimizations::{VectorOptimizer, SimdStrategy, MemoryLayout, BatchConfig};
use vexfs::storage::StorageManager;
use vexfs::fs_core::operations::OperationContext;
use vexfs::fs_core::locking::LockManager;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🗜️  VexFS Vector Compression Benchmark Suite");
    println!("============================================");
    
    // Initialize storage manager
    let storage_manager = Arc::new(StorageManager::new(4096, 1000000, None)?);
    let mut vector_storage = VectorStorageManager::new(storage_manager.clone(), 4096, 1000000);
    
    // Test different vector types and dimensions
    let test_cases = vec![
        (VectorDataType::Float32, 128, "Small Float32"),
        (VectorDataType::Float32, 256, "Medium Float32"),
        (VectorDataType::Float32, 512, "Large Float32"),
        (VectorDataType::Float32, 1024, "XLarge Float32"),
        (VectorDataType::Float16, 256, "Float16"),
        (VectorDataType::Int8, 256, "Int8"),
        (VectorDataType::Binary, 256, "Binary"),
    ];
    
    for (data_type, dimensions, description) in test_cases {
        println!("\n📊 Testing {description} (dims: {dimensions})");
        println!("{}", "=".repeat(50));
        
        // Generate test data
        let test_data = generate_test_vector(data_type, dimensions);
        
        // Benchmark compression strategies
        match vector_storage.benchmark_compression(&test_data, data_type) {
            Ok(benchmark) => {
                print_compression_results(&benchmark);
                
                // Test integration with optimization framework
                test_optimization_integration(&test_data, data_type, dimensions as u32);
            }
            Err(e) => {
                println!("❌ Benchmark failed: {:?}", e);
            }
        }
    }
    
    // Test adaptive compression selection
    println!("\n🎯 Testing Adaptive Compression Selection");
    println!("{}", "=".repeat(50));
    test_adaptive_compression(&mut vector_storage);
    
    // Performance impact analysis
    println!("\n⚡ Performance Impact Analysis");
    println!("{}", "=".repeat(50));
    test_performance_impact(&mut vector_storage, storage_manager);
    
    println!("\n✅ Compression benchmark suite completed!");
    Ok(())
}

fn generate_test_vector(data_type: VectorDataType, dimensions: usize) -> Vec<u8> {
    match data_type {
        VectorDataType::Float32 => {
            let mut data = Vec::new();
            for i in 0..dimensions {
                let val = match i % 4 {
                    0 => (i as f32 * 0.1).sin(),           // Smooth values
                    1 => if i % 10 == 0 { 1.0 } else { 0.0 }, // Sparse values
                    2 => (i as f32).sqrt() * 0.01,        // Small values
                    _ => i as f32 * 0.01,                  // Linear values
                };
                data.extend_from_slice(&val.to_le_bytes());
            }
            data
        }
        VectorDataType::Float16 => {
            let mut data = Vec::new();
            for i in 0..dimensions {
                let val = (i as f32 * 0.1).sin();
                // Convert to f16 representation (simplified)
                let f16_bits = (val * 32767.0) as i16;
                data.extend_from_slice(&f16_bits.to_le_bytes());
            }
            data
        }
        VectorDataType::Int8 => {
            (0..dimensions).map(|i| (i % 256) as u8).collect()
        }
        VectorDataType::Int16 => {
            let mut data = Vec::new();
            for i in 0..dimensions {
                let val = (i % 65536) as i16;
                data.extend_from_slice(&val.to_le_bytes());
            }
            data
        }
        VectorDataType::Binary => {
            (0..dimensions).map(|i| if i % 8 == 0 { 1 } else { 0 }).collect()
        }
    }
}

fn print_compression_results(benchmark: &CompressionBenchmark) {
    println!("Original size: {} bytes", benchmark.original_size);
    println!();
    
    // Print header
    println!("{:<20} {:<12} {:<8} {:<12} {:<12} {:<8}",
        "Strategy", "Compressed", "Ratio", "Comp Time", "Decomp Time", "Status");
    println!("{}", "-".repeat(80));
    
    // Print results
    for result in &benchmark.results {
        let status = if result.success { "✅" } else { "❌" };
        println!("{:<20} {:<12} {:<8.2} {:<12} {:<12} {:<8}",
            format!("{:?}", result.strategy),
            format!("{} bytes", result.compressed_size),
            result.compression_ratio,
            format!("{:.2}ms", result.compress_time.as_secs_f64() * 1000.0),
            format!("{:.2}ms", result.decompress_time.as_secs_f64() * 1000.0),
            status
        );
    }
    
    // Print recommendations
    println!();
    if let Some(best_ratio) = benchmark.best_by_ratio() {
        println!("🏆 Best compression ratio: {:?} ({:.2}x)",
            best_ratio.strategy, best_ratio.compression_ratio);
    }
    
    if let Some(fastest) = benchmark.fastest_compression() {
        println!("⚡ Fastest compression: {:?} ({:.2}ms)",
            fastest.strategy, fastest.compress_time.as_secs_f64() * 1000.0);
    }
    
    if let Some(balanced) = benchmark.balanced_strategy() {
        println!("⚖️  Balanced strategy: {:?} (ratio: {:.2}x, time: {:.2}ms)",
            balanced.strategy, balanced.compression_ratio,
            balanced.compress_time.as_secs_f64() * 1000.0);
    }
}

fn test_optimization_integration(data: &[u8], data_type: VectorDataType, dimensions: u32) {
    println!("\n🔧 Optimization Framework Integration:");
    
    // Test with different optimization strategies
    let configs = vec![
        (SimdStrategy::Scalar, MemoryLayout::ArrayOfStructures, "Scalar + AoS"),
        (SimdStrategy::Avx2, MemoryLayout::StructureOfArrays, "AVX2 + SoA"),
        (SimdStrategy::Auto, MemoryLayout::Hybrid, "Auto + Hybrid"),
    ];
    
    for (simd, layout, description) in configs {
        let batch_config = BatchConfig::default();
        let _optimizer = VectorOptimizer::with_config(simd, layout, batch_config);
        
        // Simulate compression with optimization
        let start = Instant::now();
        
        // Select compression strategy
        let compression = VectorCompressionStrategy::select_optimal(data, data_type, dimensions);
        
        // Apply optimization-aware compression (placeholder)
        let _optimized_time = start.elapsed();
        
        println!("  {} -> Compression: {:?}", description, compression);
    }
}

fn test_adaptive_compression(vector_storage: &mut VectorStorageManager) {
    let test_vectors = vec![
        (generate_sparse_vector(256), VectorDataType::Float32, "Sparse Vector"),
        (generate_dense_vector(256), VectorDataType::Float32, "Dense Vector"),
        (generate_correlated_vector(256), VectorDataType::Float32, "Correlated Vector"),
        (generate_random_vector(256), VectorDataType::Float32, "Random Vector"),
    ];
    
    for (data, data_type, description) in test_vectors {
        let selected = vector_storage.select_compression_strategy(&data, data_type, 256);
        println!("{}: {:?}", description, selected);
    }
}

fn test_performance_impact(vector_storage: &mut VectorStorageManager, storage_manager: Arc<StorageManager>) {
    let test_data = generate_test_vector(VectorDataType::Float32, 256);
    let strategies = [
        CompressionType::None,
        CompressionType::Quantization8Bit,
        CompressionType::ProductQuantization,
    ];
    
    println!("{:<20} {:<15} {:<15} {:<15}",
        "Strategy", "Storage (bytes)", "Insert (ms)", "Retrieve (ms)");
    println!("{}", "-".repeat(70));
    
    for &strategy in &strategies {
        // Mock context for testing
        let mut context = OperationContext::new(
            1000, // user_id
            storage_manager.clone(),
            Arc::new(LockManager::new()),
        );
        
        // Test insertion
        let insert_start = Instant::now();
        match vector_storage.store_vector(
            &mut context,
            &test_data,
            1, // file_inode
            VectorDataType::Float32,
            256,
            strategy,
        ) {
            Ok(vector_id) => {
                let insert_time = insert_start.elapsed();
                
                // Test retrieval
                let retrieve_start = Instant::now();
                match vector_storage.get_vector(&mut context, vector_id) {
                    Ok((header, _data)) => {
                        let retrieve_time = retrieve_start.elapsed();
                        
                        println!("{:<20} {:<15} {:<15.2} {:<15.2}",
                            format!("{:?}", strategy),
                            header.compressed_size,
                            insert_time.as_secs_f64() * 1000.0,
                            retrieve_time.as_secs_f64() * 1000.0
                        );
                    }
                    Err(e) => {
                        println!("{:<20} {:<15} {:<15} {:<15}",
                            format!("{:?}", strategy), "Error", "N/A", format!("Error: {:?}", e));
                    }
                }
            }
            Err(e) => {
                println!("{:<20} {:<15} {:<15} {:<15}",
                    format!("{:?}", strategy), "Error", format!("Error: {:?}", e), "N/A");
            }
        }
    }
}

fn generate_sparse_vector(dimensions: usize) -> Vec<u8> {
    let mut data = Vec::new();
    for i in 0..dimensions {
        let val = if i % 20 == 0 { 1.0f32 } else { 0.0f32 };
        data.extend_from_slice(&val.to_le_bytes());
    }
    data
}

fn generate_dense_vector(dimensions: usize) -> Vec<u8> {
    let mut data = Vec::new();
    for i in 0..dimensions {
        let val = (i as f32 * 0.1).sin() + 0.5;
        data.extend_from_slice(&val.to_le_bytes());
    }
    data
}

fn generate_correlated_vector(dimensions: usize) -> Vec<u8> {
    let mut data = Vec::new();
    let mut prev = 0.0f32;
    for _i in 0..dimensions {
        // Simple pseudo-random without external crate
        let val = prev + ((prev * 9.0 + 1.0) % 1.0 - 0.5) * 0.1;
        prev = val;
        data.extend_from_slice(&val.to_le_bytes());
    }
    data
}

fn generate_random_vector(dimensions: usize) -> Vec<u8> {
    let mut data = Vec::new();
    let mut seed = 12345u32;
    for _i in 0..dimensions {
        // Simple LCG for pseudo-random numbers
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let val = (seed as f32) / (u32::MAX as f32);
        data.extend_from_slice(&val.to_le_bytes());
    }
    data
}