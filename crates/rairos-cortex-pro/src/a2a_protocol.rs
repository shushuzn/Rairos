//! Agent-to-Agent (A2A) protocol implementation for multi-agent communication.
//!
//! Based on research from:
//! - arXiv:2507.21105 (AgentMaster) - A2A + MCP integration
//! - arXiv:2603.08852 (LDP) - Identity-aware protocols
//! - arXiv:2604.12213 (MMA2A) - Modality-native routing
//! - arXiv:2505.02279 (Survey) - Protocol comparison
//!
//! ## A2A Protocol Overview
//!
//! ```text
//! Agent A                        Agent B
//!   │                               │
//!   │──── Task Request ───────────▶│
//!   │                               │
//!   │◀─── Agent Card (capabilities)│
//!   │                               │
//!   │──── Offer/Proposal ─────────▶│
//!   │                               │
//!   │◀─── Accept/Reject ──────────│
//!   │                               │
//!   │──── Result Delivery ─────────▶│
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

use crate::utils::uuid_simple;

/// A2A message types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum A2AMessageType {
    /// Request task execution
    TaskRequest,
    /// Submit proposal/bid
    Proposal,
    /// Accept proposal
    Accept,
    /// Reject proposal
    Reject,
    /// Return result
    Result,
    /// Error response
    Error,
    /// Heartbeat/ping
    Ping,
    /// Pong response
    Pong,
}

/// A2A message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    /// Message ID
    pub id: String,
    /// Message type
    pub msg_type: A2AMessageType,
    /// Sender agent ID
    pub from: String,
    /// Receiver agent ID (empty for broadcast)
    pub to: String,
    /// Task ID this message relates to
    pub task_id: Option<String>,
    /// Message payload (JSON)
    pub payload: serde_json::Value,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Reply-to message ID
    pub in_reply_to: Option<String>,
}

impl A2AMessage {
    /// Create a new task request
    pub fn task_request(from: &str, to: &str, task: &A2ATask) -> Self {
        Self {
            id: uuid_simple(),
            msg_type: A2AMessageType::TaskRequest,
            from: from.to_string(),
            to: to.to_string(),
            task_id: Some(task.id.clone()),
            payload: serde_json::json!(task),
            timestamp: Utc::now(),
            in_reply_to: None,
        }
    }

    /// Create a proposal message
    pub fn proposal(from: &str, to: &str, task_id: &str, proposal: &TaskProposal) -> Self {
        Self {
            id: uuid_simple(),
            msg_type: A2AMessageType::Proposal,
            from: from.to_string(),
            to: to.to_string(),
            task_id: Some(task_id.to_string()),
            payload: serde_json::json!(proposal),
            timestamp: Utc::now(),
            in_reply_to: None,
        }
    }

    /// Create a result message
    pub fn result(from: &str, to: &str, task_id: &str, result: &TaskResult) -> Self {
        Self {
            id: uuid_simple(),
            msg_type: A2AMessageType::Result,
            from: from.to_string(),
            to: to.to_string(),
            task_id: Some(task_id.to_string()),
            payload: serde_json::json!(result),
            timestamp: Utc::now(),
            in_reply_to: None,
        }
    }

    /// Check if this is a response to another message
    pub fn is_response_to(&self, other: &A2AMessage) -> bool {
        self.in_reply_to.as_ref() == Some(&other.id)
    }
}

/// Task definition for A2A communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATask {
    /// Task ID
    pub id: String,
    /// Task description
    pub description: String,
    /// Required capabilities
    pub required_capabilities: Vec<String>,
    /// Task priority (1-10, higher = more urgent)
    pub priority: u8,
    /// Task deadline (if any)
    pub deadline: Option<DateTime<Utc>>,
    /// Input parameters
    pub input: HashMap<String, serde_json::Value>,
}

/// Proposal from an agent for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProposal {
    /// Agent ID submitting proposal
    pub agent_id: String,
    /// Estimated completion time (ms)
    pub estimated_time_ms: u64,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Proposed approach
    pub approach: String,
    /// Resource requirements
    pub resources: HashMap<String, serde_json::Value>,
}

/// Result from task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task ID
    pub task_id: String,
    /// Whether successful
    pub success: bool,
    /// Result data
    pub data: Option<serde_json::Value>,
    /// Error message
    pub error: Option<String>,
    /// Execution time (ms)
    pub execution_time_ms: u64,
}

/// Agent card for capability advertisement (A2A protocol)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    /// Agent ID
    pub agent_id: String,
    /// Agent name
    pub name: String,
    /// Agent description
    pub description: String,
    /// Agent version
    pub version: String,
    /// Supported capabilities
    pub capabilities: Vec<AgentCapability>,
    /// Endpoint URL
    pub endpoint: Option<String>,
    /// Authentication method
    pub auth_method: Option<String>,
    /// Provider organization
    pub provider: Option<String>,
    // =====================================================================
    // Enhanced fields based on Multi-Agent Protocol research (MPAC, MMP)
    // =====================================================================
    /// Supported communication protocols (e.g., "A2A", "MCP", "custom")
    pub supported_protocols: Vec<String>,
    /// Preferred communication pattern
    pub communication_pattern: CommunicationPattern,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Historical reliability score (0.0 - 1.0)
    pub reliability_score: f32,
    /// Maximum concurrent tasks supported
    pub max_concurrent_tasks: u32,
    /// Supported tool categories (for tool-using agents)
    pub supported_tool_categories: Vec<String>,
    /// Agent role in multi-agent system
    pub agent_role: AgentRole,
}

