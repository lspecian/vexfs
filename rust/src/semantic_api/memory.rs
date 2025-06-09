//! Agent Memory Interfaces
//!
//! This module implements episodic and semantic memory systems for AI agents,
//! providing efficient storage, retrieval, and indexing of agent memories
//! integrated with VexGraph.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error, instrument};
use uuid::Uuid;

use crate::semantic_api::{SemanticResult, SemanticError, types::*};

/// Agent memory manager
pub struct AgentMemoryManager {
    episodic_memory: Arc<RwLock<EpisodicMemoryStore>>,
    semantic_memory: Arc<RwLock<SemanticMemoryStore>>,
    working_memory: Arc<RwLock<WorkingMemoryStore>>,
    memory_index: Arc<RwLock<MemoryIndex>>,
    config: MemoryConfig,
}

/// Memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub max_episodic_memories_per_agent: usize,
    pub max_semantic_memories_per_agent: usize,
    pub max_working_memory_size: usize,
    pub memory_retention_days: u32,
    pub enable_memory_compression: bool,
    pub enable_memory_clustering: bool,
    pub similarity_threshold: f32,
    pub index_update_interval_seconds: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_episodic_memories_per_agent: 10000,
            max_semantic_memories_per_agent: 50000,
            max_working_memory_size: 1000,
            memory_retention_days: 365,
            enable_memory_compression: true,
            enable_memory_clustering: true,
            similarity_threshold: 0.8,
            index_update_interval_seconds: 300, // 5 minutes
        }
    }
}

/// Episodic memory store for agent experiences
pub struct EpisodicMemoryStore {
    memories: HashMap<String, Vec<EpisodicMemory>>, // agent_id -> memories
    memory_timeline: HashMap<String, VecDeque<String>>, // agent_id -> memory_ids (chronological)
    config: MemoryConfig,
}

/// Episodic memory representing a specific experience
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemory {
    pub memory_id: String,
    pub agent_id: String,
    pub episode_type: EpisodeType,
    pub timestamp: DateTime<Utc>,
    pub duration: Option<Duration>,
    pub context: EpisodicContext,
    pub content: EpisodicContent,
    pub emotional_valence: f32, // -1.0 (negative) to 1.0 (positive)
    pub importance_score: f32,  // 0.0 to 1.0
    pub access_count: u32,
    pub last_accessed: DateTime<Utc>,
    pub related_memories: Vec<String>,
    pub tags: Vec<String>,
}

/// Types of episodic memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EpisodeType {
    TaskExecution,
    ProblemSolving,
    Learning,
    Interaction,
    Decision,
    Error,
    Success,
    Discovery,
}

/// Context for episodic memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicContext {
    pub location: Option<String>,
    pub other_agents: Vec<String>,
    pub task_id: Option<String>,
    pub session_id: Option<String>,
    pub environmental_factors: HashMap<String, serde_json::Value>,
}

/// Content of episodic memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicContent {
    pub description: String,
    pub actions_taken: Vec<Action>,
    pub observations: Vec<Observation>,
    pub outcomes: Vec<Outcome>,
    pub lessons_learned: Vec<String>,
    pub raw_data: Option<serde_json::Value>,
}

/// Action taken during an episode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub result: Option<String>,
}

/// Observation made during an episode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub observation_type: String,
    pub content: String,
    pub confidence: f32,
    pub timestamp: DateTime<Utc>,
}

/// Outcome of an episode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub outcome_type: OutcomeType,
    pub description: String,
    pub success: bool,
    pub metrics: HashMap<String, f64>,
}

/// Types of outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutcomeType {
    TaskCompletion,
    ProblemResolution,
    LearningAchievement,
    ErrorRecovery,
    PerformanceImprovement,
}

/// Semantic memory store for conceptual knowledge
pub struct SemanticMemoryStore {
    concepts: HashMap<String, Vec<SemanticConcept>>, // agent_id -> concepts
    relationships: HashMap<String, Vec<ConceptRelationship>>, // agent_id -> relationships
    concept_index: HashMap<String, HashMap<String, String>>, // agent_id -> concept_name -> concept_id
    config: MemoryConfig,
}

