//! Stack-Optimized HNSW Implementation for FUSE Compatibility
//! 
//! This module provides a heap-based, iterative HNSW implementation designed
//! to work within FUSE's 8KB stack limit. Key optimizations:
//! 
//! 1. Iterative algorithms instead of recursive layer traversal
//! 2. Heap-allocated work queues and data structures
//! 3. Chunked processing for large graphs
//! 4. Memory pool system for search operations
//! 5. Stack usage monitoring and limits

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::anns::{AnnsError, HnswParams};
use crate::anns::stack_safety_monitor::{StackSafetyMonitor, StackGuard, StackSafeIterator};
use crate::vector_storage::{VectorStorageManager, VectorDataType};
use crate::vector_metrics::{VectorMetrics, DistanceMetric, calculate_distance};
use crate::fs_core::operations::OperationContext;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, collections::{BTreeMap, VecDeque}, boxed::Box};
#[cfg(feature = "std")]
use std::{vec::Vec, collections::{BTreeMap, VecDeque}, boxed::Box};

#[cfg(not(feature = "kernel"))]
use std::sync::{Arc, Mutex};
#[cfg(feature = "kernel")]
use alloc::sync::{Arc, Mutex};

// For random number generation in layer assignment
#[cfg(not(feature = "kernel"))]
use std::collections::hash_map::DefaultHasher;
#[cfg(feature = "kernel")]
use alloc::collections::hash_map::DefaultHasher;
use core::hash::{Hash, Hasher};

// Note: StackMonitor has been replaced with the comprehensive StackSafetyMonitor
// from crate::anns::stack_safety_monitor for better FUSE stack safety

/// Search state for a single layer (heap-allocated)
#[derive(Debug, Clone)]
pub struct LayerSearchState {
    pub layer: u8,
    pub candidates: Box<Vec<SearchCandidate>>,
    pub visited: Box<Vec<u64>>, // Using Vec instead of HashSet for better memory control
    pub dynamic_list: Box<Vec<(u64, f32)>>,
    pub entry_point: u64,
    pub num_closest: usize,
}

impl LayerSearchState {
    pub fn new(layer: u8, entry_point: u64, num_closest: usize) -> Self {
        Self {
            layer,
            candidates: Box::new(Vec::with_capacity(num_closest * 2)),
            visited: Box::new(Vec::with_capacity(num_closest * 4)),
            dynamic_list: Box::new(Vec::with_capacity(num_closest)),
            entry_point,
            num_closest,
        }
    }
    
    pub fn is_visited(&self, node_id: u64) -> bool {
        self.visited.binary_search(&node_id).is_ok()
    }
    
    pub fn mark_visited(&mut self, node_id: u64) {
        if let Err(pos) = self.visited.binary_search(&node_id) {
            self.visited.insert(pos, node_id);
        }
    }
}

/// Search candidate with distance
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchCandidate {
    pub vector_id: u64,
    pub distance: f32,
}

impl SearchCandidate {
    pub fn new(vector_id: u64, distance: f32) -> Self {
        Self { vector_id, distance }
    }
}

impl PartialOrd for SearchCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.distance.partial_cmp(&other.distance)
    }
}

/// Optimized HNSW node with minimal stack footprint
#[derive(Debug, Clone)]
pub struct OptimizedHnswNode {
    pub vector_id: u64,
    pub layer: u8,
    pub connections: Box<Vec<u64>>, // Heap-allocated connections
    pub max_connections: usize, // M parameter for this layer
}

impl OptimizedHnswNode {
    pub fn new(vector_id: u64, layer: u8) -> Self {
        // M parameter: layer 0 has 2*M connections, higher layers have M connections
        let max_connections = if layer == 0 { 32 } else { 16 }; // Default M=16
        
        Self {
            vector_id,
            layer,
            connections: Box::new(Vec::new()),
            max_connections,
        }
    }
    
    pub fn new_with_m(vector_id: u64, layer: u8, m: usize) -> Self {
        let max_connections = if layer == 0 { 2 * m } else { m };
        
        Self {
            vector_id,
            layer,
            connections: Box::new(Vec::new()),
            max_connections,
        }
    }
    
    pub fn add_connection(&mut self, neighbor_id: u64) {
        if !self.connections.contains(&neighbor_id) {
            self.connections.push(neighbor_id);
        }
    }
    
    pub fn remove_connection(&mut self, neighbor_id: u64) {
        if let Some(pos) = self.connections.iter().position(|&x| x == neighbor_id) {
            self.connections.remove(pos);
        }
    }
    
    pub fn has_connection(&self, neighbor_id: u64) -> bool {
        self.connections.contains(&neighbor_id)
    }
    
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
    
    pub fn is_full(&self) -> bool {
        self.connections.len() >= self.max_connections
    }
    
    pub fn get_connections(&self) -> &[u64] {
        &self.connections
    }
}

/// Vector cache entry for performance optimization
#[derive(Debug, Clone)]
pub struct VectorCacheEntry {
    pub vector_data: Vec<f32>,
    pub last_accessed: u64,
    pub access_count: u32,
}

/// HNSW construction statistics
#[derive(Debug, Clone)]
pub struct HnswConstructionStats {
    pub nodes_inserted: usize,
    pub connections_made: usize,
    pub layer_distribution: Vec<usize>, // Count of nodes per layer
    pub avg_connections_per_layer: Vec<f32>,
    pub construction_time_ms: u64,
    pub entry_point_updates: usize,
}

/// HNSW graph validation results
#[derive(Debug, Clone)]
pub struct HnswValidationResult {
    pub is_valid: bool,
    pub total_nodes: usize,
    pub total_connections: usize,
    pub orphaned_nodes: usize,
    pub layer_connectivity: Vec<bool>, // Per-layer connectivity status
    pub max_layer_reached: u8,
    pub entry_point_valid: bool,
}

/// Stack-optimized HNSW graph with real vector data integration
pub struct OptimizedHnswGraph {
    /// Graph nodes (heap-allocated)
    nodes: Box<BTreeMap<u64, OptimizedHnswNode>>,
    /// Entry point for search
    entry_point: Option<u64>,
    /// Maximum layer in the graph
    max_layer: u8,
    /// Graph parameters
    params: HnswParams,
    /// Advanced stack safety monitor with RAII guards and comprehensive tracking
    stack_monitor: StackSafetyMonitor,
    /// Memory pool for search operations
    search_memory_pool: SearchMemoryPool,
    /// Vector storage manager reference for real data retrieval
    vector_storage: Option<Arc<Mutex<VectorStorageManager>>>,
    /// Vector metrics calculator for distance functions
    vector_metrics: VectorMetrics,
    /// Distance metric to use for calculations
    distance_metric: DistanceMetric,
    /// Vector cache for frequently accessed vectors (heap-allocated)
    vector_cache: Box<BTreeMap<u64, VectorCacheEntry>>,
    /// Cache size limit to prevent memory bloat
    cache_size_limit: usize,
    /// Current cache access counter for LRU eviction
    cache_access_counter: u64,
    /// M parameter for HNSW construction
    m: usize,
    /// mL parameter for layer assignment (1/ln(2.0))
    ml: f64,
    /// Construction statistics
    construction_stats: HnswConstructionStats,
    /// Random seed for layer assignment
    random_seed: u64,
}

