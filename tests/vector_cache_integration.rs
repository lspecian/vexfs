//! Vector Cache Integration Tests
//!
//! This module tests the vector caching system integration with VexFS.

use vexfs::vector_cache::{VectorCacheManager, VectorCacheConfig, EvictionPolicy, PrefetchStrategy, CoherenceMode};

use vexfs::vector_storage::{VectorHeader, VectorDataType, CompressionType};
use std::time::Instant;

#[test]
fn test_vector_cache_basic_operations() {
    let config = VectorCacheConfig {
        max_size: 1024 * 1024, // 1MB
        max_entries: 100,
        eviction_policy: EvictionPolicy::LRU,
        prefetch_strategy: PrefetchStrategy::None,
        coherence_mode: CoherenceMode::WriteThrough,
        enable_compression: false,
        memory_pressure_threshold: 0.8,
        prefetch_batch_size: 8,
        enable_cache_warming: false,
    };

    let mut cache = VectorCacheManager::new(config);

    // Test cache miss
    assert!(cache.get_vector(123).is_none());
    let stats = cache.get_stats();
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.hits, 0);

    // Insert a vector
    let header = VectorHeader {
        magic: 0x56455856,
        version: 1,
        vector_id: 123,
        file_inode: 456,
        data_type: VectorDataType::Float32,
        compression: CompressionType::None,
        dimensions: 128,
        original_size: 512,
        compressed_size: 512,
        created_timestamp: 0,
        modified_timestamp: 0,
        checksum: 0,
        flags: 0,
        reserved: [],
    };

    let data = vec![0u8; 512];
    cache.insert_vector(123, 456, header, data).unwrap();

    // Test cache hit
    assert!(cache.get_vector(123).is_some());
    let stats = cache.get_stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
}

#[test]
fn test_eviction_policies() {
    let policies = vec![
        EvictionPolicy::LRU,
        EvictionPolicy::LFU,
        EvictionPolicy::ARC,
        EvictionPolicy::ValueBased,
    ];

    for policy in policies {
        let config = VectorCacheConfig {
            max_size: 1024, // Small cache to force eviction
            max_entries: 2,
            eviction_policy: policy,
            prefetch_strategy: PrefetchStrategy::None,
            coherence_mode: CoherenceMode::WriteThrough,
            enable_compression: false,
            memory_pressure_threshold: 0.8,
            prefetch_batch_size: 8,
            enable_cache_warming: false,
        };

        let mut cache = VectorCacheManager::new(config);

        // Fill cache beyond capacity
        for i in 0..3 {
            let header = VectorHeader {
                magic: 0x56455856,
                version: 1,
                vector_id: i,
                file_inode: i + 100,
                data_type: VectorDataType::Float32,
                compression: CompressionType::None,
                dimensions: 32,
                original_size: 128,
                compressed_size: 128,
                created_timestamp: 0,
                modified_timestamp: 0,
                checksum: 0,
                flags: 0,
                reserved: [],
            };

            let data = vec![0u8; 128];
            cache.insert_vector(i, i + 100, header, data).unwrap();
        }

        // Should have evicted at least one entry
        let stats = cache.get_stats();
        assert!(stats.eviction_count > 0);
        
        println!("Policy {:?}: {} evictions", policy, stats.eviction_count);
    }
}

#[test]
fn test_cache_performance_comparison() {
    let num_vectors = 1000;
    let cache_size = 100;
    let num_operations = 500;

    // Test different eviction policies
    let policies = vec![
        EvictionPolicy::LRU,
        EvictionPolicy::LFU,
        EvictionPolicy::ARC,
        EvictionPolicy::ValueBased,
    ];

    for policy in policies {
        let config = VectorCacheConfig {
            max_size: cache_size * 1024,
            max_entries: cache_size,
            eviction_policy: policy,
            prefetch_strategy: PrefetchStrategy::None,
            coherence_mode: CoherenceMode::WriteThrough,
            enable_compression: false,
            memory_pressure_threshold: 0.8,
            prefetch_batch_size: 8,
            enable_cache_warming: false,
        };

        let mut cache = VectorCacheManager::new(config);

        // Pre-populate some vectors
        for i in 0..cache_size / 2 {
            let header = VectorHeader {
                magic: 0x56455856,
                version: 1,
                vector_id: i as u64,
                file_inode: i as u64 + 1000,
                data_type: VectorDataType::Float32,
                compression: CompressionType::None,
                dimensions: 128,
                original_size: 512,
                compressed_size: 512,
                created_timestamp: 0,
                modified_timestamp: 0,
                checksum: 0,
                flags: 0,
                reserved: [],
            };

            let data = vec![0u8; 512];
            cache.insert_vector(i as u64, i as u64 + 1000, header, data).unwrap();
        }

        // Simulate access pattern (80/20 rule)
        let start_time = Instant::now();
        let hot_set_size = cache_size / 5;

        for _ in 0..num_operations {
            let vector_id = if fastrand::f64() < 0.8 {
                // 80% chance to access hot set
                fastrand::u64(0..hot_set_size as u64)
            } else {
                // 20% chance to access cold set
                fastrand::u64(0..num_vectors as u64)
            };

            // Try cache access
            if cache.get_vector(vector_id).is_none() {
                // Simulate cache miss - insert new vector
                let header = VectorHeader {
                    magic: 0x56455856,
                    version: 1,
                    vector_id,
                    file_inode: vector_id + 1000,
                    data_type: VectorDataType::Float32,
                    compression: CompressionType::None,
                    dimensions: 128,
                    original_size: 512,
                    compressed_size: 512,
                    created_timestamp: 0,
                    modified_timestamp: 0,
                    checksum: 0,
                    flags: 0,
                    reserved: [],
                };

                let data = vec![0u8; 512];
                let _ = cache.insert_vector(vector_id, vector_id + 1000, header, data);
            }
        }

        let elapsed = start_time.elapsed();
        let stats = cache.get_stats();
        let hit_rate = stats.vector_hit_rate();

        println!(
            "Policy {:?}: Hit rate {:.1}%, Time {:?}, Evictions {}",
            policy,
            hit_rate * 100.0,
            elapsed,
            stats.eviction_count
        );

        // Basic performance assertions
        assert!(hit_rate > 0.0, "Cache should have some hits");
        assert!(elapsed.as_millis() < 1000, "Operations should complete quickly");
    }
}