/// Semantic concept in agent memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticConcept {
    pub concept_id: String,
    pub agent_id: String,
    pub name: String,
    pub concept_type: ConceptType,
    pub definition: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub examples: Vec<String>,
    pub confidence: f32,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub usage_count: u32,
    pub embedding: Option<Vec<f32>>,
    pub source_memories: Vec<String>, // episodic memory IDs that contributed to this concept
}

/// Types of semantic concepts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConceptType {
    Entity,
    Relationship,
    Process,
    Rule,
    Pattern,
    Strategy,
    Heuristic,
    Fact,
}

/// Relationship between semantic concepts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptRelationship {
    pub relationship_id: String,
    pub agent_id: String,
    pub source_concept_id: String,
    pub target_concept_id: String,
    pub relationship_type: RelationshipType,
    pub strength: f32, // 0.0 to 1.0
    pub confidence: f32,
    pub created_at: DateTime<Utc>,
    pub evidence: Vec<String>, // memory IDs supporting this relationship
}

/// Types of concept relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    IsA,
    PartOf,
    CausedBy,
    Enables,
    Requires,
    Similar,
    Opposite,
    Temporal,
    Spatial,
    Functional,
}

/// Working memory for current context and active information
pub struct WorkingMemoryStore {
    active_memories: HashMap<String, WorkingMemorySet>, // agent_id -> working memory
    config: MemoryConfig,
}

/// Working memory set for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemorySet {
    pub agent_id: String,
    pub current_context: WorkingContext,
    pub active_concepts: Vec<String>, // concept IDs
    pub recent_episodes: Vec<String>, // episodic memory IDs
    pub attention_focus: Vec<AttentionItem>,
    pub goals: Vec<Goal>,
    pub constraints: Vec<Constraint>,
    pub last_updated: DateTime<Utc>,
}

/// Current working context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingContext {
    pub current_task: Option<String>,
    pub current_session: Option<String>,
    pub active_reasoning: Option<String>,
    pub environmental_state: HashMap<String, serde_json::Value>,
}

/// Item in attention focus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionItem {
    pub item_type: AttentionType,
    pub item_id: String,
    pub activation_level: f32, // 0.0 to 1.0
    pub last_accessed: DateTime<Utc>,
}

/// Types of attention items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttentionType {
    Concept,
    Memory,
    Goal,
    Problem,
    Opportunity,
}

/// Goal in working memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub goal_id: String,
    pub description: String,
    pub priority: f32,
    pub progress: f32, // 0.0 to 1.0
    pub deadline: Option<DateTime<Utc>>,
    pub sub_goals: Vec<String>,
}

/// Constraint in working memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub constraint_id: String,
    pub constraint_type: ConstraintType,
    pub description: String,
    pub severity: f32, // 0.0 to 1.0
}

/// Types of constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    Resource,
    Time,
    Quality,
    Safety,
    Policy,
    Capability,
}

/// Memory index for efficient retrieval
pub struct MemoryIndex {
    temporal_index: HashMap<String, Vec<(DateTime<Utc>, String)>>, // agent_id -> (timestamp, memory_id)
    content_index: HashMap<String, HashMap<String, Vec<String>>>, // agent_id -> keyword -> memory_ids
    similarity_index: HashMap<String, Vec<(String, Vec<f32>)>>, // agent_id -> (memory_id, embedding)
    tag_index: HashMap<String, HashMap<String, Vec<String>>>, // agent_id -> tag -> memory_ids
}

/// Memory query for retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub agent_id: String,
    pub query_type: MemoryQueryType,
    pub filters: MemoryFilters,
    pub limit: Option<usize>,
    pub similarity_threshold: Option<f32>,
}

/// Types of memory queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryQueryType {
    Episodic,
    Semantic,
    Working,
    Hybrid,
    Similarity,
    Temporal,
}

/// Filters for memory queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFilters {
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub tags: Option<Vec<String>>,
    pub content_keywords: Option<Vec<String>>,
    pub importance_threshold: Option<f32>,
    pub episode_types: Option<Vec<EpisodeType>>,
    pub concept_types: Option<Vec<ConceptType>>,
}