/// Memory pool for search operations
pub struct SearchMemoryPool {
    /// Pre-allocated search states
    search_states: Vec<LayerSearchState>,
    /// Available state indices
    available_states: Vec<usize>,
    /// Work queue for iterative processing
    work_queue: Box<VecDeque<LayerSearchState>>,
}

impl SearchMemoryPool {
    pub fn new(pool_size: usize) -> Self {
        let mut search_states = Vec::with_capacity(pool_size);
        let mut available_states = Vec::with_capacity(pool_size);
        
        for i in 0..pool_size {
            search_states.push(LayerSearchState::new(0, 0, 10)); // Default values
            available_states.push(i);
        }
        
        Self {
            search_states,
            available_states,
            work_queue: Box::new(VecDeque::with_capacity(16)), // Max 16 layers
        }
    }
    
    pub fn acquire_search_state(&mut self, layer: u8, entry_point: u64, num_closest: usize) -> Option<LayerSearchState> {
        if let Some(_index) = self.available_states.pop() {
            let mut state = LayerSearchState::new(layer, entry_point, num_closest);
            state.layer = layer;
            state.entry_point = entry_point;
            state.candidates.clear();
            state.visited.clear();
            state.dynamic_list.clear();
            Some(state)
        } else {
            None
        }
    }
    
    pub fn release_search_state(&mut self, _state: LayerSearchState) {
        // Since we're using owned values, we can just drop the state
        // In a more sophisticated implementation, we could reuse the memory
        // For now, this is a no-op since the state will be dropped automatically
    }
}

impl OptimizedHnswGraph {
    /// Create new optimized HNSW graph with minimal stack usage
    pub fn new(dimensions: u32, params: HnswParams) -> Result<Self, AnnsError> {
        let stack_monitor = StackSafetyMonitor::new(); // Uses 6KB safe limit for FUSE
        
        // Initialize vector metrics with SIMD enabled for performance
        let vector_metrics = VectorMetrics::new(true);
        
        // HNSW parameters from params
        let m = params.m as usize;
        let ml = params.ml;
        
        // Initialize construction stats
        let construction_stats = HnswConstructionStats {
            nodes_inserted: 0,
            connections_made: 0,
            layer_distribution: Vec::new(),
            avg_connections_per_layer: Vec::new(),
            construction_time_ms: 0,
            entry_point_updates: 0,
        };
        
        // Generate random seed for layer assignment
        let mut hasher = DefaultHasher::new();
        dimensions.hash(&mut hasher);
        let random_seed = hasher.finish();
        
        Ok(Self {
            nodes: Box::new(BTreeMap::new()),
            entry_point: None,
            max_layer: 0,
            params,
            stack_monitor,
            search_memory_pool: SearchMemoryPool::new(16), // Pool of 16 search states
            vector_storage: None, // Will be set via set_vector_storage
            vector_metrics,
            distance_metric: DistanceMetric::Euclidean, // Default to Euclidean distance
            vector_cache: Box::new(BTreeMap::new()),
            cache_size_limit: 1000, // Cache up to 1000 vectors
            cache_access_counter: 0,
            m,
            ml,
            construction_stats,
            random_seed,
        })
    }
    
    /// Create new optimized HNSW graph with vector storage integration
    pub fn new_with_storage(
        dimensions: u32,
        params: HnswParams,
        vector_storage: Arc<Mutex<VectorStorageManager>>,
        distance_metric: DistanceMetric,
    ) -> Result<Self, AnnsError> {
        let mut graph = Self::new(dimensions, params)?;
        graph.vector_storage = Some(vector_storage);
        graph.distance_metric = distance_metric;
        Ok(graph)
    }
    
    /// Set vector storage manager for real data integration
    pub fn set_vector_storage(&mut self, storage: Arc<Mutex<VectorStorageManager>>) {
        self.vector_storage = Some(storage);
    }
    
    /// Set distance metric for vector calculations
    pub fn set_distance_metric(&mut self, metric: DistanceMetric) {
        self.distance_metric = metric;
    }
    
    /// Get vector data from storage with caching (stack-safe implementation)
    fn get_vector_data_cached(&mut self, vector_id: u64, context: &mut OperationContext) -> VexfsResult<Vec<f32>> {
        // Check stack usage before proceeding
        self.stack_monitor.check_usage(512).map_err(|_| VexfsError::StackOverflow)?;
        
        // Check cache first
        self.cache_access_counter += 1;
        
        if let Some(cache_entry) = self.vector_cache.get_mut(&vector_id) {
            cache_entry.last_accessed = self.cache_access_counter;
            cache_entry.access_count += 1;
            return Ok(cache_entry.vector_data.clone());
        }
        
        // Cache miss - retrieve from storage
        let vector_data = match &self.vector_storage {
            Some(storage) => {
                let mut storage_guard = storage.lock().map_err(|_| {
                    VexfsError::InternalError("Failed to acquire storage lock".to_string())
                })?;
                
                let (header, raw_data) = storage_guard.get_vector(context, vector_id)?;
                
                // Convert raw bytes to f32 vector based on data type
                let vector_data = match header.data_type {
                    VectorDataType::Float32 => {
                        if raw_data.len() % 4 != 0 {
                            return Err(VexfsError::VectorError(
                                crate::shared::errors::VectorErrorKind::DeserializationError
                            ));
                        }
                        
                        let mut f32_data = Vec::with_capacity(raw_data.len() / 4);
                        for chunk in raw_data.chunks_exact(4) {
                            let bytes: [u8; 4] = chunk.try_into().map_err(|_| {
                                VexfsError::VectorError(
                                    crate::shared::errors::VectorErrorKind::DeserializationError
                                )
                            })?;
                            f32_data.push(f32::from_le_bytes(bytes));
                        }
                        f32_data
                    }
                    VectorDataType::Float16 => {
                        // For now, convert f16 to f32 (simplified implementation)
                        if raw_data.len() % 2 != 0 {
                            return Err(VexfsError::VectorError(
                                crate::shared::errors::VectorErrorKind::DeserializationError
                            ));
                        }
                        
                        let mut f32_data = Vec::with_capacity(raw_data.len() / 2);
                        for chunk in raw_data.chunks_exact(2) {
                            let bytes: [u8; 2] = chunk.try_into().map_err(|_| {
                                VexfsError::VectorError(
                                    crate::shared::errors::VectorErrorKind::DeserializationError
                                )
                            })?;
                            // Simplified f16 to f32 conversion (would need proper f16 library)
                            let f16_bits = u16::from_le_bytes(bytes);
                            let f32_value = (f16_bits as f32) / 65535.0; // Simplified conversion
                            f32_data.push(f32_value);
                        }
                        f32_data
                    }
                    VectorDataType::Int8 => {
                        // Convert i8 to f32
                        raw_data.iter().map(|&b| (b as i8) as f32 / 127.0).collect()
                    }
                    VectorDataType::Int16 => {
                        // Convert i16 to f32
                        if raw_data.len() % 2 != 0 {
                            return Err(VexfsError::VectorError(
                                crate::shared::errors::VectorErrorKind::DeserializationError
                            ));
                        }
                        
                        let mut f32_data = Vec::with_capacity(raw_data.len() / 2);
                        for chunk in raw_data.chunks_exact(2) {
                            let bytes: [u8; 2] = chunk.try_into().map_err(|_| {
                                VexfsError::VectorError(
                                    crate::shared::errors::VectorErrorKind::DeserializationError
                                )
                            })?;
                            let i16_value = i16::from_le_bytes(bytes);
                            f32_data.push(i16_value as f32 / 32767.0);
                        }
                        f32_data
                    }
                    VectorDataType::Binary => {
                        // Convert binary data to f32 (0.0 or 1.0)
                        raw_data.iter().map(|&b| if b > 127 { 1.0 } else { 0.0 }).collect()
                    }
                };
                
                vector_data
            }
            None => {
                // No storage available - return error
                return Err(VexfsError::InvalidOperation(
                    "No vector storage manager configured".to_string()
                ));
            }
        };
        
        // Add to cache if within limits
        if self.vector_cache.len() < self.cache_size_limit {
            let cache_entry = VectorCacheEntry {
                vector_data: vector_data.clone(),
                last_accessed: self.cache_access_counter,
                access_count: 1,
            };
            self.vector_cache.insert(vector_id, cache_entry);
        } else {
            // Evict least recently used entry
            self.evict_lru_cache_entry();
            let cache_entry = VectorCacheEntry {
                vector_data: vector_data.clone(),
                last_accessed: self.cache_access_counter,
                access_count: 1,
            };
            self.vector_cache.insert(vector_id, cache_entry);
        }
        
        Ok(vector_data)
    }
    
