//! Agent Registry Module for agent discovery and lifecycle management.
//!
//! Based on Microsoft Multi-Agent Reference Architecture:
//! - Agent Registry serves as centralized directory for agent management
//! - Enables dynamic discovery of agent capabilities
//! - Tracks agent operational status
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │              AgentRegistry                             │
//! │  ┌─────────────────────────────────────────────┐   │
//! │  │ agents: HashMap<AgentId, RegisteredAgent>    │   │
//! │  └─────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────┘
//!                         │
//!         ┌───────────────┼───────────────┐
//!         ▼               ▼               ▼
//!    ┌─────────┐     ┌─────────┐     ┌─────────┐
//!    │Research │     │  Coder  │     │  QA     │
//!    │ Agent  │     │  Agent  │     │  Agent  │
//!    └─────────┘     └─────────┘     └─────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::utils::current_timestamp;

/// Agent identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Agent operational status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    /// Agent is idle and ready to accept tasks
    Idle,
    /// Agent is currently processing a task
    Busy,
    /// Agent is not available
    Offline,
    /// Agent is initializing
    Starting,
    /// Agent encountered an error
    Error,
}

impl AgentStatus {
    pub fn is_available(&self) -> bool {
        matches!(self, AgentStatus::Idle | AgentStatus::Busy)
    }
}

/// Capability tags for agent discovery
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Capability {
    /// Capability name (e.g., "web_search", "code_generation")
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Version of the capability
    pub version: Option<String>,
}

/// Agent metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    /// Agent name
    pub name: String,
    /// Agent description
    pub description: String,
    /// Version string
    pub version: String,
    /// Owner/creator
    pub owner: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom metadata
    #[serde(default)]
    pub extras: HashMap<String, String>,
}

/// A registered agent entry
#[derive(Debug, Clone)]
pub struct RegisteredAgent {
    /// Unique agent ID
    pub id: AgentId,
    /// Agent metadata
    pub metadata: AgentMetadata,
    /// Agent status
    pub status: AgentStatus,
    /// Capabilities offered by this agent
    pub capabilities: Vec<Capability>,
    /// Current workload (number of tasks)
    pub workload: u32,
    /// Maximum concurrent tasks
    pub max_concurrent: u32,
    /// Last heartbeat timestamp
    pub last_heartbeat: u64,
    /// Created at timestamp
    pub created_at: u64,
}

impl RegisteredAgent {
    /// Check if agent can accept more tasks
    pub fn can_accept_task(&self) -> bool {
        self.status.is_available() && self.workload < self.max_concurrent
    }

    /// Get agent availability score (0.0 - 1.0)
    pub fn availability_score(&self) -> f32 {
        if !self.status.is_available() {
            return 0.0;
        }
        let capacity_ratio = 1.0 - (self.workload as f32 / self.max_concurrent as f32);
        capacity_ratio.max(0.0)
    }
}

/// Registry statistics
#[derive(Debug, Clone, Default)]
pub struct RegistryStats {
    pub total_agents: usize,
    pub available_agents: usize,
    pub busy_agents: usize,
    pub offline_agents: usize,
    pub agents_by_capability: HashMap<String, usize>,
}