/// Memory query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryResult {
    pub query_id: String,
    pub agent_id: String,
    pub episodic_memories: Vec<EpisodicMemory>,
    pub semantic_concepts: Vec<SemanticConcept>,
    pub working_context: Option<WorkingMemorySet>,
    pub total_results: usize,
    pub execution_time_ms: u64,
}

impl AgentMemoryManager {
    /// Create a new agent memory manager
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            episodic_memory: Arc::new(RwLock::new(EpisodicMemoryStore::new(config.clone()))),
            semantic_memory: Arc::new(RwLock::new(SemanticMemoryStore::new(config.clone()))),
            working_memory: Arc::new(RwLock::new(WorkingMemoryStore::new(config.clone()))),
            memory_index: Arc::new(RwLock::new(MemoryIndex::new())),
            config,
        }
    }

    /// Store an episodic memory
    #[instrument(skip(self))]
    pub async fn store_episodic_memory(&self, memory: EpisodicMemory) -> SemanticResult<()> {
        let memory_id = memory.memory_id.clone();
        let agent_id = memory.agent_id.clone();

        // Store in episodic memory
        {
            let mut episodic = self.episodic_memory.write().await;
            episodic.store_memory(memory.clone())?;
        }

        // Update index
        {
            let mut index = self.memory_index.write().await;
            index.add_episodic_memory(&memory)?;
        }

        // Extract concepts for semantic memory
        self.extract_concepts_from_episode(&memory).await?;

        info!("Stored episodic memory: {} for agent: {}", memory_id, agent_id);
        Ok(())
    }

    /// Store a semantic concept
    #[instrument(skip(self))]
    pub async fn store_semantic_concept(&self, concept: SemanticConcept) -> SemanticResult<()> {
        let concept_id = concept.concept_id.clone();
        let agent_id = concept.agent_id.clone();

        // Store in semantic memory
        {
            let mut semantic = self.semantic_memory.write().await;
            semantic.store_concept(concept.clone())?;
        }

        // Update index
        {
            let mut index = self.memory_index.write().await;
            index.add_semantic_concept(&concept)?;
        }

        info!("Stored semantic concept: {} for agent: {}", concept_id, agent_id);
        Ok(())
    }

    /// Update working memory
    #[instrument(skip(self))]
    pub async fn update_working_memory(&self, working_memory: WorkingMemorySet) -> SemanticResult<()> {
        let agent_id = working_memory.agent_id.clone();

        let mut working = self.working_memory.write().await;
        working.update_memory(working_memory)?;

        debug!("Updated working memory for agent: {}", agent_id);
        Ok(())
    }

    /// Query memories
    #[instrument(skip(self))]
    pub async fn query_memories(&self, query: MemoryQuery) -> SemanticResult<MemoryQueryResult> {
        let start_time = Instant::now();
        let query_id = Uuid::new_v4().to_string();

        let mut result = MemoryQueryResult {
            query_id,
            agent_id: query.agent_id.clone(),
            episodic_memories: vec![],
            semantic_concepts: vec![],
            working_context: None,
            total_results: 0,
            execution_time_ms: 0,
        };

        match query.query_type {
            MemoryQueryType::Episodic => {
                let episodic = self.episodic_memory.read().await;
                result.episodic_memories = episodic.query_memories(&query)?;
            }
            MemoryQueryType::Semantic => {
                let semantic = self.semantic_memory.read().await;
                result.semantic_concepts = semantic.query_concepts(&query)?;
            }
            MemoryQueryType::Working => {
                let working = self.working_memory.read().await;
                result.working_context = working.get_memory(&query.agent_id)?;
            }
            MemoryQueryType::Hybrid => {
                let episodic = self.episodic_memory.read().await;
                let semantic = self.semantic_memory.read().await;
                let working = self.working_memory.read().await;

                result.episodic_memories = episodic.query_memories(&query)?;
                result.semantic_concepts = semantic.query_concepts(&query)?;
                result.working_context = working.get_memory(&query.agent_id)?;
            }
            MemoryQueryType::Similarity => {
                // TODO: Implement similarity-based retrieval using embeddings
            }
            MemoryQueryType::Temporal => {
                // TODO: Implement temporal-based retrieval
            }
        }

        result.total_results = result.episodic_memories.len() + result.semantic_concepts.len();
        result.execution_time_ms = start_time.elapsed().as_millis() as u64;

        debug!("Memory query completed: {} results in {}ms", result.total_results, result.execution_time_ms);
        Ok(result)
    }

    /// Extract concepts from an episodic memory
    async fn extract_concepts_from_episode(&self, memory: &EpisodicMemory) -> SemanticResult<()> {
        // TODO: Implement concept extraction using NLP and pattern recognition
        // This would analyze the episodic memory content and extract semantic concepts
        Ok(())
    }

    /// Consolidate memories (move from working to long-term storage)
    #[instrument(skip(self))]
    pub async fn consolidate_memories(&self, agent_id: &str) -> SemanticResult<usize> {
        let mut consolidated_count = 0;

        // TODO: Implement memory consolidation logic
        // This would move important memories from working memory to long-term storage
        // and strengthen connections between related memories

        info!("Consolidated {} memories for agent: {}", consolidated_count, agent_id);
        Ok(consolidated_count)
    }

    /// Forget old or unimportant memories
    #[instrument(skip(self))]
    pub async fn forget_memories(&self, agent_id: &str) -> SemanticResult<usize> {
        let mut forgotten_count = 0;

        // Remove old episodic memories
        {
            let mut episodic = self.episodic_memory.write().await;
            forgotten_count += episodic.forget_old_memories(agent_id, &self.config)?;
        }

        // Remove unused semantic concepts
        {
            let mut semantic = self.semantic_memory.write().await;
            forgotten_count += semantic.forget_unused_concepts(agent_id, &self.config)?;
        }

        info!("Forgot {} memories for agent: {}", forgotten_count, agent_id);
        Ok(forgotten_count)
    }
}