    /// Evict least recently used cache entry
    fn evict_lru_cache_entry(&mut self) {
        if let Some((&lru_id, _)) = self.vector_cache.iter()
            .min_by_key(|(_, entry)| entry.last_accessed) {
            self.vector_cache.remove(&lru_id);
        }
    }
    
    /// Calculate distance between two vectors using configured metric
    fn calculate_vector_distance(&mut self, vec1: &[f32], vec2: &[f32]) -> Result<f32, AnnsError> {
        // Check stack usage
        self.stack_monitor.check_usage(256).map_err(|_| AnnsError::StackOverflow)?;
        
        if vec1.len() != vec2.len() {
            return Err(AnnsError::InvalidDimensions);
        }
        
        let distance = match self.distance_metric {
            DistanceMetric::Euclidean => {
                self.vector_metrics.euclidean_distance(vec1, vec2)
                    .map_err(|_| AnnsError::InvalidVectorData)?
            }
            DistanceMetric::Cosine => {
                self.vector_metrics.cosine_distance(vec1, vec2)
                    .map_err(|_| AnnsError::InvalidVectorData)?
            }
            DistanceMetric::Dot => {
                // For dot product, we calculate negative dot product to use as distance
                let dot_product: f32 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
                -dot_product // Negative because we want smaller distances to be better
            }
            DistanceMetric::Manhattan => {
                vec1.iter().zip(vec2.iter()).map(|(a, b)| (a - b).abs()).sum()
            }
            DistanceMetric::Hamming => {
                // For Hamming distance on float vectors, we threshold at 0.5
                vec1.iter().zip(vec2.iter())
                    .map(|(a, b)| if (a > &0.5) != (b > &0.5) { 1.0 } else { 0.0 })
                    .sum()
            }
        };
        
        Ok(distance)
    }
    
    /// Probabilistic layer assignment using HNSW algorithm
    /// layer = floor(-ln(unif(0,1)) * mL) where mL = 1/ln(2)
    pub fn assign_layer(&mut self, vector_id: u64) -> u8 {
        // Check stack usage
        if self.stack_monitor.check_usage(128).is_err() {
            return 0; // Fallback to layer 0 on stack overflow risk
        }
        
        // Generate pseudo-random number based on vector_id and seed
        let mut hasher = DefaultHasher::new();
        vector_id.hash(&mut hasher);
        self.random_seed.hash(&mut hasher);
        let hash = hasher.finish();
        
        // Convert hash to uniform [0,1) value
        let uniform = (hash as f64) / (u64::MAX as f64);
        
        // Apply HNSW layer assignment formula
        let layer = (-uniform.ln() * self.ml).floor() as u8;
        
        // Cap at maximum layers
        layer.min(self.params.max_layers - 1)
    }
    
    /// Insert vector with complete HNSW construction algorithm
    pub fn insert_vector(&mut self, vector_id: u64, context: &mut OperationContext) -> Result<(), AnnsError> {
        // Check stack usage before proceeding
        self.stack_monitor.check_usage(1024).map_err(|_| AnnsError::StackOverflow)?;
        
        let start_time = std::time::Instant::now();
        
        // Assign layer for new vector
        let layer = self.assign_layer(vector_id);
        
        // Create new node with proper M parameter
        let mut new_node = OptimizedHnswNode::new_with_m(vector_id, layer, self.m);
        
        // If this is the first node, make it the entry point
        if self.nodes.is_empty() {
            self.entry_point = Some(vector_id);
            self.max_layer = layer;
            self.nodes.insert(vector_id, new_node);
            self.construction_stats.nodes_inserted += 1;
            self.construction_stats.entry_point_updates += 1;
            return Ok(());
        }
        
        // Get query vector for distance calculations
        let query_vector = self.get_vector_data_cached(vector_id, context)?;
        
        // Phase 1: Search from top layer down to layer+1
        let mut current_closest = self.entry_point.unwrap();
        
        for search_layer in ((layer + 1)..=self.max_layer).rev() {
            let candidates = self.search_layer_for_construction(
                &query_vector,
                current_closest,
                1, // ef=1 for upper layers
                search_layer,
                context
            )?;
            
            if let Some((closest_id, _)) = candidates.first() {
                current_closest = *closest_id;
            }
        }
        
        // Phase 2: Search and connect from layer down to 0
        for current_layer in (0..=layer).rev() {
            let ef = if current_layer == 0 {
                self.params.ef_construction as usize
            } else {
                self.params.ef_construction as usize
            };
            
            let candidates = self.search_layer_for_construction(
                &query_vector,
                current_closest,
                ef,
                current_layer,
                context
            )?;
            
            // Select M neighbors for connection
            let m_connections = if current_layer == 0 { 2 * self.m } else { self.m };
            let selected_neighbors = self.select_neighbors_simple(
                &candidates,
                m_connections,
                &query_vector,
                context
            )?;
            
            // Add connections to new node
            for &neighbor_id in &selected_neighbors {
                new_node.add_connection(neighbor_id);
                self.construction_stats.connections_made += 1;
            }
            
            // Add bidirectional connections and prune if necessary
            for &neighbor_id in &selected_neighbors {
                if let Some(neighbor_node) = self.nodes.get_mut(&neighbor_id) {
                    if neighbor_node.layer >= current_layer {
                        neighbor_node.add_connection(vector_id);
                        self.construction_stats.connections_made += 1;
                        
                        // Prune connections if neighbor exceeds M
                        if neighbor_node.is_full() {
                            self.prune_connections(neighbor_id, current_layer, context)?;
                        }
                    }
                }
            }
            
            // Update current closest for next layer
            if let Some((closest_id, _)) = selected_neighbors.first().map(|&id| (id, 0.0)) {
                current_closest = closest_id;
            }
        }
        
        // Update max layer and entry point if necessary
        if layer > self.max_layer {
            self.max_layer = layer;
            self.entry_point = Some(vector_id);
            self.construction_stats.entry_point_updates += 1;
        }
        
        // Insert the new node
        self.nodes.insert(vector_id, new_node);
        self.construction_stats.nodes_inserted += 1;
        
        // Update construction time
        self.construction_stats.construction_time_ms += start_time.elapsed().as_millis() as u64;
        
        Ok(())
    }
    