/// Agent Registry - centralized directory for agent management
#[derive(Debug, Clone)]
pub struct AgentRegistry {
    /// All registered agents
    agents: Arc<RwLock<HashMap<AgentId, RegisteredAgent>>>,
    /// Capability index: capability -> agent IDs
    capability_index: Arc<RwLock<HashMap<String, HashSet<AgentId>>>>,
    /// Tag index: tag -> agent IDs
    tag_index: Arc<RwLock<HashMap<String, HashSet<AgentId>>>>,
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentRegistry {
    /// Create a new agent registry
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            capability_index: Arc::new(RwLock::new(HashMap::new())),
            tag_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new agent
    pub async fn register(&self, agent: RegisteredAgent) -> Result<(), RegistryError> {
        let id = agent.id.clone();
        let capabilities = agent.capabilities.clone();
        let tags = agent.metadata.tags.clone();

        // Check for duplicate
        {
            let agents = self.agents.read().await;
            if agents.contains_key(&id) {
                return Err(RegistryError::AlreadyExists(id.to_string()));
            }
        }

        // Insert agent
        {
            let mut agents = self.agents.write().await;
            agents.insert(id.clone(), agent);
        }

        // Update capability index
        {
            let mut cap_index = self.capability_index.write().await;
            for cap in &capabilities {
                cap_index.entry(cap.name.clone()).or_default().insert(id.clone());
            }
        }

        // Update tag index
        {
            let mut tag_index = self.tag_index.write().await;
            for tag in &tags {
                tag_index.entry(tag.clone()).or_default().insert(id.clone());
            }
        }

        Ok(())
    }

    /// Unregister an agent
    pub async fn unregister(&self, id: &AgentId) -> Option<RegisteredAgent> {
        // Get agent info first
        let agent = {
            let agents = self.agents.read().await;
            agents.get(id).cloned()
        };

        if let Some(agent) = agent {
            // Remove from main registry
            let removed = {
                let mut agents = self.agents.write().await;
                agents.remove(id)
            };

            if removed.is_some() {
                // Update capability index
                {
                    let mut cap_index = self.capability_index.write().await;
                    for cap in &agent.capabilities {
                        if let Some(ids) = cap_index.get_mut(&cap.name) {
                            ids.remove(id);
                            if ids.is_empty() {
                                cap_index.remove(&cap.name);
                            }
                        }
                    }
                }

                // Update tag index
                {
                    let mut tag_index = self.tag_index.write().await;
                    for tag in &agent.metadata.tags {
                        if let Some(ids) = tag_index.get_mut(tag) {
                            ids.remove(id);
                            if ids.is_empty() {
                                tag_index.remove(tag);
                            }
                        }
                    }
                }
            }

            removed
        } else {
            None
        }
    }

    /// Get an agent by ID
    pub async fn get(&self, id: &AgentId) -> Option<RegisteredAgent> {
        let agents = self.agents.read().await;
        agents.get(id).cloned()
    }

    /// List all registered agents
    pub async fn list(&self) -> Vec<RegisteredAgent> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }

    /// List agents by status
    pub async fn list_by_status(&self, status: AgentStatus) -> Vec<RegisteredAgent> {
        let agents = self.agents.read().await;
        agents.values().filter(|a| a.status == status).cloned().collect()
    }