impl EpisodicMemoryStore {
    fn new(config: MemoryConfig) -> Self {
        Self {
            memories: HashMap::new(),
            memory_timeline: HashMap::new(),
            config,
        }
    }

    fn store_memory(&mut self, memory: EpisodicMemory) -> SemanticResult<()> {
        let agent_id = memory.agent_id.clone();
        let memory_id = memory.memory_id.clone();

        // Add to memories
        self.memories.entry(agent_id.clone())
            .or_insert_with(Vec::new)
            .push(memory);

        // Add to timeline
        self.memory_timeline.entry(agent_id)
            .or_insert_with(VecDeque::new)
            .push_back(memory_id);

        Ok(())
    }

    fn query_memories(&self, query: &MemoryQuery) -> SemanticResult<Vec<EpisodicMemory>> {
        let agent_memories = self.memories.get(&query.agent_id)
            .map(|m| m.as_slice())
            .unwrap_or(&[]);

        let mut results: Vec<EpisodicMemory> = agent_memories
            .iter()
            .filter(|memory| self.matches_filters(memory, &query.filters))
            .cloned()
            .collect();

        // Sort by importance and recency
        results.sort_by(|a, b| {
            let importance_cmp = b.importance_score.partial_cmp(&a.importance_score)
                .unwrap_or(std::cmp::Ordering::Equal);
            if importance_cmp != std::cmp::Ordering::Equal {
                return importance_cmp;
            }
            b.timestamp.cmp(&a.timestamp)
        });

        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    fn matches_filters(&self, memory: &EpisodicMemory, filters: &MemoryFilters) -> bool {
        // Time range filter
        if let Some((start, end)) = &filters.time_range {
            if memory.timestamp < *start || memory.timestamp > *end {
                return false;
            }
        }

        // Tags filter
        if let Some(required_tags) = &filters.tags {
            if !required_tags.iter().any(|tag| memory.tags.contains(tag)) {
                return false;
            }
        }

        // Importance threshold filter
        if let Some(threshold) = filters.importance_threshold {
            if memory.importance_score < threshold {
                return false;
            }
        }

        // Episode type filter
        if let Some(episode_types) = &filters.episode_types {
            if !episode_types.contains(&memory.episode_type) {
                return false;
            }
        }

        true
    }

    fn forget_old_memories(&mut self, agent_id: &str, config: &MemoryConfig) -> SemanticResult<usize> {
        let cutoff_date = Utc::now() - chrono::Duration::days(config.memory_retention_days as i64);
        let mut forgotten_count = 0;

        if let Some(memories) = self.memories.get_mut(agent_id) {
            let initial_len = memories.len();
            memories.retain(|memory| {
                memory.timestamp > cutoff_date || memory.importance_score > 0.8
            });
            forgotten_count = initial_len - memories.len();
        }

        Ok(forgotten_count)
    }
}

impl SemanticMemoryStore {
    fn new(config: MemoryConfig) -> Self {
        Self {
            concepts: HashMap::new(),
            relationships: HashMap::new(),
            concept_index: HashMap::new(),
            config,
        }
    }