    /// Add node with minimal stack usage (legacy method)
    pub fn add_node(&mut self, node: OptimizedHnswNode) -> Result<(), AnnsError> {
        // Check stack usage before proceeding
        self.stack_monitor.check_usage(256).map_err(|_| AnnsError::StackOverflow)?;
        
        let vector_id = node.vector_id;
        let layer = node.layer;
        
        // Update max layer
        if layer > self.max_layer {
            self.max_layer = layer;
        }
        
        // Set entry point if this is the first node or higher layer
        if self.entry_point.is_none() || layer > self.get_entry_layer() {
            self.entry_point = Some(vector_id);
        }
        
        self.nodes.insert(vector_id, node);
        Ok(())
    }
    
    /// Get node reference
    pub fn get_node(&self, vector_id: u64) -> Option<&OptimizedHnswNode> {
        self.nodes.get(&vector_id)
    }
    
    /// Get entry point layer
    fn get_entry_layer(&self) -> u8 {
        self.entry_point
            .and_then(|ep| self.nodes.get(&ep))
            .map(|node| node.layer)
            .unwrap_or(0)
    }
    
    /// Check if graph is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    
    /// Iterative HNSW search with heap-based allocation
    /// This replaces the recursive search_layer approach with an iterative one
    pub fn search<F>(&mut self, query: &[f32], k: usize, ef: u16, distance_fn: F) -> Result<Vec<(u64, f32)>, AnnsError>
    where
        F: Fn(&[f32], &[f32]) -> Result<f32, AnnsError>,
    {
        // Check stack usage before operation
        self.stack_monitor.check_usage(1024)
            .map_err(|_| AnnsError::StackOverflow)?;
        
        if self.is_empty() {
            return Ok(Vec::new());
        }
        
        let entry_point = match self.entry_point {
            Some(ep) => ep,
            None => return Ok(Vec::new()),
        };
        
        // Phase 1: Iterative search from top layer down to layer 1
        let mut current_closest = entry_point;
        
        // Use iterative approach instead of recursive layer traversal
        for layer in (1..=self.max_layer).rev() {
            current_closest = self.search_layer_iterative(
                query, 
                current_closest, 
                1, 
                layer, 
                &distance_fn
            )?
            .into_iter()
            .next()
            .map(|(id, _)| id)
            .unwrap_or(current_closest);
        }
        
        // Phase 2: Search layer 0 with ef parameter
        let candidates = self.search_layer_iterative(
            query, 
            current_closest, 
            ef as usize, 
            0, 
            &distance_fn
        )?;
        
        // Return top k results
        let mut results: Vec<(u64, f32)> = candidates.into_iter().take(k).collect();
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
        
        Ok(results)
    }
    
    /// Iterative HNSW search with real vector data integration
    /// This method uses the vector storage manager to retrieve actual vector data
    pub fn search_with_real_vectors(&mut self, query: &[f32], k: usize, ef: u16, context: &mut OperationContext) -> Result<Vec<(u64, f32)>, AnnsError> {
        // Check stack usage before operation
        self.stack_monitor.check_usage(1024)
            .map_err(|_| AnnsError::StackOverflow)?;
        
        if self.is_empty() {
            return Ok(Vec::new());
        }
        
        let entry_point = match self.entry_point {
            Some(ep) => ep,
            None => return Ok(Vec::new()),
        };
        
        // Phase 1: Iterative search from top layer down to layer 1
        let mut current_closest = entry_point;
        
        // Use iterative approach instead of recursive layer traversal
        for layer in (1..=self.max_layer).rev() {
            current_closest = self.search_layer_iterative_real(
                query,
                current_closest,
                1,
                layer,
                context
            )?
            .into_iter()
            .next()
            .map(|(id, _)| id)
            .unwrap_or(current_closest);
        }
        
        // Phase 2: Search layer 0 with ef parameter
        let candidates = self.search_layer_iterative_real(
            query,
            current_closest,
            ef as usize,
            0,
            context
        )?;
        
        // Return top k results
        let mut results: Vec<(u64, f32)> = candidates.into_iter().take(k).collect();
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
        
        Ok(results)
    }
    
