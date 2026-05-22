//! # rairos-cortex-pro
//!
//! Production multi-agent collaboration framework with LangGraph-style orchestration.
//!
//! ## Overview
//!
//! This crate provides advanced multi-agent capabilities for research automation,
//! building on the experimental `rairos-cortex` with production-ready features.
//!
//! ## Key Concepts
//!
//! - **Agent**: An autonomous entity that can perform specialized tasks
//! - **Crew**: A team of agents working together on a shared objective
//! - **Pipeline**: A directed workflow graph (LangGraph-style)
//! - **State**: Shared context passed through the pipeline
//!
//! ## Architecture
//!
//! ```text
//! ResearchCrew
//! ├── ResearcherAgent    → Paper search, extraction, indexing
//! ├── GapAnalyzerAgent   → Research gap detection
//! ├── CitationGraphAgent → Citation network analysis
//! ├── VectorIndexerAgent → Vector storage and retrieval
//! └── ReportWriterAgent  → Synthesis and report generation
//! ```
//!
//! ## SparksMatter Integration
//!
//! With the `tools` feature enabled, this crate supports SparksMatter-style
//! multi-agent workflows for materials discovery:
//!
//! ```text
//! Ideation → Planning → Execution (with tools) → Reporting
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use rairos_cortex_pro::{ResearchCrew, CrewConfig, ResearchState};
//!
//! let crew = ResearchCrew::new(CrewConfig::default());
//! let result = crew.run("machine learning for materials discovery").await?;
//! println!("Report: {}", result.report);
//! ```

pub mod agent;
pub mod crew;
pub mod pipeline;
pub mod state;
pub mod error;

#[cfg(feature = "tools")]
pub mod sparks_crew;

#[cfg(feature = "tools")]
pub mod sparks_agents;

#[cfg(feature = "tools")]
pub mod tools;

/// Memory module for persistent context across agent interactions
#[cfg(feature = "tools")]
pub mod memory;

/// LLM-based agents using real LLM calls
#[cfg(feature = "tools")]
pub mod llm_agents;

/// MCTS planner for tool selection (ToolTree-style)
#[cfg(feature = "tools")]
pub mod mcts_planner;

/// SSE streaming for real-time agent progress
#[cfg(feature = "tools")]
pub mod streaming_sse;

/// Hierarchical agent delegation (manager → sub-team)
#[cfg(feature = "tools")]
pub mod hierarchical;

/// Benchmark module for agent evaluation (GAIA, AgentBench style)
#[cfg(feature = "tools")]
pub mod benchmark;

/// A2A protocol for agent-to-agent communication
#[cfg(feature = "tools")]
pub mod a2a_protocol;

/// Active Learning for materials discovery (LEAP-style)
#[cfg(feature = "tools")]
pub mod active_learning;

/// Context compression for token optimization (ECoRAG, ACC-RAG style)
#[cfg(feature = "tools")]
pub mod context_compression;

/// DAG workflow executor for parallel task execution (ROMA, AdaptOrch style)
#[cfg(feature = "tools")]
pub mod dag_executor;

/// Experience replay for self-improving agents (R³, ERL, LEAFE style)
#[cfg(feature = "tools")]
pub mod experience_replay;

/// Multi-agent consensus for agent agreement (EMS, AgentAuditor style)
#[cfg(feature = "tools")]
pub mod multi_agent_consensus;

/// Self-correction and reflection (CRITIC, Self-Refiner, Reflexion style)
#[cfg(feature = "tools")]
pub mod self_correction;

/// Transaction compensation for robust error recovery (RAC style)
#[cfg(feature = "tools")]
pub mod transaction_compensation;

/// FLARE-inspired lookahead planning
#[cfg(feature = "tools")]
pub mod flare_planning;

/// Atomic fact memory augmentation
#[cfg(feature = "tools")]
pub mod atomic_fact_memory;

/// Forest-of-Thought reasoning
#[cfg(feature = "tools")]
pub mod forest_of_thought;

/// Graph-of-Agents topology (GoA arXiv:2604.17148)
#[cfg(feature = "tools")]
pub mod graph_of_agents;

/// PICCO prompt framework (arXiv:2604.14197)
#[cfg(feature = "tools")]
pub mod picco_prompt;

/// ROMA recursive meta-agent (arXiv:2602.01848)
#[cfg(feature = "tools")]
pub mod roma_meta_agent;

/// Adaptive orchestration for dynamic multi-agent topology (AdaptOrch, Symphony-Coord, BIGMAS)
#[cfg(feature = "tools")]
pub mod adaptive_orchestration;

