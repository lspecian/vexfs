//! Efficient Serialization for Semantic API
//! 
//! This module implements efficient serialization of semantic events
//! optimized for agent consumption, including compression and format options.

use crate::semantic_api::{SemanticResult, SemanticError};
use crate::semantic_api::types::*;
use serde::{Serialize, Deserialize};
use std::io::{Read, Write};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use lz4::{Decoder, EncoderBuilder};

/// Serialization format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SerializationFormat {
    Json,
    JsonCompact,
    MessagePack,
    Cbor,
    Bincode,
}

/// Compression options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Gzip,
    Lz4,
}

/// Serialization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializationConfig {
    pub format: SerializationFormat,
    pub compression: CompressionType,
    pub compression_threshold: usize,
    pub pretty_print: bool,
    pub include_metadata: bool,
}

impl Default for SerializationConfig {
    fn default() -> Self {
        Self {
            format: SerializationFormat::Json,
            compression: CompressionType::None,
            compression_threshold: 1024,
            pretty_print: false,
            include_metadata: true,
        }
    }
}

/// Serializer for semantic events
#[derive(Debug)]
pub struct SemanticSerializer {
    config: SerializationConfig,
}

impl SemanticSerializer {
    /// Create a new serializer with the given configuration
    pub fn new(config: SerializationConfig) -> Self {
        Self { config }
    }
    
    /// Serialize a single semantic event
    pub fn serialize_event(&self, event: &SemanticEvent) -> SemanticResult<Vec<u8>> {
        let data = self.serialize_to_format(event)?;
        self.apply_compression(data)
    }
    
    /// Serialize multiple semantic events
    pub fn serialize_events(&self, events: &[SemanticEvent]) -> SemanticResult<Vec<u8>> {
        let data = self.serialize_to_format(events)?;
        self.apply_compression(data)
    }
    
    /// Serialize an event query response
    pub fn serialize_query_response(&self, response: &EventQueryResponse) -> SemanticResult<Vec<u8>> {
        let data = self.serialize_to_format(response)?;
        self.apply_compression(data)
    }
    
    /// Serialize an API response
    pub fn serialize_api_response<T: Serialize>(&self, response: &ApiResponse<T>) -> SemanticResult<Vec<u8>> {
        let data = self.serialize_to_format(response)?;
        self.apply_compression(data)
    }
    
    /// Serialize stream message
    pub fn serialize_stream_message(&self, message: &StreamMessage) -> SemanticResult<Vec<u8>> {
        let data = self.serialize_to_format(message)?;
        self.apply_compression(data)
    }
    
    /// Deserialize a single semantic event
    pub fn deserialize_event(&self, data: &[u8]) -> SemanticResult<SemanticEvent> {
        let decompressed = self.decompress_data(data)?;
        self.deserialize_from_format(&decompressed)
    }
    
    /// Deserialize multiple semantic events
    pub fn deserialize_events(&self, data: &[u8]) -> SemanticResult<Vec<SemanticEvent>> {
        let decompressed = self.decompress_data(data)?;
        self.deserialize_from_format(&decompressed)
    }
    
    /// Deserialize an event query response
    pub fn deserialize_query_response(&self, data: &[u8]) -> SemanticResult<EventQueryResponse> {
        let decompressed = self.decompress_data(data)?;
        self.deserialize_from_format(&decompressed)
    }
    
    /// Serialize to the configured format
    fn serialize_to_format<T: Serialize>(&self, value: &T) -> SemanticResult<Vec<u8>> {
        match self.config.format {
            SerializationFormat::Json => {
                if self.config.pretty_print {
                    serde_json::to_vec_pretty(value)
                        .map_err(|e| SemanticError::SerializationError(e.to_string()))
                } else {
                    serde_json::to_vec(value)
                        .map_err(|e| SemanticError::SerializationError(e.to_string()))
                }
            }
            SerializationFormat::JsonCompact => {
                serde_json::to_vec(value)
                    .map_err(|e| SemanticError::SerializationError(e.to_string()))
            }
            SerializationFormat::MessagePack => {
                rmp_serde::to_vec(value)
                    .map_err(|e| SemanticError::SerializationError(e.to_string()))
            }
            SerializationFormat::Cbor => {
                serde_cbor::to_vec(value)
                    .map_err(|e| SemanticError::SerializationError(e.to_string()))
            }
            SerializationFormat::Bincode => {
                bincode::serialize(value)
                    .map_err(|e| SemanticError::SerializationError(e.to_string()))
            }
        }
    }
    