#[test]
fn test_cache_memory_pressure() {
    let config = VectorCacheConfig {
        max_size: 2048, // Small cache
        max_entries: 10,
        eviction_policy: EvictionPolicy::ARC,
        prefetch_strategy: PrefetchStrategy::None,
        coherence_mode: CoherenceMode::WriteThrough,
        enable_compression: false,
        memory_pressure_threshold: 0.7, // Lower threshold
        prefetch_batch_size: 8,
        enable_cache_warming: false,
    };

    let mut cache = VectorCacheManager::new(config);

    // Fill cache to trigger memory pressure
    for i in 0..15 {
        let header = VectorHeader {
            magic: 0x56455856,
            version: 1,
            vector_id: i,
            file_inode: i + 100,
            data_type: VectorDataType::Float32,
            compression: CompressionType::None,
            dimensions: 64,
            original_size: 256,
            compressed_size: 256,
            created_timestamp: 0,
            modified_timestamp: 0,
            checksum: 0,
            flags: 0,
            reserved: [],
        };

        let data = vec![0u8; 256];
        cache.insert_vector(i, i + 100, header, data).unwrap();

        // Perform maintenance to trigger eviction
        cache.maintenance().unwrap();
    }

    let stats = cache.get_stats();
    println!("Memory pressure: {:.1}%", stats.memory_pressure * 100.0);
    println!("Evictions: {}", stats.eviction_count);

    // Should have triggered evictions due to memory pressure
    assert!(stats.eviction_count > 0);
    assert!(stats.memory_pressure <= 1.0);
}

#[test]
fn test_cache_invalidation() {
    let mut cache = VectorCacheManager::with_defaults();

    // Insert a vector
    let header = VectorHeader {
        magic: 0x56455856,
        version: 1,
        vector_id: 123,
        file_inode: 456,
        data_type: VectorDataType::Float32,
        compression: CompressionType::None,
        dimensions: 128,
        original_size: 512,
        compressed_size: 512,
        created_timestamp: 0,
        modified_timestamp: 0,
        checksum: 0,
        flags: 0,
        reserved: [],
    };

    let data = vec![0u8; 512];
    cache.insert_vector(123, 456, header, data).unwrap();

    // Verify it's cached
    assert!(cache.get_vector(123).is_some());

    // Invalidate the entry
    cache.invalidate_vector(123).unwrap();

    // Should be gone from cache
    assert!(cache.get_vector(123).is_none());
}

#[test]
fn test_cache_utilization() {
    let config = VectorCacheConfig {
        max_size: 1024,
        max_entries: 10,
        eviction_policy: EvictionPolicy::LRU,
        prefetch_strategy: PrefetchStrategy::None,
        coherence_mode: CoherenceMode::WriteThrough,
        enable_compression: false,
        memory_pressure_threshold: 0.8,
        prefetch_batch_size: 8,
        enable_cache_warming: false,
    };

    let mut cache = VectorCacheManager::new(config);

    // Initially empty
    assert_eq!(cache.get_utilization(), 0.0);

    // Add some vectors
    for i in 0..3 {
        let header = VectorHeader {
            magic: 0x56455856,
            version: 1,
            vector_id: i,
            file_inode: i + 100,
            data_type: VectorDataType::Float32,
            compression: CompressionType::None,
            dimensions: 32,
            original_size: 128,
            compressed_size: 128,
            created_timestamp: 0,
            modified_timestamp: 0,
            checksum: 0,
            flags: 0,
            reserved: [],
        };

        let data = vec![0u8; 128];
        cache.insert_vector(i, i + 100, header, data).unwrap();
    }

    // Should have some utilization
    let utilization = cache.get_utilization();
    assert!(utilization > 0.0);
    assert!(utilization <= 1.0);
    
    println!("Cache utilization: {:.1}%", utilization * 100.0);
}