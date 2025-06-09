//! Benchmarks for Cross-Layer Integration Framework (Task 21)
//! 
//! This benchmark suite measures the performance of the unified transaction management,
//! vector clock operations, journal ordering, and other critical components of
//! the Cross-Layer Integration Framework.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(feature = "cross_layer_integration")]
use vexfs::cross_layer_integration::{
    CrossLayerIntegrationFramework, IntegrationConfig,
    VectorClock, LamportTimestamp, JournalOrderingService,
    VersionedMetadataManager, TwoPhaseCommitCoordinator,
    RecoveryManager, PerformanceCache,
};

use vexfs::cross_layer_consistency::CrossLayerIsolationLevel;

#[cfg(feature = "cross_layer_integration")]
fn benchmark_framework_creation(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("framework_creation", |b| {
        b.to_async(&rt).iter(|| async {
            let config = IntegrationConfig::default();
            let framework = CrossLayerIntegrationFramework::new(config).await.unwrap();
            framework
        });
    });
}

#[cfg(feature = "cross_layer_integration")]
fn benchmark_transaction_lifecycle(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let framework = rt.block_on(async {
        let config = IntegrationConfig {
            max_concurrent_transactions: 1000,
            ..Default::default()
        };
        let framework = CrossLayerIntegrationFramework::new(config).await.unwrap();
        framework.start().await.unwrap();
        framework
    });
    
    c.bench_function("transaction_lifecycle", |b| {
        b.to_async(&rt).iter(|| async {
            let tx_id = framework.begin_unified_transaction(
                vec!["filesystem".to_string()],
                CrossLayerIsolationLevel::ReadCommitted,
                None,
            ).await.unwrap();
            
            framework.add_unified_operation(
                tx_id,
                "filesystem".to_string(),
                "write".to_string(),
                vec![1, 2, 3, 4],
                HashMap::new(),
            ).await.unwrap();
            
            framework.commit_unified_transaction(tx_id).await.unwrap();
        });
    });
    
    rt.block_on(async {
        framework.stop().await.unwrap();
    });
}

#[cfg(feature = "cross_layer_integration")]
fn benchmark_vector_clock_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_clock");
    
    // Benchmark vector clock tick operation
    group.bench_function("tick", |b| {
        let mut clock = VectorClock::new("node1".to_string());
        b.iter(|| {
            clock.tick();
        });
    });
    
    // Benchmark vector clock update operation
    group.bench_function("update", |b| {
        let mut clock1 = VectorClock::new("node1".to_string());
        let mut clock2 = VectorClock::new("node2".to_string());
        clock2.tick();
        
        b.iter(|| {
            clock1.update(&clock2);
        });
    });
    
    // Benchmark vector clock comparison operations
    group.bench_function("happens_before", |b| {
        let mut clock1 = VectorClock::new("node1".to_string());
        let mut clock2 = VectorClock::new("node2".to_string());
        clock1.tick();
        clock2.update(&clock1);
        
        b.iter(|| {
            clock1.happens_before(&clock2)
        });
    });
    
    group.bench_function("concurrent_with", |b| {
        let clock1 = VectorClock::new("node1".to_string());
        let clock2 = VectorClock::new("node2".to_string());
        
        b.iter(|| {
            clock1.concurrent_with(&clock2)
        });
    });
    
    group.finish();
}

#[cfg(feature = "cross_layer_integration")]
fn benchmark_lamport_timestamp_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("lamport_timestamp");
    
    group.bench_function("tick", |b| {
        let mut ts = LamportTimestamp::new(1);
        b.iter(|| {
            ts.tick();
        });
    });
    
    group.bench_function("update", |b| {
        let mut ts1 = LamportTimestamp::new(1);
        let mut ts2 = LamportTimestamp::new(2);
        ts2.tick();
        
        b.iter(|| {
            ts1.update(ts2);
        });
    });
    
    group.finish();
}

#[cfg(feature = "cross_layer_integration")]
fn benchmark_journal_ordering(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("journal_ordering");
    
    for batch_size in [1, 10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("add_entry", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let mut service = JournalOrderingService::new("node1".to_string(), batch_size);
                    
                    for i in 0..batch_size {
                        service.add_entry(
                            Uuid::new_v4(),
                            Uuid::new_v4(),
                            "filesystem".to_string(),
                            "write".to_string(),
                            vec![i as u8; 4],
                            HashMap::new(),
                        ).await.unwrap();
                    }
                });
            },
        );
    }
    
    group.finish();
}

#[cfg(feature = "cross_layer_integration")]
fn benchmark_versioned_metadata(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("versioned_metadata");
    
    group.bench_function("create_version", |b| {
        b.to_async(&rt).iter(|| async {
            let mut manager = VersionedMetadataManager::new();
            manager.create_version(Uuid::new_v4()).await.unwrap();
        });
    });
    
    group.bench_function("create_snapshot", |b| {
        b.to_async(&rt).iter(|| async {
            let mut manager = VersionedMetadataManager::new();
            manager.create_snapshot().await.unwrap();
        });
    });
    
    group.finish();
}