    /// Deserialize from the configured format
    fn deserialize_from_format<T: for<'de> Deserialize<'de>>(&self, data: &[u8]) -> SemanticResult<T> {
        match self.config.format {
            SerializationFormat::Json | SerializationFormat::JsonCompact => {
                serde_json::from_slice(data)
                    .map_err(|e| SemanticError::SerializationError(e.to_string()))
            }
            SerializationFormat::MessagePack => {
                rmp_serde::from_slice(data)
                    .map_err(|e| SemanticError::SerializationError(e.to_string()))
            }
            SerializationFormat::Cbor => {
                serde_cbor::from_slice(data)
                    .map_err(|e| SemanticError::SerializationError(e.to_string()))
            }
            SerializationFormat::Bincode => {
                bincode::deserialize(data)
                    .map_err(|e| SemanticError::SerializationError(e.to_string()))
            }
        }
    }
    
    /// Apply compression if configured and data exceeds threshold
    fn apply_compression(&self, data: Vec<u8>) -> SemanticResult<Vec<u8>> {
        if self.config.compression == CompressionType::None || 
           data.len() < self.config.compression_threshold {
            return Ok(data);
        }
        
        match self.config.compression {
            CompressionType::None => Ok(data),
            CompressionType::Gzip => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&data)
                    .map_err(|e| SemanticError::SerializationError(e.to_string()))?;
                encoder.finish()
                    .map_err(|e| SemanticError::SerializationError(e.to_string()))
            }
            CompressionType::Lz4 => {
                let mut encoder = EncoderBuilder::new()
                    .level(4)
                    .build(Vec::new())
                    .map_err(|e| SemanticError::SerializationError(e.to_string()))?;
                encoder.write_all(&data)
                    .map_err(|e| SemanticError::SerializationError(e.to_string()))?;
                let (compressed, _) = encoder.finish();
                Ok(compressed)
            }
        }
    }
    
    /// Decompress data if needed
    fn decompress_data(&self, data: &[u8]) -> SemanticResult<Vec<u8>> {
        match self.config.compression {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => {
                let mut decoder = GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)
                    .map_err(|e| SemanticError::SerializationError(e.to_string()))?;
                Ok(decompressed)
            }
            CompressionType::Lz4 => {
                let mut decoder = Decoder::new(data)
                    .map_err(|e| SemanticError::SerializationError(e.to_string()))?;
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)
                    .map_err(|e| SemanticError::SerializationError(e.to_string()))?;
                Ok(decompressed)
            }
        }
    }
    
    /// Get the MIME type for the current format
    pub fn get_content_type(&self) -> &'static str {
        match self.config.format {
            SerializationFormat::Json | SerializationFormat::JsonCompact => "application/json",
            SerializationFormat::MessagePack => "application/msgpack",
            SerializationFormat::Cbor => "application/cbor",
            SerializationFormat::Bincode => "application/octet-stream",
        }
    }
    
    /// Get the file extension for the current format
    pub fn get_file_extension(&self) -> &'static str {
        match self.config.format {
            SerializationFormat::Json | SerializationFormat::JsonCompact => "json",
            SerializationFormat::MessagePack => "msgpack",
            SerializationFormat::Cbor => "cbor",
            SerializationFormat::Bincode => "bin",
        }
    }
    
    /// Estimate serialized size without actually serializing
    pub fn estimate_size<T: Serialize>(&self, value: &T) -> SemanticResult<usize> {
        // For estimation, we use JSON as it's generally readable
        let json_size = serde_json::to_vec(value)
            .map_err(|e| SemanticError::SerializationError(e.to_string()))?
            .len();
        
        // Apply format-specific size adjustments
        let estimated_size = match self.config.format {
            SerializationFormat::Json | SerializationFormat::JsonCompact => json_size,
            SerializationFormat::MessagePack => (json_size as f64 * 0.7) as usize, // ~30% smaller
            SerializationFormat::Cbor => (json_size as f64 * 0.8) as usize, // ~20% smaller
            SerializationFormat::Bincode => (json_size as f64 * 0.6) as usize, // ~40% smaller
        };
        
        // Apply compression estimation
        let final_size = if self.config.compression != CompressionType::None && 
                           estimated_size >= self.config.compression_threshold {
            match self.config.compression {
                CompressionType::None => estimated_size,
                CompressionType::Gzip => (estimated_size as f64 * 0.3) as usize, // ~70% compression
                CompressionType::Lz4 => (estimated_size as f64 * 0.5) as usize, // ~50% compression
            }
        } else {
            estimated_size
        };
        
        Ok(final_size)
    }
}