/// Communication patterns for agents
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CommunicationPattern {
    /// Synchronous request-response
    Synchronous,
    /// Asynchronous messaging
    Asynchronous,
    /// Publish-subscribe
    PubSub,
    /// Streaming responses
    Streaming,
    /// Hybrid approach
    Hybrid,
}

/// Agent roles in multi-agent systems
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AgentRole {
    /// Orchestrator/planner agent
    Orchestrator,
    /// Executor/worker agent
    Executor,
    /// Critic/reviewer agent
    Critic,
    /// Tool specialist agent
    ToolSpecialist,
    /// General-purpose agent
    General,
}

/// A capability that an agent provides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    /// Capability name
    pub name: String,
    /// Capability description
    pub description: String,
    /// Input schema (JSON Schema)
    pub input_schema: serde_json::Value,
    /// Output schema (JSON Schema)
    pub output_schema: serde_json::Value,
    /// Whether this is a default capability
    pub is_default: bool,
}

/// A2A Protocol handler for agent communication
pub struct A2AProtocol {
    /// Local agent ID
    agent_id: String,
    /// Agent card
    agent_card: AgentCard,
    /// Known agent cards
    known_agents: HashMap<String, AgentCard>,
    /// Message history
    message_history: Vec<A2AMessage>,
    /// Pending tasks
    pending_tasks: HashMap<String, A2ATask>,
}

impl A2AProtocol {
    /// Create a new A2A protocol handler
    pub fn new(agent_id: &str, name: &str, description: &str) -> Self {
        let agent_card = AgentCard {
            agent_id: agent_id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: vec![],
            endpoint: None,
            auth_method: None,
            provider: None,
            supported_protocols: vec!["A2A".to_string()],
            communication_pattern: CommunicationPattern::Synchronous,
            avg_response_time_ms: 1000.0,
            reliability_score: 0.9,
            max_concurrent_tasks: 5,
            supported_tool_categories: vec![],
            agent_role: AgentRole::General,
        };

        Self {
            agent_id: agent_id.to_string(),
            agent_card,
            known_agents: HashMap::new(),
            message_history: Vec::new(),
            pending_tasks: HashMap::new(),
        }
    }

    /// Get the agent card
    pub fn agent_card(&self) -> &AgentCard {
        &self.agent_card
    }

    /// Add a capability to this agent
    pub fn add_capability(&mut self, capability: AgentCapability) {
        self.agent_card.capabilities.push(capability);
    }

    /// Register a known agent
    pub fn register_agent(&mut self, card: AgentCard) {
        self.known_agents.insert(card.agent_id.clone(), card);
    }

    /// Get an agent's card
    pub fn get_agent_card(&self, agent_id: &str) -> Option<&AgentCard> {
        self.known_agents.get(agent_id)
    }

    /// Send a message (record it in history)
    pub fn send_message(&mut self, msg: A2AMessage) {
        self.message_history.push(msg);
    }

    /// Receive a message
    pub fn receive_message(&mut self, msg: A2AMessage) -> Option<A2AMessage> {
        self.message_history.push(msg.clone());

        // Auto-respond to ping
        if msg.msg_type == A2AMessageType::Ping {
            return Some(A2AMessage {
                id: uuid_simple(),
                msg_type: A2AMessageType::Pong,
                from: self.agent_id.clone(),
                to: msg.from.clone(),
                task_id: None,
                payload: serde_json::json!({}),
                timestamp: Utc::now(),
                in_reply_to: Some(msg.id),
            });
        }

        None
    }

    /// Find agents that can handle a task
    pub fn find_agents_for_task(&self, task: &A2ATask) -> Vec<&AgentCard> {
        self.known_agents
            .values()
            .filter(|card| {
                task.required_capabilities.iter().all(|req| {
                    card.capabilities.iter().any(|cap| &cap.name == req)
                })
            })
            .collect()
    }

    /// Get message history
    pub fn message_history(&self) -> &[A2AMessage] {
        &self.message_history
    }