#[cfg(feature = "cross_layer_integration")]
fn benchmark_two_phase_commit(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("two_phase_commit");
    
    group.bench_function("prepare_commit_cycle", |b| {
        b.to_async(&rt).iter(|| async {
            let mut coordinator = TwoPhaseCommitCoordinator::new(
                "coordinator1".to_string(),
                Duration::from_secs(30),
            );
            
            let transaction_id = Uuid::new_v4();
            let transaction = vexfs::cross_layer_integration::TwoPhaseCommitTransaction {
                transaction_id,
                coordinator_id: "coordinator1".to_string(),
                state: vexfs::cross_layer_integration::TwoPhaseCommitState::Init,
                participants: vec!["fs".to_string(), "graph".to_string()],
                prepare_votes: HashMap::new(),
                timeout: Duration::from_secs(30),
                started_at: std::time::SystemTime::now(),
                prepared_at: None,
                committed_at: None,
                operations: Vec::new(),
            };
            
            coordinator.active_commits.insert(transaction_id, transaction);
            
            coordinator.prepare_transaction(transaction_id).await.unwrap();
            coordinator.commit_transaction(transaction_id).await.unwrap();
        });
    });
    
    group.finish();
}

#[cfg(feature = "cross_layer_integration")]
fn benchmark_performance_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_cache");
    
    group.bench_function("cache_query", |b| {
        let mut cache = PerformanceCache::new(1000);
        let query_result = vexfs::cross_layer_integration::QueryResult {
            rows: vec![HashMap::new()],
            execution_time: Duration::from_millis(50),
            layers_accessed: vec!["filesystem".to_string()],
        };
        
        b.iter(|| {
            let query_hash = format!("query_{}", fastrand::u64(..));
            cache.cache_query(query_hash, query_result.clone());
        });
    });
    
    group.bench_function("get_cached_query", |b| {
        let mut cache = PerformanceCache::new(1000);
        let query_result = vexfs::cross_layer_integration::QueryResult {
            rows: vec![HashMap::new()],
            execution_time: Duration::from_millis(50),
            layers_accessed: vec!["filesystem".to_string()],
        };
        
        // Pre-populate cache
        for i in 0..100 {
            let query_hash = format!("query_{}", i);
            cache.cache_query(query_hash, query_result.clone());
        }
        
        b.iter(|| {
            let query_hash = format!("query_{}", fastrand::u64(..100));
            cache.get_cached_query(&query_hash);
        });
    });
    
    group.finish();
}

#[cfg(feature = "cross_layer_integration")]
fn benchmark_concurrent_transactions(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_transactions");
    
    for concurrency in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_tx", concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async {
                    let config = IntegrationConfig {
                        max_concurrent_transactions: concurrency * 2,
                        ..Default::default()
                    };
                    let framework = std::sync::Arc::new(
                        CrossLayerIntegrationFramework::new(config).await.unwrap()
                    );
                    framework.start().await.unwrap();
                    
                    let mut handles = Vec::new();
                    
                    for i in 0..concurrency {
                        let framework_clone = std::sync::Arc::clone(&framework);
                        let handle = tokio::spawn(async move {
                            let tx_id = framework_clone.begin_unified_transaction(
                                vec!["filesystem".to_string()],
                                CrossLayerIsolationLevel::ReadCommitted,
                                None,
                            ).await.unwrap();
                            
                            framework_clone.add_unified_operation(
                                tx_id,
                                "filesystem".to_string(),
                                "write".to_string(),
                                vec![i as u8; 4],
                                HashMap::new(),
                            ).await.unwrap();
                            
                            framework_clone.commit_unified_transaction(tx_id).await.unwrap();
                        });
                        handles.push(handle);
                    }
                    
                    for handle in handles {
                        handle.await.unwrap();
                    }
                    
                    framework.stop().await.unwrap();
                });
            },
        );
    }
    
    group.finish();
}

#[cfg(feature = "cross_layer_integration")]
fn benchmark_recovery_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("recovery_operations");
    
    group.bench_function("check_recovery_needed", |b| {
        b.to_async(&rt).iter(|| async {
            let mut manager = RecoveryManager::new();
            manager.check_recovery_needed().await.unwrap();
        });
    });
    
    group.finish();
}

// Fallback benchmarks when cross_layer_integration feature is not enabled
#[cfg(not(feature = "cross_layer_integration"))]
fn benchmark_fallback(c: &mut Criterion) {
    c.bench_function("feature_not_enabled", |b| {
        b.iter(|| {
            // Simple benchmark that always runs
            let uuid = Uuid::new_v4();
            uuid != Uuid::nil()
        });
    });
}

#[cfg(feature = "cross_layer_integration")]
criterion_group!(
    benches,
    benchmark_framework_creation,
    benchmark_transaction_lifecycle,
    benchmark_vector_clock_operations,
    benchmark_lamport_timestamp_operations,
    benchmark_journal_ordering,
    benchmark_versioned_metadata,
    benchmark_two_phase_commit,
    benchmark_performance_cache,
    benchmark_concurrent_transactions,
    benchmark_recovery_operations
);

#[cfg(not(feature = "cross_layer_integration"))]
criterion_group!(benches, benchmark_fallback);

criterion_main!(benches);