    fn store_concept(&mut self, concept: SemanticConcept) -> SemanticResult<()> {
        let agent_id = concept.agent_id.clone();
        let concept_id = concept.concept_id.clone();
        let concept_name = concept.name.clone();

        // Add to concepts
        self.concepts.entry(agent_id.clone())
            .or_insert_with(Vec::new)
            .push(concept);

        // Update index
        self.concept_index.entry(agent_id)
            .or_insert_with(HashMap::new)
            .insert(concept_name, concept_id);

        Ok(())
    }

    fn query_concepts(&self, query: &MemoryQuery) -> SemanticResult<Vec<SemanticConcept>> {
        let agent_concepts = self.concepts.get(&query.agent_id)
            .map(|c| c.as_slice())
            .unwrap_or(&[]);

        let mut results: Vec<SemanticConcept> = agent_concepts
            .iter()
            .filter(|concept| self.matches_concept_filters(concept, &query.filters))
            .cloned()
            .collect();

        // Sort by confidence and usage
        results.sort_by(|a, b| {
            let confidence_cmp = b.confidence.partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal);
            if confidence_cmp != std::cmp::Ordering::Equal {
                return confidence_cmp;
            }
            b.usage_count.cmp(&a.usage_count)
        });

        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    fn matches_concept_filters(&self, concept: &SemanticConcept, filters: &MemoryFilters) -> bool {
        // Concept type filter
        if let Some(concept_types) = &filters.concept_types {
            if !concept_types.contains(&concept.concept_type) {
                return false;
            }
        }

        // Content keywords filter
        if let Some(keywords) = &filters.content_keywords {
            let content = format!("{} {}", concept.name, concept.definition).to_lowercase();
            if !keywords.iter().any(|keyword| content.contains(&keyword.to_lowercase())) {
                return false;
            }
        }

        true
    }

    fn forget_unused_concepts(&mut self, agent_id: &str, config: &MemoryConfig) -> SemanticResult<usize> {
        let mut forgotten_count = 0;

        if let Some(concepts) = self.concepts.get_mut(agent_id) {
            let initial_len = concepts.len();
            concepts.retain(|concept| {
                concept.usage_count > 0 || concept.confidence > 0.5
            });
            forgotten_count = initial_len - concepts.len();
        }

        Ok(forgotten_count)
    }
}

impl WorkingMemoryStore {
    fn new(config: MemoryConfig) -> Self {
        Self {
            active_memories: HashMap::new(),
            config,
        }
    }

    fn update_memory(&mut self, memory: WorkingMemorySet) -> SemanticResult<()> {
        self.active_memories.insert(memory.agent_id.clone(), memory);
        Ok(())
    }

    fn get_memory(&self, agent_id: &str) -> SemanticResult<Option<WorkingMemorySet>> {
        Ok(self.active_memories.get(agent_id).cloned())
    }
}

impl MemoryIndex {
    fn new() -> Self {
        Self {
            temporal_index: HashMap::new(),
            content_index: HashMap::new(),
            similarity_index: HashMap::new(),
            tag_index: HashMap::new(),
        }
    }