/// Batch serializer for efficient processing of multiple events
#[derive(Debug)]
pub struct BatchSerializer {
    serializer: SemanticSerializer,
    batch_size: usize,
}

impl BatchSerializer {
    /// Create a new batch serializer
    pub fn new(config: SerializationConfig, batch_size: usize) -> Self {
        Self {
            serializer: SemanticSerializer::new(config),
            batch_size,
        }
    }
    
    /// Serialize events in batches
    pub fn serialize_events_batched(&self, events: &[SemanticEvent]) -> SemanticResult<Vec<Vec<u8>>> {
        let mut results = Vec::new();
        
        for chunk in events.chunks(self.batch_size) {
            let serialized = self.serializer.serialize_events(chunk)?;
            results.push(serialized);
        }
        
        Ok(results)
    }
    
    /// Serialize events as streaming format (JSONL, etc.)
    pub fn serialize_events_streaming(&self, events: &[SemanticEvent]) -> SemanticResult<Vec<u8>> {
        let mut result = Vec::new();
        
        for event in events {
            let serialized = self.serializer.serialize_event(event)?;
            result.extend_from_slice(&serialized);
            result.push(b'\n'); // Add newline separator for streaming formats
        }
        
        Ok(result)
    }
}

/// Adaptive serializer that chooses format based on data characteristics
#[derive(Debug)]
pub struct AdaptiveSerializer {
    configs: Vec<(SerializationConfig, f64)>, // (config, efficiency_score)
}

impl AdaptiveSerializer {
    /// Create a new adaptive serializer
    pub fn new() -> Self {
        let configs = vec![
            (SerializationConfig {
                format: SerializationFormat::Bincode,
                compression: CompressionType::Lz4,
                ..Default::default()
            }, 0.9), // High efficiency for binary data
            (SerializationConfig {
                format: SerializationFormat::MessagePack,
                compression: CompressionType::Gzip,
                ..Default::default()
            }, 0.8), // Good balance
            (SerializationConfig {
                format: SerializationFormat::JsonCompact,
                compression: CompressionType::Gzip,
                ..Default::default()
            }, 0.6), // Human readable but larger
            (SerializationConfig {
                format: SerializationFormat::Json,
                compression: CompressionType::None,
                pretty_print: true,
                ..Default::default()
            }, 0.3), // Debug/development format
        ];
        
        Self { configs }
    }
    
    /// Choose the best serialization config for the given data
    pub fn choose_config<T: Serialize>(&self, value: &T, prefer_human_readable: bool) -> SemanticResult<SerializationConfig> {
        if prefer_human_readable {
            // Return JSON for human readability
            return Ok(SerializationConfig {
                format: SerializationFormat::Json,
                compression: CompressionType::None,
                pretty_print: true,
                ..Default::default()
            });
        }
        
        // For now, return the most efficient config
        // In a real implementation, we might test multiple formats and choose the best
        Ok(self.configs[0].0.clone())
    }
    
    /// Serialize with adaptive format selection
    pub fn serialize_adaptive<T: Serialize>(&self, value: &T, prefer_human_readable: bool) -> SemanticResult<(Vec<u8>, SerializationConfig)> {
        let config = self.choose_config(value, prefer_human_readable)?;
        let serializer = SemanticSerializer::new(config.clone());
        let data = serializer.serialize_to_format(value)?;
        let compressed = serializer.apply_compression(data)?;
        Ok((compressed, config))
    }
}

/// Serialization utilities
pub mod utils {
    use super::*;
    
    /// Convert between serialization formats
    pub fn convert_format<T>(
        data: &[u8],
        from_config: &SerializationConfig,
        to_config: &SerializationConfig,
    ) -> SemanticResult<Vec<u8>>
    where
        T: for<'de> Deserialize<'de> + Serialize,
    {
        let from_serializer = SemanticSerializer::new(from_config.clone());
        let to_serializer = SemanticSerializer::new(to_config.clone());
        
        let value: T = from_serializer.deserialize_from_format(data)?;
        to_serializer.serialize_to_format(&value)
    }
    
