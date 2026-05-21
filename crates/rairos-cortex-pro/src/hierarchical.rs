//! Hierarchical agent delegation - manager agents spawning sub-teams.
//!
//! Based on research from:
//! - arXiv:2411.18241 (CrewAI hierarchical mode)
//! - arXiv:2308.08155 (AutoGen v0.4 - nested chat)
//! - arXiv:2508.10146 (Agentic AI Frameworks - hierarchical delegation)
//!
//! ## Architecture
//!
//! ```text
//!                    ┌─────────────────┐
//!                    │   ManagerAgent  │
//!                    │ (Orchestrator)  │
//!                    └────────┬────────┘
//!                             │
//!            ┌────────────────┼────────────────┐
//!            ▼                ▼                ▼
//!     ┌──────────┐     ┌──────────┐     ┌──────────┐
//!     │Scientist │     │ Planner  │     │ Critic   │
//!     │ Sub-team │     │ Sub-team │     │ Sub-team │
//!     └────┬─────┘     └────┬─────┘     └────┬─────┘
//!          │                │                │
//!          ▼                ▼                ▼
//!     ┌──────────┐     ┌──────────┐     ┌──────────┐
//!     │Tool/Repo │     │Tool/Repo │     │Tool/Repo │
//!     └──────────┘     └──────────┘     └──────────┘
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Agent role in hierarchy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentLevel {
    /// Top-level manager orchestrating sub-teams
    Manager,
    /// Specialized agent with specific tools
    Specialist,
    /// Worker agent executing tasks
    Worker,
}

/// Hierarchical agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchicalConfig {
    /// Agent's level in hierarchy
    pub level: AgentLevel,
    /// Agent's specialization domain
    pub domain: String,
    /// Maximum sub-agents this agent can spawn
    pub max_sub_agents: usize,
    /// Timeout for sub-team tasks (ms)
    pub subteam_timeout_ms: u64,
    /// Whether agent can delegate to sub-team
    pub can_delegate: bool,
}

impl HierarchicalConfig {
    pub fn manager(domain: &str) -> Self {
        Self {
            level: AgentLevel::Manager,
            domain: domain.to_string(),
            max_sub_agents: 5,
            subteam_timeout_ms: 60000,
            can_delegate: true,
        }
    }

    pub fn specialist(domain: &str) -> Self {
        Self {
            level: AgentLevel::Specialist,
            domain: domain.to_string(),
            max_sub_agents: 2,
            subteam_timeout_ms: 30000,
            can_delegate: true,
        }
    }

    pub fn worker(domain: &str) -> Self {
        Self {
            level: AgentLevel::Worker,
            domain: domain.to_string(),
            max_sub_agents: 0,
            subteam_timeout_ms: 15000,
            can_delegate: false,
        }
    }
}

/// A task that can be delegated to a sub-team
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedTask {
    /// Task ID
    pub id: String,
    /// Task description
    pub description: String,
    /// Expected output format
    pub expected_output: String,
    /// Parent agent that delegated this task
    pub delegated_by: String,
    /// Sub-agents assigned to this task
    pub assigned_agents: Vec<String>,
    /// Task status
    pub status: TaskStatus,
    /// Result from sub-team execution
    pub result: Option<String>,
}

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Delegated,
}

/// Sub-team definition
#[derive(Debug, Clone)]
pub struct SubTeam {
    /// Team ID
    pub id: String,
    /// Manager agent of this team
    pub manager_id: String,
    /// Sub-agent IDs
    pub agent_ids: Vec<String>,
    /// Active tasks
    pub tasks: Vec<DelegatedTask>,
    /// Team status
    pub status: TeamStatus,
}

/// Team status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TeamStatus {
    Active,
    Completed,
    Failed,
    Suspended,
}

/// Hierarchical delegation manager
pub struct DelegationManager {
    /// Active sub-teams
    teams: RwLock<HashMap<String, SubTeam>>,
    /// Task queue
    pending_tasks: RwLock<Vec<DelegatedTask>>,
    /// Completed tasks
    completed_tasks: RwLock<HashMap<String, DelegatedTask>>,
    /// Maximum concurrent teams
    max_concurrent_teams: usize,
}