    /// Get pending task count
    pub fn pending_task_count(&self) -> usize {
        self.pending_tasks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_task_request() {
        let task = A2ATask {
            id: "task-1".to_string(),
            description: "Analyze materials".to_string(),
            required_capabilities: vec!["analysis".to_string()],
            priority: 5,
            deadline: None,
            input: HashMap::new(),
        };

        let msg = A2AMessage::task_request("agent-a", "agent-b", &task);

        assert_eq!(msg.msg_type, A2AMessageType::TaskRequest);
        assert_eq!(msg.from, "agent-a");
        assert_eq!(msg.to, "agent-b");
        assert_eq!(msg.task_id, Some("task-1".to_string()));
    }

    #[test]
    fn test_agent_card() {
        let mut protocol = A2AProtocol::new("agent-1", "Test Agent", "A test agent");

        protocol.add_capability(AgentCapability {
            name: "analysis".to_string(),
            description: "Performs analysis".to_string(),
            input_schema: serde_json::json!({}),
            output_schema: serde_json::json!({}),
            is_default: true,
        });

        assert_eq!(protocol.agent_card().capabilities.len(), 1);
    }

    #[test]
    fn test_register_agent() {
        let mut protocol = A2AProtocol::new("agent-1", "Agent 1", "First agent");

        let card = AgentCard {
            agent_id: "agent-2".to_string(),
            name: "Agent 2".to_string(),
            description: "Second agent".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            endpoint: None,
            auth_method: None,
            provider: None,
            supported_protocols: vec!["A2A".to_string()],
            communication_pattern: CommunicationPattern::Synchronous,
            avg_response_time_ms: 1000.0,
            reliability_score: 0.9,
            max_concurrent_tasks: 5,
            supported_tool_categories: vec![],
            agent_role: AgentRole::Executor,
        };

        protocol.register_agent(card);
        assert!(protocol.get_agent_card("agent-2").is_some());
    }

    #[test]
    fn test_find_agents_for_task() {
        let mut protocol = A2AProtocol::new("agent-1", "Agent 1", "First agent");

        // Add agent with analysis capability
        let mut card1 = AgentCard {
            agent_id: "analyst".to_string(),
            name: "Analyst".to_string(),
            description: "Analysis agent".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![
                AgentCapability {
                    name: "analysis".to_string(),
                    description: "Analysis".to_string(),
                    input_schema: serde_json::json!({}),
                    output_schema: serde_json::json!({}),
                    is_default: false,
                },
            ],
            endpoint: None,
            auth_method: None,
            provider: None,
            supported_protocols: vec!["A2A".to_string()],
            communication_pattern: CommunicationPattern::Synchronous,
            avg_response_time_ms: 1000.0,
            reliability_score: 0.9,
            max_concurrent_tasks: 5,
            supported_tool_categories: vec![],
            agent_role: AgentRole::ToolSpecialist,
        };
        protocol.register_agent(card1);

        // Add agent with planning capability
        let mut card2 = AgentCard {
            agent_id: "planner".to_string(),
            name: "Planner".to_string(),
            description: "Planning agent".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![
                AgentCapability {
                    name: "planning".to_string(),
                    description: "Planning".to_string(),
                    input_schema: serde_json::json!({}),
                    output_schema: serde_json::json!({}),
                    is_default: false,
                },
            ],
            endpoint: None,
            auth_method: None,
            provider: None,
            supported_protocols: vec!["A2A".to_string()],
            communication_pattern: CommunicationPattern::Synchronous,
            avg_response_time_ms: 1500.0,
            reliability_score: 0.85,
            max_concurrent_tasks: 3,
            supported_tool_categories: vec![],
            agent_role: AgentRole::Orchestrator,
        };
        protocol.register_agent(card2);

        // Find agents for analysis task
        let task = A2ATask {
            id: "task-1".to_string(),
            description: "Analyze data".to_string(),
            required_capabilities: vec!["analysis".to_string()],
            priority: 5,
            deadline: None,
            input: HashMap::new(),
        };

        let agents = protocol.find_agents_for_task(&task);
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].agent_id, "analyst");
    }

    #[test]
    fn test_ping_pong() {
        let mut protocol = A2AProtocol::new("agent-b", "Agent B", "Second agent");

        let ping = A2AMessage {
            id: "ping-1".to_string(),
            msg_type: A2AMessageType::Ping,
            from: "agent-a".to_string(),
            to: "agent-b".to_string(),
            task_id: None,
            payload: serde_json::json!({}),
            timestamp: Utc::now(),
            in_reply_to: None,
        };

        let pong = protocol.receive_message(ping);
        assert!(pong.is_some());
        assert_eq!(pong.unwrap().msg_type, A2AMessageType::Pong);
    }

    #[test]
    fn test_message_response_tracking() {
        let mut protocol = A2AProtocol::new("agent-a", "Agent A", "First agent");

        let task = A2ATask {
            id: "task-1".to_string(),
            description: "Test task".to_string(),
            required_capabilities: vec![],
            priority: 5,
            deadline: None,
            input: HashMap::new(),
        };

        let request = A2AMessage::task_request("agent-a", "agent-b", &task);
        let request_id = request.id.clone();

        let response = A2AMessage {
            id: "resp-1".to_string(),
            msg_type: A2AMessageType::Accept,
            from: "agent-b".to_string(),
            to: "agent-a".to_string(),
            task_id: Some("task-1".to_string()),
            payload: serde_json::json!({}),
            timestamp: Utc::now(),
            in_reply_to: Some(request_id),
        };

        protocol.receive_message(request);
        protocol.send_message(response);

        assert_eq!(protocol.message_history().len(), 2);
    }
}