    /// Compress existing data
    pub fn compress_data(data: &[u8], compression: CompressionType) -> SemanticResult<Vec<u8>> {
        let config = SerializationConfig {
            compression,
            compression_threshold: 0, // Always compress
            ..Default::default()
        };
        let serializer = SemanticSerializer::new(config);
        serializer.apply_compression(data.to_vec())
    }
    
    /// Decompress data
    pub fn decompress_data(data: &[u8], compression: CompressionType) -> SemanticResult<Vec<u8>> {
        let config = SerializationConfig {
            compression,
            ..Default::default()
        };
        let serializer = SemanticSerializer::new(config);
        serializer.decompress_data(data)
    }
    
    /// Get optimal compression for data
    pub fn get_optimal_compression(data: &[u8]) -> CompressionType {
        // Simple heuristic: use LZ4 for smaller data, Gzip for larger data
        if data.len() < 10_000 {
            CompressionType::Lz4
        } else {
            CompressionType::Gzip
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_event() -> SemanticEvent {
        SemanticEvent {
            event_id: 1,
            event_type: SemanticEventType::FilesystemCreate,
            event_subtype: None,
            timestamp: SemanticTimestamp {
                timestamp: Utc::now(),
                sequence: 1,
                cpu_id: 0,
                process_id: 1234,
            },
            global_sequence: 1,
            local_sequence: 1,
            flags: EventFlags::from_kernel_flags(0),
            priority: EventPriority::Normal,
            event_size: 100,
            event_version: 1,
            checksum: None,
            compression_type: None,
            encryption_type: None,
            causality_links: Vec::new(),
            parent_event_id: None,
            root_cause_event_id: None,
            agent_visibility_mask: 0xFF,
            agent_relevance_score: 50,
            replay_priority: 1,
            context: SemanticContext {
                transaction_id: None,
                session_id: None,
                causality_chain_id: None,
                filesystem: None,
                graph: None,
                vector: None,
                agent: None,
                system: None,
                semantic: None,
            },
            payload: None,
            metadata: None,
        }
    }

    #[test]
    fn test_json_serialization() {
        let config = SerializationConfig {
            format: SerializationFormat::Json,
            ..Default::default()
        };
        let serializer = SemanticSerializer::new(config);
        let event = create_test_event();
        
        let serialized = serializer.serialize_event(&event).unwrap();
        let deserialized: SemanticEvent = serializer.deserialize_event(&serialized).unwrap();
        
        assert_eq!(event.event_id, deserialized.event_id);
        assert_eq!(event.event_type, deserialized.event_type);
    }
    
    #[test]
    fn test_compression() {
        let config = SerializationConfig {
            format: SerializationFormat::Json,
            compression: CompressionType::Gzip,
            compression_threshold: 0, // Always compress
            ..Default::default()
        };
        let serializer = SemanticSerializer::new(config);
        let event = create_test_event();
        
        let serialized = serializer.serialize_event(&event).unwrap();
        let deserialized: SemanticEvent = serializer.deserialize_event(&serialized).unwrap();
        
        assert_eq!(event.event_id, deserialized.event_id);
    }
    
    #[test]
    fn test_batch_serialization() {
        let config = SerializationConfig::default();
        let batch_serializer = BatchSerializer::new(config, 2);
        let events = vec![create_test_event(), create_test_event(), create_test_event()];
        
        let batches = batch_serializer.serialize_events_batched(&events).unwrap();
        assert_eq!(batches.len(), 2); // 3 events with batch size 2 = 2 batches
    }
    
    #[test]
    fn test_content_type() {
        let json_config = SerializationConfig {
            format: SerializationFormat::Json,
            ..Default::default()
        };
        let json_serializer = SemanticSerializer::new(json_config);
        assert_eq!(json_serializer.get_content_type(), "application/json");
        
        let msgpack_config = SerializationConfig {
            format: SerializationFormat::MessagePack,
            ..Default::default()
        };
        let msgpack_serializer = SemanticSerializer::new(msgpack_config);
        assert_eq!(msgpack_serializer.get_content_type(), "application/msgpack");
    }
    
    #[test]
    fn test_adaptive_serializer() {
        let adaptive = AdaptiveSerializer::new();
        let event = create_test_event();
        
        let (data, config) = adaptive.serialize_adaptive(&event, false).unwrap();
        assert!(!data.is_empty());
        assert_eq!(config.format, SerializationFormat::Bincode);
        
        let (data, config) = adaptive.serialize_adaptive(&event, true).unwrap();
        assert!(!data.is_empty());
        assert_eq!(config.format, SerializationFormat::Json);
    }
}