    /// Find agents by capability
    pub async fn find_by_capability(&self, capability: &str) -> Vec<RegisteredAgent> {
        let ids = {
            let cap_index = self.capability_index.read().await;
            cap_index.get(capability).cloned()
        };

        if let Some(ids) = ids {
            let agents = self.agents.read().await;
            ids.iter()
                .filter_map(|id| agents.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Find agents by tag
    pub async fn find_by_tag(&self, tag: &str) -> Vec<RegisteredAgent> {
        let ids = {
            let tag_index = self.tag_index.read().await;
            tag_index.get(tag).cloned()
        };

        if let Some(ids) = ids {
            let agents = self.agents.read().await;
            ids.iter()
                .filter_map(|id| agents.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Find available agents for a capability
    pub async fn find_available(&self, capability: Option<&str>) -> Vec<RegisteredAgent> {
        let agents = self.agents.read().await;

        let filtered: Box<dyn Iterator<Item = (&AgentId, &RegisteredAgent)>> = if let Some(cap) = capability {
            let cap_index = self.capability_index.read().await;
            if let Some(ids) = cap_index.get(cap) {
                Box::new(ids.iter().filter_map(|id| agents.get_key_value(id)))
            } else {
                return Vec::new();
            }
        } else {
            Box::new(agents.iter())
        };

        filtered
            .filter(|(_, agent)| agent.can_accept_task())
            .map(|(_, agent)| agent.clone())
            .collect()
    }

    /// Update agent status
    pub async fn update_status(&self, id: &AgentId, status: AgentStatus) -> bool {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(id) {
            agent.status = status;
            agent.last_heartbeat = current_timestamp();
            true
        } else {
            false
        }
    }

    /// Update agent workload
    pub async fn update_workload(&self, id: &AgentId, delta: i32) -> bool {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(id) {
            let new_workload = (agent.workload as i32 + delta).max(0) as u32;
            agent.workload = new_workload.min(agent.max_concurrent * 2); // Allow overflow tracking
            agent.last_heartbeat = current_timestamp();
            true
        } else {
            false
        }
    }

    /// Send heartbeat for an agent
    pub async fn heartbeat(&self, id: &AgentId) -> bool {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(id) {
            agent.last_heartbeat = current_timestamp();
            if agent.status == AgentStatus::Offline {
                agent.status = AgentStatus::Idle;
            }
            true
        } else {
            false
        }
    }

    /// Get registry statistics
    pub async fn stats(&self) -> RegistryStats {
        let agents = self.agents.read().await;
        let mut stats = RegistryStats::default();
        stats.total_agents = agents.len();

        let cap_index = self.capability_index.read().await;
        stats.agents_by_capability = cap_index
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect();

        drop(cap_index);

        for agent in agents.values() {
            match agent.status {
                AgentStatus::Idle | AgentStatus::Starting => stats.available_agents += 1,
                AgentStatus::Busy => stats.busy_agents += 1,
                AgentStatus::Offline | AgentStatus::Error => stats.offline_agents += 1,
            }
        }

        stats
    }

    /// Get all registered capability names
    pub async fn capabilities(&self) -> Vec<String> {
        let cap_index = self.capability_index.read().await;
        cap_index.keys().cloned().collect()
    }

    /// Get all registered tags
    pub async fn tags(&self) -> Vec<String> {
        let tag_index = self.tag_index.read().await;
        tag_index.keys().cloned().collect()
    }

    /// Check if an agent exists
    pub async fn contains(&self, id: &AgentId) -> bool {
        let agents = self.agents.read().await;
        agents.contains_key(id)
    }

    /// Get total agent count
    pub async fn len(&self) -> usize {
        let agents = self.agents.read().await;
        agents.len()
    }

    /// Check if registry is empty
    pub async fn is_empty(&self) -> bool {
        let agents = self.agents.read().await;
        agents.is_empty()
    }

    /// Clear all agents
    pub async fn clear(&self) {
        let mut agents = self.agents.write().await;
        agents.clear();
        drop(agents);

        let mut cap_index = self.capability_index.write().await;
        cap_index.clear();
        drop(cap_index);

        let mut tag_index = self.tag_index.write().await;
        tag_index.clear();
    }
}

/// Registry errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegistryError {
    AlreadyExists(String),
    NotFound(String),
    InvalidState(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::AlreadyExists(id) => write!(f, "Agent already registered: {}", id),
            RegistryError::NotFound(id) => write!(f, "Agent not found: {}", id),
            RegistryError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
        }
    }
}

impl std::error::Error for RegistryError {}

// =============================================================================
// Builder for RegisteredAgent
// =============================================================================

/// Builder for creating RegisteredAgent entries
pub struct RegisteredAgentBuilder {
    id: Option<AgentId>,
    metadata: Option<AgentMetadata>,
    capabilities: Vec<Capability>,
    status: AgentStatus,
    max_concurrent: u32,
}

impl RegisteredAgentBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            id: None,
            metadata: None,
            capabilities: Vec::new(),
            status: AgentStatus::Idle,
            max_concurrent: 5,
        }
    }

    /// Set the agent ID
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(AgentId(id.into()));
        self
    }