/// Deliberation-first orchestration (DOVA arXiv:2603.13327)
#[cfg(feature = "tools")]
pub mod deliberation;

/// Memory Intelligence Agent (MIA arXiv:2604.04503)
#[cfg(feature = "tools")]
pub mod mia;

/// Worker Pool for parallel task execution
#[cfg(feature = "tools")]
pub mod worker_pool;

/// Tool Registry for centralized tool management
#[cfg(feature = "tools")]
pub mod tool_registry;

/// Task Tracker for multi-agent workflow state management
#[cfg(feature = "tools")]
pub mod task_tracker;

/// Event Emitter for framework observability
#[cfg(feature = "tools")]
pub mod event_emitter;

/// Agent Registry for centralized agent discovery and lifecycle management
#[cfg(feature = "tools")]
pub mod agent_registry;

/// Intent Classifier for request routing and intent classification
#[cfg(feature = "tools")]
pub mod intent_classifier;

/// Conversation History for audit trail and context management
#[cfg(feature = "tools")]
pub mod conversation_history;

/// Agent safety and audit (SafeAgent, ReliabilityBench)
#[cfg(feature = "tools")]
pub mod agent_safety;

/// Integrations with other Rairos crates
pub mod integrations;

#[cfg(feature = "api")]
pub mod api;

pub use agent::{Agent, AgentConfig, AgentOutput, AgentRole};
pub use crew::{ResearchCrew, CrewConfig, CrewResult};
pub use pipeline::{Pipeline, PipelineNode, PipelineEdge};
pub use state::{ResearchState, Phase, ResearchContext, CrewContext};
pub use error::{CortexProError, Result};

#[cfg(feature = "tools")]
pub use sparks_crew::{SparksCrew, Plan, PlanStep, ExecutionResult, ResearchReport};

#[cfg(feature = "tools")]
pub use memory::{MemoryBank, IdeationEntry, IdeationStatus, ExperimentationEntry, MemoryStats, MemoryEntryBuilder, ExperimentationEntryBuilder};

#[cfg(feature = "tools")]
pub use llm_agents::{LlmHypothesisAgent, LlmHypothesisCriticAgent, LlmPlannerAgent, LlmPlanCriticAgent, LlmReportWriterAgent};

#[cfg(feature = "tools")]
pub use mcts_planner::{MctsPlanner, Tool, ToolCategory, ToolSelection};

#[cfg(feature = "tools")]
pub use streaming_sse::{SseBroadcaster, SseEvent, SseTimer};

#[cfg(feature = "tools")]
pub use hierarchical::{DelegationManager, DelegatedTask, HierarchicalConfig, AgentLevel, SubTeam, TeamStatus, DelegationStats};

#[cfg(feature = "tools")]
pub use benchmark::{BenchmarkMetrics, BenchmarkReport, BenchmarkTask, TaskResult, MilestoneResult, gaia_benchmark_tasks, collaboration_tasks, run_benchmark};

#[cfg(feature = "tools")]
pub use a2a_protocol::{A2AProtocol, A2AMessage, A2ATask, AgentCard, AgentCapability, A2AMessageType};

#[cfg(feature = "tools")]
pub use active_learning::{ActiveLearningManager, MaterialCandidate, EvaluationResult, AcquisitionFunction, ActiveLearningStats, HypothesisGenerator};

#[cfg(feature = "tools")]
pub use context_compression::{ContextCompressor, CompressedContext, CompressionRatio, TokenBudget, BudgetEntry};

#[cfg(feature = "tools")]
pub use dag_executor::{TaskDag, DagNode, DagExecutor, DagExecutionResult, NodeResult, dag_from_plan_steps};

#[cfg(feature = "tools")]
pub use experience_replay::{ExperienceReplay, Experience, TrajectoryStep, ExperienceOutcome, SelfReflector, ConsolidationResult};

#[cfg(feature = "tools")]
pub use multi_agent_consensus::{MultiAgentConsensus, ConsensusResult, Vote, VotingMechanism, EvidenceAuditor, AuditResult, Evidence, ConsensusAgent};

#[cfg(feature = "tools")]
pub use self_correction::{SelfCorrector, ReflexionAgent, ExternalValidator, EvaluationResult as SelfCorrectionEvaluationResult, Correction, Reflection, CorrectionType, ValidationResult, ValidationIssue, ValidationRule};

#[cfg(feature = "tools")]
pub use transaction_compensation::{TransactionLog, ToolEvent, ToolError, CompensationAction, RollbackResult, VigilLearner, EmotionalEntry, RBTDiagnosis, RollbackType, CompensationType};