impl Default for DelegationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DelegationManager {
    /// Create a new delegation manager
    pub fn new() -> Self {
        Self {
            teams: RwLock::new(HashMap::new()),
            pending_tasks: RwLock::new(Vec::new()),
            completed_tasks: RwLock::new(HashMap::new()),
            max_concurrent_teams: 10,
        }
    }

    /// Create a new sub-team
    pub async fn create_team(
        &self,
        team_id: &str,
        manager_id: &str,
        agent_ids: Vec<String>,
    ) -> Result<SubTeam, String> {
        let mut teams = self.teams.write().await;
        if teams.len() >= self.max_concurrent_teams {
            return Err("Maximum concurrent teams reached".to_string());
        }

        if teams.contains_key(team_id) {
            return Err(format!("Team {} already exists", team_id));
        }

        let team = SubTeam {
            id: team_id.to_string(),
            manager_id: manager_id.to_string(),
            agent_ids,
            tasks: Vec::new(),
            status: TeamStatus::Active,
        };

        teams.insert(team_id.to_string(), team.clone());
        Ok(team)
    }

    /// Delegate a task to a sub-team
    pub async fn delegate_task(
        &self,
        team_id: &str,
        task: DelegatedTask,
    ) -> Result<(), String> {
        let mut teams = self.teams.write().await;
        let team = teams
            .get_mut(team_id)
            .ok_or_else(|| format!("Team {} not found", team_id))?;

        if team.status != TeamStatus::Active {
            return Err(format!("Team {} is not active", team_id));
        }

        let mut task = task;
        task.status = TaskStatus::Delegated;
        team.tasks.push(task);

        Ok(())
    }

    /// Update task status
    pub async fn update_task_status(
        &self,
        task_id: &str,
        status: TaskStatus,
        result: Option<String>,
    ) -> Result<(), String> {
        let mut teams = self.teams.write().await;

        for team in teams.values_mut() {
            if let Some(task) = team.tasks.iter_mut().find(|t| t.id == task_id) {
                task.status = status.clone();
                if let Some(r) = result {
                    task.result = Some(r);
                }

                // If completed, move to completed
                if status == TaskStatus::Completed || status == TaskStatus::Failed {
                    let mut completed = self.completed_tasks.write().await;
                    completed.insert(task_id.to_string(), task.clone());
                }

                return Ok(());
            }
        }

        Err(format!("Task {} not found", task_id))
    }

    /// Get team status
    pub async fn get_team(&self, team_id: &str) -> Option<SubTeam> {
        let teams = self.teams.read().await;
        teams.get(team_id).cloned()
    }

    /// Get all active teams
    pub async fn get_active_teams(&self) -> Vec<SubTeam> {
        let teams = self.teams.read().await;
        teams
            .values()
            .filter(|t| t.status == TeamStatus::Active)
            .cloned()
            .collect()
    }

    /// Get task result
    pub async fn get_task_result(&self, task_id: &str) -> Option<String> {
        let completed = self.completed_tasks.read().await;
        completed.get(task_id).and_then(|t| t.result.clone())
    }

    /// Archive completed team
    pub async fn archive_team(&self, team_id: &str) -> Result<(), String> {
        let mut teams = self.teams.write().await;
        if let Some(team) = teams.get_mut(team_id) {
            team.status = TeamStatus::Completed;
            Ok(())
        } else {
            Err(format!("Team {} not found", team_id))
        }
    }

    /// Get delegation statistics
    pub async fn stats(&self) -> DelegationStats {
        let teams = self.teams.read().await;
        let completed = self.completed_tasks.read().await;
        let pending = self.pending_tasks.read().await;

        DelegationStats {
            total_teams: teams.len(),
            active_teams: teams.values().filter(|t| t.status == TeamStatus::Active).count(),
            total_tasks: teams.values().map(|t| t.tasks.len()).sum::<usize>() + pending.len(),
            completed_tasks: completed.len(),
            pending_tasks: pending.len(),
        }
    }
}

/// Delegation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationStats {
    pub total_teams: usize,
    pub active_teams: usize,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub pending_tasks: usize,
}

/// Helper to create delegated tasks
pub struct DelegatedTaskBuilder {
    id: Option<String>,
    description: Option<String>,
    expected_output: Option<String>,
    delegated_by: Option<String>,
    assigned_agents: Vec<String>,
}