    fn add_episodic_memory(&mut self, memory: &EpisodicMemory) -> SemanticResult<()> {
        let agent_id = &memory.agent_id;
        let memory_id = &memory.memory_id;

        // Add to temporal index
        self.temporal_index.entry(agent_id.clone())
            .or_insert_with(Vec::new)
            .push((memory.timestamp, memory_id.clone()));

        // Add to tag index
        for tag in &memory.tags {
            self.tag_index.entry(agent_id.clone())
                .or_insert_with(HashMap::new)
                .entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(memory_id.clone());
        }

        Ok(())
    }

    fn add_semantic_concept(&mut self, concept: &SemanticConcept) -> SemanticResult<()> {
        let agent_id = &concept.agent_id;
        let concept_id = &concept.concept_id;

        // Add to content index
        let keywords: Vec<String> = concept.name.split_whitespace()
            .chain(concept.definition.split_whitespace())
            .map(|s| s.to_lowercase())
            .collect();

        for keyword in keywords {
            self.content_index.entry(agent_id.clone())
                .or_insert_with(HashMap::new)
                .entry(keyword)
                .or_insert_with(Vec::new)
                .push(concept_id.clone());
        }

        // Add to similarity index if embedding exists
        if let Some(embedding) = &concept.embedding {
            self.similarity_index.entry(agent_id.clone())
                .or_insert_with(Vec::new)
                .push((concept_id.clone(), embedding.clone()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_episodic_memory_storage() {
        let memory_manager = AgentMemoryManager::new(MemoryConfig::default());
        
        let memory = EpisodicMemory {
            memory_id: Uuid::new_v4().to_string(),
            agent_id: "test_agent".to_string(),
            episode_type: EpisodeType::TaskExecution,
            timestamp: Utc::now(),
            duration: Some(Duration::from_secs(300)),
            context: EpisodicContext {
                location: Some("workspace".to_string()),
                other_agents: vec![],
                task_id: Some("task_123".to_string()),
                session_id: Some("session_456".to_string()),
                environmental_factors: HashMap::new(),
            },
            content: EpisodicContent {
                description: "Completed data analysis task".to_string(),
                actions_taken: vec![],
                observations: vec![],
                outcomes: vec![],
                lessons_learned: vec!["Data quality is crucial".to_string()],
                raw_data: None,
            },
            emotional_valence: 0.8,
            importance_score: 0.9,
            access_count: 0,
            last_accessed: Utc::now(),
            related_memories: vec![],
            tags: vec!["analysis".to_string(), "success".to_string()],
        };
        
        assert!(memory_manager.store_episodic_memory(memory).await.is_ok());
    }

    #[tokio::test]
    async fn test_semantic_concept_storage() {
        let memory_manager = AgentMemoryManager::new(MemoryConfig::default());
        
        let concept = SemanticConcept {
            concept_id: Uuid::new_v4().to_string(),
            agent_id: "test_agent".to_string(),
            name: "Data Quality".to_string(),
            concept_type: ConceptType::Rule,
            definition: "The accuracy and reliability of data".to_string(),
            properties: HashMap::new(),
            examples: vec!["Clean data leads to better results".to_string()],
            confidence: 0.9,
            created_at: Utc::now(),
            last_updated: Utc::now(),
            usage_count: 1,
            embedding: None,
            source_memories: vec![],
        };
        
        assert!(memory_manager.store_semantic_concept(concept).await.is_ok());
    }

    #[tokio::test]
    async fn test_memory_query() {
        let memory_manager = AgentMemoryManager::new(MemoryConfig::default());
        
        let query = MemoryQuery {
            agent_id: "test_agent".to_string(),
            query_type: MemoryQueryType::Episodic,
            filters: MemoryFilters {
                time_range: None,
                tags: Some(vec!["analysis".to_string()]),
                content_keywords: None,
                importance_threshold: Some(0.5),
                episode_types: None,
                concept_types: None,
            },
            limit: Some(10),
            similarity_threshold: None,
        };
        
        let result = memory_manager.query_memories(query).await.unwrap();
        assert_eq!(result.agent_id, "test_agent");
    }
}