#[cfg(feature = "tools")]
pub use flare_planning::{FlarePlanner, FlareConfig, PlanningState, PlannedAction, Trajectory, ValueEstimator, HeuristicValueEstimator, PlanningResult, PpaPlanner, Pitfall, PpaPlanningResult};

#[cfg(feature = "tools")]
pub use atomic_fact_memory::{AtomicFactMemory, AtomicFact, MemoryEntry, TaskType, MemoryTier, Outcome, AtomicReasoner, CognitiveRoute, RouteType, ConsolidationReport, MemorySystemStats};

#[cfg(feature = "tools")]
pub use forest_of_thought::{ForestReasoner, ForestConfig, ReasoningTree, ThoughtNode, NodeType, ReasoningResult, Consensus, ForestStats, DiagramOfThoughtReasoner, RoleToken};

#[cfg(feature = "tools")]
pub use graph_of_agents::{CommunicationGraph, GraphConfig, AgentNode, AgentState, GraphEdge, AgentMessage, MessageType, PoolingStrategy, GraphStats, ConditionalGraphDesigner, EdgeCondition, ConditionType, BeliefCollaborationManager, Belief, GoAAgentRole};

#[cfg(feature = "tools")]
pub use picco_prompt::{PiccoPrompt, Persona, ExpertiseLevel, Instructions, Priority, ContextItem, ContextType, Constraint, ConstraintType, OutputSpec, FormatType, OutputStructure, PreferencePromptOptimizer, PromptCandidate, PreferenceFeedback, OptimizerConfig, OptimizerStats};

#[cfg(feature = "tools")]
pub use roma_meta_agent::{MetaAgent, RomaTask, TaskStatus, MetaRole, RomaConfig, ExecutionResult as RomaExecutionResult, RomaStats, ChainOfMindset, Mindset, Mindsetswitch, TdpSupervisor, SubGoal, ScopedContext, TdpConfig};

#[cfg(feature = "tools")]
pub use adaptive_orchestration::{AdaptiveOrchestrator, OrchestrationConfig, TopologySelection, Topology, TaskDependency, TaskDag as OrchestrationTaskDag, OrchestrationStats, TopologyRecord, TaskStats, SymphonyCoordinator, SymphonyConfig, CapabilitySignal, SlotRequirement, AgentSelection, CandidateAgent};

#[cfg(feature = "tools")]
pub use deliberation::{DeliberationEngine, DeliberationConfig, DeliberationResult, DeliberationTrigger, DeliberationStats, DeliberationRecord, ThinkingTier, QueryAnalysis, HybridReasoning, HybridReasoningConfig, ReasoningPhase, Perspective, BlackboardEntry, Critique, RefinementResult};

#[cfg(feature = "tools")]
pub use mia::{MemoryIntelligenceAgent, MIAConfig, MIAResult, MIAStats, MemoryManager, CompressedTrajectory, MemoryStats as MIA_MemoryStats, Planner, PlannerConfig, SearchPlan, SearchStep, SearchAction, Executor, ExecutorConfig, ExecutionResult as MIA_ExecutionResult, PlanExecution, MemoryConverter, Reflection as MIA_Reflection, PlanTemplate};

#[cfg(feature = "tools")]
pub use worker_pool::{WorkerPool, WorkerPoolConfig, WorkerPoolStats, AsyncWorkerPool, Task, TaskPayload as WorkerTaskPayload, TaskResult as WorkerTaskResult, TaskHandle, Priority as TaskPriority, PriorityTaskQueue};

#[cfg(feature = "tools")]
pub use task_tracker::{TaskTracker, TaskTrackerHandle, TaskState, TrackPriority, TrackedTask, TrackerStats, TrackerEvent, TrackerEventType};

#[cfg(feature = "tools")]
pub use tool_registry::{ToolRegistry, ToolRegistryBuilder, ToolBuilder, ToolSchema, ToolParameter, ToolExecResult, ToolContext, RegisteredTool, ToolRegistryError, JsonValue};

#[cfg(feature = "tools")]
pub use event_emitter::{EventEmitter, Event, EventType, EventHandler, EventData, EmitterStats, HandlerId, global_emitter, event_types};

#[cfg(feature = "tools")]
pub use agent_safety::{AgentSafetyGuard, SafetyConfig, SafetyVerdict, RiskLevel, AuditEntry, SafetyRule, SafetyCheckResult, SafetyStats, ValueDriftDetector, ValuePrinciple, ValueCategory, DriftRecord, DriftSummary, ReliabilityTracker, ReliabilityMetrics};

/// Common utilities module
pub mod utils;
pub use utils::{uuid_simple, current_timestamp, current_timestamp_ms, generate_id, format_timestamp};