impl DelegatedTaskBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            description: None,
            expected_output: None,
            delegated_by: None,
            assigned_agents: Vec::new(),
        }
    }

    pub fn id(mut self, id: &str) -> Self {
        self.id = Some(id.to_string());
        self
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn expected_output(mut self, output: &str) -> Self {
        self.expected_output = Some(output.to_string());
        self
    }

    pub fn delegated_by(mut self, agent_id: &str) -> Self {
        self.delegated_by = Some(agent_id.to_string());
        self
    }

    pub fn assign_agents(mut self, agents: Vec<&str>) -> Self {
        self.assigned_agents = agents.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn build(self) -> Result<DelegatedTask, String> {
        Ok(DelegatedTask {
            id: self.id.ok_or("Missing task ID")?,
            description: self.description.ok_or("Missing description")?,
            expected_output: self.expected_output.ok_or("Missing expected output")?,
            delegated_by: self.delegated_by.ok_or("Missing delegator")?,
            assigned_agents: self.assigned_agents,
            status: TaskStatus::Pending,
            result: None,
        })
    }
}

impl Default for DelegatedTaskBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_team() {
        let manager = DelegationManager::new();

        let team = manager
            .create_team("team-1", "manager-1", vec!["agent-1".to_string(), "agent-2".to_string()])
            .await
            .unwrap();

        assert_eq!(team.id, "team-1");
        assert_eq!(team.manager_id, "manager-1");
        assert_eq!(team.agent_ids.len(), 2);
        assert_eq!(team.status, TeamStatus::Active);
    }

    #[tokio::test]
    async fn test_delegate_task() {
        let manager = DelegationManager::new();

        manager
            .create_team("team-1", "manager-1", vec!["agent-1".to_string()])
            .await
            .unwrap();

        let task = DelegatedTaskBuilder::new()
            .id("task-1")
            .description("Analyze thermoelectric materials")
            .expected_output("JSON analysis")
            .delegated_by("manager-1")
            .assign_agents(vec!["agent-1"])
            .build()
            .unwrap();

        manager.delegate_task("team-1", task).await.unwrap();

        let team = manager.get_team("team-1").await.unwrap();
        assert_eq!(team.tasks.len(), 1);
        assert_eq!(team.tasks[0].status, TaskStatus::Delegated);
    }

    #[tokio::test]
    async fn test_update_task_status() {
        let manager = DelegationManager::new();

        manager
            .create_team("team-1", "manager-1", vec!["agent-1".to_string()])
            .await
            .unwrap();

        let task = DelegatedTaskBuilder::new()
            .id("task-1")
            .description("Analyze materials")
            .expected_output("JSON")
            .delegated_by("manager-1")
            .build()
            .unwrap();

        manager.delegate_task("team-1", task).await.unwrap();

        manager
            .update_task_status("task-1", TaskStatus::Completed, Some("Result: success".to_string()))
            .await
            .unwrap();

        let result = manager.get_task_result("task-1").await;
        assert_eq!(result, Some("Result: success".to_string()));
    }

    #[tokio::test]
    async fn test_stats() {
        let manager = DelegationManager::new();

        manager
            .create_team("team-1", "manager-1", vec!["agent-1".to_string()])
            .await
            .unwrap();

        let stats = manager.stats().await;
        assert_eq!(stats.total_teams, 1);
        assert_eq!(stats.active_teams, 1);
    }

    #[tokio::test]
    async fn test_archive_team() {
        let manager = DelegationManager::new();

        manager
            .create_team("team-1", "manager-1", vec!["agent-1".to_string()])
            .await
            .unwrap();

        manager.archive_team("team-1").await.unwrap();

        let team = manager.get_team("team-1").await.unwrap();
        assert_eq!(team.status, TeamStatus::Completed);
    }

    #[test]
    fn test_hierarchical_config() {
        let manager_config = HierarchicalConfig::manager("materials");
        assert_eq!(manager_config.level, AgentLevel::Manager);
        assert!(manager_config.can_delegate);

        let worker_config = HierarchicalConfig::worker("analysis");
        assert_eq!(worker_config.level, AgentLevel::Worker);
        assert!(!worker_config.can_delegate);
    }
}