    /// Set agent name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.metadata = Some(self.metadata.take().map(|m| AgentMetadata {
            name,
            ..m
        }).unwrap_or_else(|| AgentMetadata {
            name,
            description: String::new(),
            version: "1.0.0".to_string(),
            owner: None,
            tags: Vec::new(),
            extras: HashMap::new(),
        }));
        self
    }

    /// Set agent description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        let desc = desc.into();
        self.metadata = Some(self.metadata.take().map(|m| AgentMetadata {
            description: desc,
            ..m
        }).unwrap_or_else(|| AgentMetadata {
            name: "unnamed".to_string(),
            description: desc,
            version: "1.0.0".to_string(),
            owner: None,
            tags: Vec::new(),
            extras: HashMap::new(),
        }));
        self
    }

    /// Set agent version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        let version = version.into();
        self.metadata = Some(self.metadata.take().map(|m| AgentMetadata {
            version,
            ..m
        }).unwrap_or_else(|| AgentMetadata {
            name: "unnamed".to_string(),
            description: String::new(),
            version,
            owner: None,
            tags: Vec::new(),
            extras: HashMap::new(),
        }));
        self
    }

    /// Add a capability
    pub fn capability(mut self, name: impl Into<String>) -> Self {
        self.capabilities.push(Capability {
            name: name.into(),
            description: None,
            version: None,
        });
        self
    }

    /// Add a capability with description
    pub fn capability_with_desc(
        mut self,
        name: impl Into<String>,
        desc: impl Into<String>,
    ) -> Self {
        self.capabilities.push(Capability {
            name: name.into(),
            description: Some(desc.into()),
            version: None,
        });
        self
    }

    /// Add a tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        if let Some(ref mut m) = self.metadata {
            m.tags.push(tag.into());
        }
        self
    }

    /// Set max concurrent tasks
    pub fn max_concurrent(mut self, max: u32) -> Self {
        self.max_concurrent = max;
        self
    }

    /// Set initial status
    pub fn status(mut self, status: AgentStatus) -> Self {
        self.status = status;
        self
    }

    /// Build the RegisteredAgent
    pub fn build(self) -> Result<RegisteredAgent, RegistryError> {
        let id = self.id.ok_or_else(|| RegistryError::InvalidState("Agent ID is required".to_string()))?;
        let metadata = self.metadata.ok_or_else(|| RegistryError::InvalidState("Agent name is required".to_string()))?;

        Ok(RegisteredAgent {
            id,
            metadata,
            status: self.status,
            capabilities: self.capabilities,
            workload: 0,
            max_concurrent: self.max_concurrent,
            last_heartbeat: current_timestamp(),
            created_at: current_timestamp(),
        })
    }
}

impl Default for RegisteredAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Global Registry (Singleton)
// =============================================================================

use std::sync::OnceLock;

static GLOBAL_REGISTRY: OnceLock<AgentRegistry> = OnceLock::new();

/// Get the global agent registry instance
pub fn global_registry() -> &'static AgentRegistry {
    GLOBAL_REGISTRY.get_or_init(|| AgentRegistry::new())
}

// =============================================================================
// Utilities
// =============================================================================

// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_get() {
        let registry = AgentRegistry::new();

        let agent = RegisteredAgentBuilder::new()
            .id("agent-1")
            .name("Research Agent")
            .description("Searches the web for information")
            .capability("web_search")
            .capability("web_scrape")
            .tag("research")
            .tag("info_gathering")
            .build()
            .unwrap();

        registry.register(agent).await.unwrap();

        let retrieved = registry.get(&AgentId::new("agent-1")).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().metadata.name, "Research Agent");
    }

    #[tokio::test]
    async fn test_find_by_capability() {
        let registry = AgentRegistry::new();

        // Register agents with different capabilities
        registry.register(
            RegisteredAgentBuilder::new()
                .id("researcher")
                .name("Researcher")
                .capability("web_search")
                .build()
                .unwrap(),
        )
        .await
        .unwrap();

        registry.register(
            RegisteredAgentBuilder::new()
                .id("coder")
                .name("Coder")
                .capability("code_generation")
                .build()
                .unwrap(),
        )
        .await
        .unwrap();

        let searchers = registry.find_by_capability("web_search").await;
        assert_eq!(searchers.len(), 1);
        assert_eq!(searchers[0].id.0, "researcher");

        let coders = registry.find_by_capability("code_generation").await;
        assert_eq!(coders.len(), 1);
        assert_eq!(coders[0].id.0, "coder");
    }

    #[tokio::test]
    async fn test_find_available() {
        let registry = AgentRegistry::new();

        registry.register(
            RegisteredAgentBuilder::new()
                .id("idle-agent")
                .name("Idle Agent")
                .status(AgentStatus::Idle)
                .build()
                .unwrap(),
        )
        .await
        .unwrap();

        registry.register(
            RegisteredAgentBuilder::new()
                .id("busy-agent")
                .name("Busy Agent")
                .status(AgentStatus::Busy)
                .build()
                .unwrap(),
        )
        .await
        .unwrap();

        let available = registry.find_available(None).await;
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].id.0, "idle-agent");
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = AgentRegistry::new();

        registry.register(
            RegisteredAgentBuilder::new()
                .id("temp-agent")
                .name("Temp Agent")
                .build()
                .unwrap(),
        )
        .await
        .unwrap();

        assert!(registry.contains(&AgentId::new("temp-agent")).await);

        let removed = registry.unregister(&AgentId::new("temp-agent")).await;
        assert!(removed.is_some());

        assert!(!registry.contains(&AgentId::new("temp-agent")).await);
    }

    #[tokio::test]
    async fn test_stats() {
        let registry = AgentRegistry::new();

        registry.register(
            RegisteredAgentBuilder::new()
                .id("a1")
                .name("Agent 1")
                .status(AgentStatus::Idle)
                .build()
                .unwrap(),
        )
        .await
        .unwrap();

        registry.register(
            RegisteredAgentBuilder::new()
                .id("a2")
                .name("Agent 2")
                .status(AgentStatus::Busy)
                .build()
                .unwrap(),
        )
        .await
        .unwrap();

        let stats = registry.stats().await;
        assert_eq!(stats.total_agents, 2);
        assert_eq!(stats.available_agents, 1);
        assert_eq!(stats.busy_agents, 1);
    }

    #[tokio::test]
    async fn test_workload_tracking() {
        let registry = AgentRegistry::new();

        registry.register(
            RegisteredAgentBuilder::new()
                .id("worker")
                .name("Worker")
                .max_concurrent(3)
                .build()
                .unwrap(),
        )
        .await
        .unwrap();

        // Increment workload
        registry.update_workload(&AgentId::new("worker"), 1).await;
        {
            let agent = registry.get(&AgentId::new("worker")).await.unwrap();
            assert_eq!(agent.workload, 1);
        }

        // Decrement workload
        registry.update_workload(&AgentId::new("worker"), -1).await;
        {
            let agent = registry.get(&AgentId::new("worker")).await.unwrap();
            assert_eq!(agent.workload, 0);
        }
    }

    #[tokio::test]
    async fn test_availability_score() {
        let registry = AgentRegistry::new();

        registry.register(
            RegisteredAgentBuilder::new()
                .id("available")
                .name("Available")
                .status(AgentStatus::Idle)
                .max_concurrent(4)
                .build()
                .unwrap(),
        )
        .await
        .unwrap();

        registry.update_workload(&AgentId::new("available"), 2).await;

        let agent = registry.get(&AgentId::new("available")).await.unwrap();
        // With 2/4 workload, score should be 0.5
        assert!((agent.availability_score() - 0.5).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_global_registry() {
        let registry = global_registry();

        // Register through global
        registry
            .register(
                RegisteredAgentBuilder::new()
                    .id("global-test")
                    .name("Global Test")
                    .build()
                    .unwrap(),
            )
            .await
            .unwrap();

        // Get through global
        let agent = registry.get(&AgentId::new("global-test")).await;
        assert!(agent.is_some());
    }
}