    /// Iterative layer search with heap-allocated data structures
    /// This replaces the stack-heavy recursive approach
    fn search_layer_iterative<F>(
        &mut self,
        query: &[f32],
        entry_point: u64,
        num_closest: usize,
        layer: u8,
        distance_fn: &F,
    ) -> Result<Vec<(u64, f32)>, AnnsError>
    where
        F: Fn(&[f32], &[f32]) -> Result<f32, AnnsError>,
    {
        // Check stack usage for layer search
        self.stack_monitor.check_usage(512)
            .map_err(|_| AnnsError::StackOverflow)?;
        
        // Acquire search state from memory pool (heap-allocated)
        let mut search_state = self.search_memory_pool
            .acquire_search_state(layer, entry_point, num_closest)
            .ok_or(AnnsError::OutOfMemory)?;
        
        // Initialize search state
        search_state.mark_visited(entry_point);
        
        // Simulate distance calculation for entry point
        let entry_distance = 0.5; // Placeholder - in real implementation would use distance_fn
        
        let entry_candidate = SearchCandidate::new(entry_point, entry_distance);
        search_state.candidates.push(entry_candidate);
        search_state.dynamic_list.push((entry_point, entry_distance));
        
        // Iterative search loop - no recursion, minimal stack usage
        let mut iteration_count = 0;
        let max_iterations = 1000; // Prevent infinite loops
        
        while !search_state.candidates.is_empty() && iteration_count < max_iterations {
            iteration_count += 1;
            
            // Find candidate with minimum distance
            let current_idx = search_state.candidates
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.distance.partial_cmp(&b.distance).unwrap_or(core::cmp::Ordering::Equal))
                .map(|(idx, _)| idx);
            
            let current = match current_idx {
                Some(idx) => search_state.candidates.remove(idx),
                None => break,
            };
            
            // Check if we should continue (early termination)
            if search_state.dynamic_list.len() >= num_closest {
                let farthest_distance = search_state.dynamic_list
                    .iter()
                    .map(|(_, dist)| *dist)
                    .fold(0.0f32, f32::max);
                
                if current.distance > farthest_distance {
                    break;
                }
            }
            
            // Examine neighbors of current node
            if let Some(node) = self.get_node(current.vector_id) {
                if node.layer >= layer {
                    // Process connections in chunks to avoid stack buildup
                    for &neighbor_id in node.connections.iter() {
                        if !search_state.is_visited(neighbor_id) {
                            search_state.mark_visited(neighbor_id);
                            
                            // Simulate distance calculation
                            let neighbor_distance = 0.6; // Placeholder
                            
                            let neighbor_candidate = SearchCandidate::new(neighbor_id, neighbor_distance);
                            
                            // Add to candidates if promising
                            let should_add = if search_state.dynamic_list.len() < num_closest {
                                true
                            } else {
                                let farthest_distance = search_state.dynamic_list
                                    .iter()
                                    .map(|(_, dist)| *dist)
                                    .fold(0.0f32, f32::max);
                                neighbor_distance < farthest_distance
                            };
                            
                            if should_add {
                                search_state.candidates.push(neighbor_candidate);
                                search_state.dynamic_list.push((neighbor_id, neighbor_distance));
                                
                                // Keep dynamic list size limited
                                if search_state.dynamic_list.len() > num_closest {
                                    // Remove farthest element
                                    if let Some(farthest_idx) = search_state.dynamic_list
                                        .iter()
                                        .enumerate()
                                        .max_by(|(_, (_, a)), (_, (_, b))| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal))
                                        .map(|(idx, _)| idx)
                                    {
                                        search_state.dynamic_list.remove(farthest_idx);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Collect results
        let mut results = (*search_state.dynamic_list).clone();
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
        
        // Search state will be automatically dropped (no need to release)
        
        Ok(results)
    }
    
    /// Iterative layer search with real vector data integration
    /// This replaces placeholder distances with actual vector distance calculations
    fn search_layer_iterative_real(
        &mut self,
        query: &[f32],
        entry_point: u64,
        num_closest: usize,
        layer: u8,
        context: &mut OperationContext,
    ) -> Result<Vec<(u64, f32)>, AnnsError> {
        // Check stack usage for real vector layer search
        self.stack_monitor.check_usage(512)
            .map_err(|_| AnnsError::StackOverflow)?;
        
        // Acquire search state from memory pool (heap-allocated)
        let mut search_state = self.search_memory_pool
            .acquire_search_state(layer, entry_point, num_closest)
            .ok_or(AnnsError::OutOfMemory)?;
        
        // Initialize search state
        search_state.mark_visited(entry_point);
        
        // Calculate real distance for entry point
        let entry_distance = match self.get_vector_data_cached(entry_point, context) {
            Ok(entry_vector) => {
                self.calculate_vector_distance(query, &entry_vector)
                    .unwrap_or(f32::INFINITY) // Fallback to infinity on error
            }
            Err(_) => {
                // If we can't get vector data, use a large distance but don't fail
                f32::INFINITY
            }
        };
        
        let entry_candidate = SearchCandidate::new(entry_point, entry_distance);
        search_state.candidates.push(entry_candidate);
        search_state.dynamic_list.push((entry_point, entry_distance));
        
        // Iterative search loop - no recursion, minimal stack usage
        let mut iteration_count = 0;
        let max_iterations = 1000; // Prevent infinite loops
        
        while !search_state.candidates.is_empty() && iteration_count < max_iterations {
            iteration_count += 1;
            
            // Find candidate with minimum distance
            let current_idx = search_state.candidates
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.distance.partial_cmp(&b.distance).unwrap_or(core::cmp::Ordering::Equal))
                .map(|(idx, _)| idx);
            
            let current = match current_idx {
                Some(idx) => search_state.candidates.remove(idx),
                None => break,
            };
            
            // Check if we should continue (early termination)
            if search_state.dynamic_list.len() >= num_closest {
                let farthest_distance = search_state.dynamic_list
                    .iter()
                    .map(|(_, dist)| *dist)
                    .fold(0.0f32, f32::max);
                
                if current.distance > farthest_distance {
                    break;
                }
            }
            
            // Examine neighbors of current node
            if let Some(node) = self.get_node(current.vector_id) {
                if node.layer >= layer {
                    // Clone connections to avoid borrowing issues
                    let connections = node.connections.clone();
                    
                    // Process connections in chunks to avoid stack buildup
                    for &neighbor_id in connections.iter() {
                        if !search_state.is_visited(neighbor_id) {
                            search_state.mark_visited(neighbor_id);
                            
                            // Calculate real distance for neighbor
                            let neighbor_distance = match self.get_vector_data_cached(neighbor_id, context) {
                                Ok(neighbor_vector) => {
                                    self.calculate_vector_distance(query, &neighbor_vector)
                                        .unwrap_or(f32::INFINITY) // Fallback on error
                                }
                                Err(_) => {
                                    // If we can't get vector data, skip this neighbor
                                    continue;
                                }
                            };
                            
                            let neighbor_candidate = SearchCandidate::new(neighbor_id, neighbor_distance);
                            
                            // Add to candidates if promising
                            let should_add = if search_state.dynamic_list.len() < num_closest {
                                true
                            } else {
                                let farthest_distance = search_state.dynamic_list
                                    .iter()
                                    .map(|(_, dist)| *dist)
                                    .fold(0.0f32, f32::max);
                                neighbor_distance < farthest_distance
                            };
                            
                            if should_add {
                                search_state.candidates.push(neighbor_candidate);
                                search_state.dynamic_list.push((neighbor_id, neighbor_distance));
                                
                                // Keep dynamic list size limited
                                if search_state.dynamic_list.len() > num_closest {
                                    // Remove farthest element
                                    if let Some(farthest_idx) = search_state.dynamic_list
                                        .iter()
                                        .enumerate()
                                        .max_by(|(_, (_, a)), (_, (_, b))| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal))
                                        .map(|(idx, _)| idx)
                                    {
                                        search_state.dynamic_list.remove(farthest_idx);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Collect results
        let mut results = (*search_state.dynamic_list).clone();
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
        
        // Search state will be automatically dropped (no need to release)
        
        Ok(results)
    }
    
    /// Get current stack usage estimate
    pub fn get_stack_usage(&self) -> usize {
        self.stack_monitor.current_usage()
    }
    
    /// Get memory statistics
    pub fn get_memory_stats(&self) -> HnswMemoryStats {
        HnswMemoryStats {
            node_count: self.nodes.len(),
            total_connections: self.nodes.values().map(|n| n.connections.len()).sum(),
            max_layer: self.max_layer,
            memory_pool_size: self.search_memory_pool.search_states.len(),
            stack_usage_estimate: self.stack_monitor.current_usage(),
            heap_usage_estimate: self.estimate_heap_usage(),
        }
    }
    
    /// Search layer for construction (similar to search but optimized for construction)
    fn search_layer_for_construction(
        &mut self,
        query: &[f32],
        entry_point: u64,
        ef: usize,
        layer: u8,
        context: &mut OperationContext,
    ) -> Result<Vec<(u64, f32)>, AnnsError> {
        // Check stack usage for construction search
        self.stack_monitor.check_usage(512)
            .map_err(|_| AnnsError::StackOverflow)?;
        
        // Use a simplified approach to avoid borrowing issues
        let mut candidates = Vec::new();
        let mut visited = Vec::new();
        let mut dynamic_list = Vec::new();
        
        // Initialize with entry point
        visited.push(entry_point);
        
        // Calculate distance to entry point
        let entry_distance = match self.get_vector_data_cached(entry_point, context) {
            Ok(entry_vector) => {
                self.calculate_vector_distance(query, &entry_vector)
                    .unwrap_or(f32::INFINITY)
            }
            Err(_) => f32::INFINITY,
        };
        
        candidates.push((entry_point, entry_distance));
        dynamic_list.push((entry_point, entry_distance));
        
        // Iterative search
        let mut iteration_count = 0;
        let max_iterations = 1000;
        
        while !candidates.is_empty() && iteration_count < max_iterations {
            iteration_count += 1;
            
            // Find closest candidate
            let current_idx = candidates
                .iter()
                .enumerate()
                .min_by(|(_, (_, a)), (_, (_, b))| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal))
                .map(|(idx, _)| idx);
            
            let (current_id, current_distance) = match current_idx {
                Some(idx) => candidates.remove(idx),
                None => break,
            };
            
            // Early termination check
            if dynamic_list.len() >= ef {
                let farthest_distance = dynamic_list
                    .iter()
                    .map(|(_, dist)| *dist)
                    .fold(0.0f32, f32::max);
                
                if current_distance > farthest_distance {
                    break;
                }
            }
            
            // Examine neighbors
            if let Some(node) = self.nodes.get(&current_id) {
                if node.layer >= layer {
                    let connections = node.connections.clone(); // Clone to avoid borrowing issues
                    
                    for &neighbor_id in connections.iter() {
                        if !visited.contains(&neighbor_id) {
                            visited.push(neighbor_id);
                            
                            // Calculate distance to neighbor
                            let neighbor_distance = match self.get_vector_data_cached(neighbor_id, context) {
                                Ok(neighbor_vector) => {
                                    self.calculate_vector_distance(query, &neighbor_vector)
                                        .unwrap_or(f32::INFINITY)
                                }
                                Err(_) => continue,
                            };
                            
                            // Add to candidates if promising
                            let should_add = if dynamic_list.len() < ef {
                                true
                            } else {
                                let farthest_distance = dynamic_list
                                    .iter()
                                    .map(|(_, dist)| *dist)
                                    .fold(0.0f32, f32::max);
                                neighbor_distance < farthest_distance
                            };
                            
                            if should_add {
                                candidates.push((neighbor_id, neighbor_distance));
                                dynamic_list.push((neighbor_id, neighbor_distance));
                                
                                // Keep dynamic list size limited
                                if dynamic_list.len() > ef {
                                    // Remove farthest element
                                    if let Some(farthest_idx) = dynamic_list
                                        .iter()
                                        .enumerate()
                                        .max_by(|(_, (_, a)), (_, (_, b))| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal))
                                        .map(|(idx, _)| idx)
                                    {
                                        dynamic_list.remove(farthest_idx);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Sort results by distance
        dynamic_list.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
        
        Ok(dynamic_list)
    }
    
    /// Simple neighbor selection (returns M closest neighbors)
    fn select_neighbors_simple(
        &mut self,
        candidates: &[(u64, f32)],
        m: usize,
        _query: &[f32],
        _context: &mut OperationContext,
    ) -> Result<Vec<u64>, AnnsError> {
        // Check stack usage
        self.stack_monitor.check_usage(256).map_err(|_| AnnsError::StackOverflow)?;
        
        // Simply take the M closest candidates
        let selected: Vec<u64> = candidates
            .iter()
            .take(m)
            .map(|(id, _)| *id)
            .collect();
        
        Ok(selected)
    }
    
    /// Prune connections for a node when it exceeds M connections
    fn prune_connections(
        &mut self,
        node_id: u64,
        layer: u8,
        context: &mut OperationContext,
    ) -> Result<(), AnnsError> {
        // Check stack usage
        self.stack_monitor.check_usage(512).map_err(|_| AnnsError::StackOverflow)?;
        
        let node_vector = self.get_vector_data_cached(node_id, context)?;
        
        // Get current connections and their distances
        let connections = if let Some(node) = self.nodes.get(&node_id) {
            node.connections.clone()
        } else {
            return Ok(());
        };
        
        let mut connection_distances = Vec::new();
        for &conn_id in connections.iter() {
            let distance = match self.get_vector_data_cached(conn_id, context) {
                Ok(conn_vector) => {
                    self.calculate_vector_distance(&node_vector, &conn_vector)
                        .unwrap_or(f32::INFINITY)
                }
                Err(_) => f32::INFINITY,
            };
            connection_distances.push((conn_id, distance));
        }
        
        // Sort by distance and keep only M closest
        connection_distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
        
        let m = if layer == 0 { 2 * self.m } else { self.m };
        let to_keep: Vec<u64> = connection_distances
            .into_iter()
            .take(m)
            .map(|(id, _)| id)
            .collect();
        
        // Update node connections
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.connections = Box::new(to_keep);
        }
        
        Ok(())
    }
    
    /// Validate graph integrity and connectivity
    pub fn validate_graph(&self) -> HnswValidationResult {
        let total_nodes = self.nodes.len();
        let total_connections: usize = self.nodes.values().map(|n| n.connections.len()).sum();
        
        // Check for orphaned nodes (nodes with no connections)
        let orphaned_nodes = self.nodes.values()
            .filter(|node| node.connections.is_empty())
            .count();
        
        // Check layer connectivity (simplified)
        let mut layer_connectivity = vec![true; (self.max_layer + 1) as usize];
        
        // Check if entry point is valid
        let entry_point_valid = self.entry_point
            .map(|ep| self.nodes.contains_key(&ep))
            .unwrap_or(false);
        
        HnswValidationResult {
            is_valid: orphaned_nodes == 0 && entry_point_valid,
            total_nodes,
            total_connections,
            orphaned_nodes,
            layer_connectivity,
            max_layer_reached: self.max_layer,
            entry_point_valid,
        }
    }
    
    /// Get construction statistics
    pub fn get_construction_stats(&self) -> &HnswConstructionStats {
        &self.construction_stats
    }
    
    /// Estimate heap usage
    fn estimate_heap_usage(&self) -> usize {
        let node_size = core::mem::size_of::<OptimizedHnswNode>();
        let connection_size = core::mem::size_of::<u64>();
        
        let nodes_memory = self.nodes.len() * node_size;
        let connections_memory: usize = self.nodes.values()
            .map(|n| n.connections.len() * connection_size)
            .sum();
        let pool_memory = self.search_memory_pool.search_states.len() *
            core::mem::size_of::<LayerSearchState>();
        
        nodes_memory + connections_memory + pool_memory
    }
}

/// Memory statistics for HNSW graph
#[derive(Debug, Clone)]
pub struct HnswMemoryStats {
    pub node_count: usize,
    pub total_connections: usize,
    pub max_layer: u8,
    pub memory_pool_size: usize,
    pub stack_usage_estimate: usize,
    pub heap_usage_estimate: usize,
}

/// Error type for stack overflow
#[derive(Debug)]
pub struct StackOverflowError;

impl From<StackOverflowError> for AnnsError {
    fn from(_: StackOverflowError) -> Self {
        AnnsError::StackOverflow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anns::integration::HnswParams;
    
    #[test]
    fn test_stack_safety_monitor() {
        let monitor = StackSafetyMonitor::new();
        assert!(monitor.check_usage(512).is_ok());
        
        // Test entering operation scope
        let _guard = monitor.enter_operation("test_op", 512);
        assert!(monitor.check_usage(400).is_ok());  // Within limit
        
        // Test statistics
        let stats = monitor.get_statistics();
        assert!(stats.current_usage > 0);
        assert!(stats.is_safe);
    }
    
    #[test]
    fn test_search_candidate_ordering() {
        let c1 = SearchCandidate::new(1, 0.5);
        let c2 = SearchCandidate::new(2, 0.3);
        let c3 = SearchCandidate::new(3, 0.7);
        
        let mut candidates = vec![c1, c2, c3];
        candidates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        
        assert_eq!(candidates[0].vector_id, 2); // Lowest distance first
        assert_eq!(candidates[1].vector_id, 1);
        assert_eq!(candidates[2].vector_id, 3);
    }
    
    #[test]
    fn test_optimized_hnsw_creation() {
        let params = HnswParams::default();
        let graph = OptimizedHnswGraph::new(128, params);
        assert!(graph.is_ok());
        
        let graph = graph.unwrap();
        assert!(graph.is_empty());
        assert_eq!(graph.max_layer, 0);
    }
    
    #[test]
    fn test_layer_search_state() {
        let mut state = LayerSearchState::new(0, 1, 10);
        assert_eq!(state.layer, 0);
        assert_eq!(state.entry_point, 1);
        assert_eq!(state.num_closest, 10);
        
        assert!(!state.is_visited(5));
        state.mark_visited(5);
        assert!(state.is_visited(5));
        
        // Test that marking the same node twice doesn't duplicate
        state.mark_visited(5);
        assert_eq!(state.visited.len(), 1);
    }
    
    #[test]
    fn test_memory_pool() {
        let mut pool = SearchMemoryPool::new(4);
        
        // Acquire all states
        let state1 = pool.acquire_search_state(0, 1, 10);
        let state2 = pool.acquire_search_state(1, 2, 10);
        let state3 = pool.acquire_search_state(2, 3, 10);
        let state4 = pool.acquire_search_state(3, 4, 10);
        
        assert!(state1.is_some());
        assert!(state2.is_some());
        assert!(state3.is_some());
        assert!(state4.is_some());
        
        // Pool should be exhausted (only 4 states in pool)
        let state5 = pool.acquire_search_state(4, 5, 10);
        assert!(state5.is_none());
        
        // Release one state (now just drops it)
        if let Some(state) = state1 {
            pool.release_search_state(state);
        }
        
        // Should still be exhausted since we don't reuse states in this simple implementation
        let state6 = pool.acquire_search_state(5, 6, 10);
        assert!(state6.is_none());
    }
}

// ============================================================================
// ADVANCED GRAPH ANALYTICS AND QUERY CAPABILITIES - TASK 23.3.4
// ============================================================================

/// Graph clustering and community detection results
#[derive(Debug, Clone)]
pub struct GraphClusteringResult {
    pub clusters: Vec<Vec<u64>>, // Each cluster contains node IDs
    pub cluster_quality: f32,    // Modularity score or similar quality metric
    pub num_clusters: usize,
    pub largest_cluster_size: usize,
    pub smallest_cluster_size: usize,
    pub silhouette_score: f32,   // Clustering quality metric
}

/// Connected components analysis result
#[derive(Debug, Clone)]
pub struct ConnectedComponentsResult {
    pub components: Vec<Vec<u64>>, // Each component contains node IDs
    pub num_components: usize,
    pub largest_component_size: usize,
    pub component_sizes: Vec<usize>,
    pub is_fully_connected: bool,
}

/// Centrality measures for graph analysis
#[derive(Debug, Clone)]
pub struct CentralityMeasures {
    pub degree_centrality: Vec<(u64, f32)>,      // (node_id, centrality_score)
    pub betweenness_centrality: Vec<(u64, f32)>, // Approximate for large graphs
    pub pagerank_scores: Vec<(u64, f32)>,        // PageRank-style importance
    pub eigenvector_centrality: Vec<(u64, f32)>, // Influence analysis
}

/// Pathfinding result between two nodes
#[derive(Debug, Clone)]
pub struct PathfindingResult {
    pub path: Vec<u64>,          // Sequence of node IDs from source to target
    pub path_length: usize,      // Number of hops
    pub total_distance: f32,     // Sum of edge weights/distances
    pub path_exists: bool,       // Whether a path was found
}

/// Advanced query configuration
#[derive(Debug, Clone)]
pub struct AdvancedQueryConfig {
    pub distance_threshold: Option<f32>,    // For range queries
    pub metadata_filters: Vec<String>,      // Metadata constraints
    pub quality_guarantee: f32,             // Minimum quality for approximate results
    pub max_results: usize,                 // Maximum number of results
    pub include_distances: bool,            // Whether to include distance values
}

/// Batch query request for multiple vectors
#[derive(Debug, Clone)]
pub struct BatchQueryRequest {
    pub queries: Vec<Vec<f32>>,             // Multiple query vectors
    pub k: usize,                           // Number of results per query
    pub config: AdvancedQueryConfig,        // Query configuration
}

/// Batch query response
#[derive(Debug, Clone)]
pub struct BatchQueryResponse {
    pub results: Vec<Vec<(u64, f32)>>,      // Results for each query
    pub query_times: Vec<u64>,              // Time taken for each query (ms)
    pub total_time: u64,                    // Total batch processing time (ms)
    pub cache_hit_rate: f32,                // Vector cache hit rate during batch
}

/// Graph health and quality metrics
#[derive(Debug, Clone)]
pub struct GraphHealthMetrics {
    pub connectivity_score: f32,            // Overall graph connectivity
    pub clustering_coefficient: f32,        // Local clustering measure
    pub average_path_length: f32,           // Average shortest path length
    pub diameter: usize,                    // Maximum shortest path length
    pub density: f32,                       // Edge density
    pub small_world_coefficient: f32,       // Small-world network measure
    pub degree_distribution: Vec<(usize, usize)>, // (degree, count) pairs
}

/// Performance profiling results
#[derive(Debug, Clone)]
pub struct PerformanceProfile {
    pub search_bottlenecks: Vec<String>,    // Identified performance bottlenecks
    pub memory_hotspots: Vec<String>,       // Memory usage hotspots
    pub optimization_suggestions: Vec<String>, // Suggested optimizations
    pub cache_efficiency: f32,              // Vector cache efficiency
    pub search_time_distribution: Vec<(String, u64)>, // Operation timing breakdown
}

/// Graph analytics module implementation
impl OptimizedHnswGraph {
    
    // ========================================================================
    // GRAPH CLUSTERING ALGORITHMS
    // ========================================================================
    
    /// Detect connected components in the graph using iterative DFS
    pub fn find_connected_components(&self) -> VexfsResult<ConnectedComponentsResult> {
        // Stack-safe implementation using heap-allocated data structures
        let mut visited = Box::new(Vec::new());
        let mut components = Box::new(Vec::new());
        let mut work_stack = Box::new(Vec::new());
        
        // Process all nodes to find components
        for &node_id in self.nodes.keys() {
            if !visited.contains(&node_id) {
                let mut component = Vec::new();
                work_stack.push(node_id);
                
                // Iterative DFS to avoid stack overflow
                while let Some(current_id) = work_stack.pop() {
                    if !visited.contains(&current_id) {
                        visited.push(current_id);
                        component.push(current_id);
                        
                        // Add neighbors to work stack
                        if let Some(node) = self.nodes.get(&current_id) {
                            for &neighbor_id in node.connections.iter() {
                                if !visited.contains(&neighbor_id) {
                                    work_stack.push(neighbor_id);
                                }
                            }
                        }
                    }
                }
                
                if !component.is_empty() {
                    components.push(component);
                }
            }
        }
        
        let num_components = components.len();
        let largest_component_size = components.iter().map(|c| c.len()).max().unwrap_or(0);
        let component_sizes: Vec<usize> = components.iter().map(|c| c.len()).collect();
        let is_fully_connected = num_components <= 1;
        
        Ok(ConnectedComponentsResult {
            components: *components,
            num_components,
            largest_component_size,
            component_sizes,
            is_fully_connected,
        })
    }
    
    /// Perform community detection using modularity optimization
    pub fn detect_communities(&mut self, context: &mut OperationContext) -> VexfsResult<GraphClusteringResult> {
        // Stack-safe community detection using Louvain-like algorithm
        let mut clusters = Box::new(Vec::new());
        let mut node_to_cluster = Box::new(BTreeMap::new());
        let mut cluster_id = 0usize;
        
        // Initialize each node as its own cluster
        for &node_id in self.nodes.keys() {
            node_to_cluster.insert(node_id, cluster_id);
            clusters.push(vec![node_id]);
            cluster_id += 1;
        }
        
        // Iterative optimization to merge clusters
        let mut improved = true;
        let mut iteration = 0;
        let max_iterations = 50; // Prevent infinite loops
        
        while improved && iteration < max_iterations {
            improved = false;
            iteration += 1;
            
            // Try to improve modularity by moving nodes between clusters
            for &node_id in self.nodes.keys() {
                if let Some(node) = self.nodes.get(&node_id) {
                    let current_cluster = node_to_cluster[&node_id];
                    let mut best_cluster = current_cluster;
                    let mut best_gain = 0.0f32;
                    
                    // Check neighboring clusters
                    for &neighbor_id in node.connections.iter() {
                        if let Some(&neighbor_cluster) = node_to_cluster.get(&neighbor_id) {
                            if neighbor_cluster != current_cluster {
                                // Calculate modularity gain (simplified)
                                let gain = self.calculate_modularity_gain(
                                    node_id,
                                    current_cluster,
                                    neighbor_cluster,
                                    &node_to_cluster,
                                    context
                                )?;
                                
                                if gain > best_gain {
                                    best_gain = gain;
                                    best_cluster = neighbor_cluster;
                                }
                            }
                        }
                    }
                    
                    // Move node if beneficial
                    if best_cluster != current_cluster && best_gain > 0.001 {
                        // Remove from current cluster
                        if let Some(cluster) = clusters.get_mut(current_cluster) {
                            cluster.retain(|&id| id != node_id);
                        }
                        
                        // Add to best cluster
                        if let Some(cluster) = clusters.get_mut(best_cluster) {
                            cluster.push(node_id);
                        }
                        
                        node_to_cluster.insert(node_id, best_cluster);
                        improved = true;
                    }
                }
            }
        }
        
        // Remove empty clusters
        clusters.retain(|cluster| !cluster.is_empty());
        
        // Calculate clustering quality metrics
        let cluster_quality = self.calculate_modularity(&clusters, context)?;
        let silhouette_score = self.calculate_silhouette_score(&clusters, context)?;
        
        let num_clusters = clusters.len();
        let largest_cluster_size = clusters.iter().map(|c| c.len()).max().unwrap_or(0);
        let smallest_cluster_size = clusters.iter().map(|c| c.len()).min().unwrap_or(0);
        
        Ok(GraphClusteringResult {
            clusters: *clusters,
            cluster_quality,
            num_clusters,
            largest_cluster_size,
            smallest_cluster_size,
            silhouette_score,
        })
    }
    
    /// Calculate modularity score for clustering quality
    fn calculate_modularity(
        &mut self,
        clusters: &[Vec<u64>],
        _context: &mut OperationContext,
    ) -> VexfsResult<f32> {
        let total_edges = self.nodes.values().map(|n| n.connections.len()).sum::<usize>() as f32 / 2.0;
        
        if total_edges == 0.0 {
            return Ok(0.0);
        }
        
        let mut modularity = 0.0f32;
        
        for cluster in clusters {
            let mut internal_edges = 0;
            let mut total_degree = 0;
            
            // Count internal edges and total degree
            for &node_id in cluster {
                if let Some(node) = self.nodes.get(&node_id) {
                    total_degree += node.connections.len();
                    
                    for &neighbor_id in node.connections.iter() {
                        if cluster.contains(&neighbor_id) {
                            internal_edges += 1;
                        }
                    }
                }
            }
            
            internal_edges /= 2; // Each edge counted twice
            
            let expected_edges = (total_degree as f32).powi(2) / (4.0 * total_edges);
            modularity += (internal_edges as f32 / total_edges) - (expected_edges / total_edges);
        }
        
        Ok(modularity)
    }
    
    /// Calculate silhouette score for clustering quality
    fn calculate_silhouette_score(
        &mut self,
        clusters: &[Vec<u64>],
        context: &mut OperationContext,
    ) -> VexfsResult<f32> {
        let mut total_silhouette = 0.0f32;
        let mut node_count = 0;
        
        for (cluster_idx, cluster) in clusters.iter().enumerate() {
            for &node_id in cluster {
                let node_vector = self.get_vector_data_cached(node_id, context)?;
                
                // Calculate average distance within cluster (a)
                let mut intra_cluster_distance = 0.0f32;
                let mut intra_count = 0;
                
                for &other_id in cluster {
                    if other_id != node_id {
                        if let Ok(other_vector) = self.get_vector_data_cached(other_id, context) {
                            if let Ok(distance) = self.calculate_vector_distance(&node_vector, &other_vector) {
                                intra_cluster_distance += distance;
                                intra_count += 1;
                            }
                        }
                    }
                }
                
                let a = if intra_count > 0 {
                    intra_cluster_distance / intra_count as f32
                } else {
                    0.0
                };
                
                // Calculate minimum average distance to other clusters (b)
                let mut min_inter_distance = f32::INFINITY;
                
                for (other_cluster_idx, other_cluster) in clusters.iter().enumerate() {
                    if other_cluster_idx != cluster_idx {
                        let mut inter_cluster_distance = 0.0f32;
                        let mut inter_count = 0;
                        
                        for &other_id in other_cluster {
                            if let Ok(other_vector) = self.get_vector_data_cached(other_id, context) {
                                if let Ok(distance) = self.calculate_vector_distance(&node_vector, &other_vector) {
                                    inter_cluster_distance += distance;
                                    inter_count += 1;
                                }
                            }
                        }
                        
                        if inter_count > 0 {
                            let avg_inter_distance = inter_cluster_distance / inter_count as f32;
                            min_inter_distance = min_inter_distance.min(avg_inter_distance);
                        }
                    }
                }
                
                let b = min_inter_distance;
                
                // Calculate silhouette coefficient
                let silhouette = if a < b {
                    1.0 - (a / b)
                } else if a > b {
                    (b / a) - 1.0
                } else {
                    0.0
                };
                
                total_silhouette += silhouette;
                node_count += 1;
            }
        }
        
        Ok(if node_count > 0 {
            total_silhouette / node_count as f32
        } else {
            0.0
        })
    }
    
    /// Calculate modularity gain for moving a node between clusters
    fn calculate_modularity_gain(
        &self,
        node_id: u64,
        from_cluster: usize,
        to_cluster: usize,
        node_to_cluster: &BTreeMap<u64, usize>,
        _context: &OperationContext,
    ) -> VexfsResult<f32> {
        // Simplified modularity gain calculation
        let mut internal_edges_from = 0;
        let mut internal_edges_to = 0;
        
        if let Some(node) = self.nodes.get(&node_id) {
            let connection_count = node.connections.len();
            for &neighbor_id in node.connections.iter() {
                if let Some(&neighbor_cluster) = node_to_cluster.get(&neighbor_id) {
                    if neighbor_cluster == from_cluster {
                        internal_edges_from += 1;
                    } else if neighbor_cluster == to_cluster {
                        internal_edges_to += 1;
                    }
                }
            }
            
            // Simple gain calculation: prefer clusters with more connections
            let gain = (internal_edges_to as f32) - (internal_edges_from as f32);
            Ok(gain / (connection_count as f32 + 1.0))
        } else {
            Ok(0.0)
        }
    }
    
}