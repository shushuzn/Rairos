//! Multi-Agent Consensus Module for agent agreement and negotiation.
//!
//! Based on research from:
//! - EMS (arXiv:2604.02863) - Efficient Majority-then-Stopping voting
//! - AgentAuditor (arXiv:2602.09341) - Evidence-based adjudication
//! - Dialogue Diplomats (arXiv:2511.17654) - Conflict resolution via HCN
//!
//! ## Architecture
//!
//! ```text
//! Agent 1 ──┐
//! Agent 2 ──┼──► Consensus ──► Agreed Action
//! Agent 3 ──┘      │
//!                   │
//!                   ▼
//!            ┌─────────────────┐
//!            │  Deliberation   │ ← Multi-round discussion
//!            │  Voting        │ ← Evidence-based
//!            │  Arbitration   │ ← Conflict resolution
//!            └─────────────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};

/// A vote from an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// Agent ID
    pub agent_id: String,
    /// The agent's chosen option
    pub choice: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Reasoning for this choice
    pub reasoning: String,
    /// Evidence supporting this choice
    pub evidence: Vec<String>,
}

/// Voting mechanism types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum VotingMechanism {
    /// Simple majority vote
    Majority,
    /// Majority-then-Stopping (EMS)
    MajorityThenStopping,
    /// Borda count ranking
    BordaCount,
    /// Ranked choice voting
    RankedChoice,
    /// Evidence-weighted voting
    EvidenceWeighted,
}

/// Result of a consensus decision
#[derive(Debug, Clone)]
pub struct ConsensusResult {
    /// Whether consensus was reached
    pub reached: bool,
    /// The agreed decision
    pub decision: Option<String>,
    /// Confidence in the decision
    pub confidence: f32,
    /// Votes by agent
    pub votes: Vec<Vote>,
    /// Number of deliberation rounds
    pub rounds: u32,
    /// Disagreement score (0.0 = full agreement)
    pub disagreement_score: f32,
}

/// A deliberation turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliberationTurn {
    /// Agent ID
    pub agent_id: String,
    /// The agent's position after this turn
    pub position: String,
    /// Reasoning for the position
    pub reasoning: String,
    /// Whether this agent changed their mind
    pub changed_mind: bool,
}

/// Multi-Agent Consensus handler
pub struct MultiAgentConsensus {
    /// Registered agents
    agents: HashMap<String, ConsensusAgent>,
    /// Voting mechanism
    mechanism: VotingMechanism,
    /// Maximum deliberation rounds
    max_rounds: u32,
    /// Consensus threshold (0.0 - 1.0)
    consensus_threshold: f32,
}

/// A participating agent in consensus
#[derive(Debug, Clone)]
pub struct ConsensusAgent {
    /// Agent ID
    pub id: String,
    /// Agent's current position
    pub position: Option<String>,
    /// Historical positions
    pub history: Vec<String>,
    /// Trust score (0.0 - 1.0)
    pub trust_score: f32,
}

impl MultiAgentConsensus {
    /// Create a new consensus handler
    pub fn new(mechanism: VotingMechanism) -> Self {
        Self {
            agents: HashMap::new(),
            mechanism,
            max_rounds: 5,
            consensus_threshold: 0.7,
        }
    }

    /// Set consensus threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.consensus_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Set maximum deliberation rounds
    pub fn with_max_rounds(mut self, rounds: u32) -> Self {
        self.max_rounds = rounds;
        self
    }

    /// Register an agent for consensus
    pub fn register_agent(&mut self, agent_id: &str, initial_position: Option<&str>, trust_score: f32) {
        let agent = ConsensusAgent {
            id: agent_id.to_string(),
            position: initial_position.map(String::from),
            history: initial_position.map(String::from).into_iter().collect(),
            trust_score: trust_score.clamp(0.0, 1.0),
        };
        self.agents.insert(agent_id.to_string(), agent);
    }

    /// Run a single vote with the current mechanism
    pub fn vote(&self, votes: Vec<Vote>) -> ConsensusResult {
        match self.mechanism {
            VotingMechanism::Majority => self.majority_vote(votes),
            VotingMechanism::MajorityThenStopping => self.ems_vote(votes),
            VotingMechanism::BordaCount => self.borda_vote(votes),
            VotingMechanism::EvidenceWeighted => self.evidence_weighted_vote(votes),
            VotingMechanism::RankedChoice => self.ranked_choice_vote(votes),
        }
    }

    /// Majority voting
    fn majority_vote(&self, mut votes: Vec<Vote>) -> ConsensusResult {
        let mut choice_counts: HashMap<String, (u32, f32)> = HashMap::new();

        for vote in &votes {
            let entry = choice_counts.entry(vote.choice.clone()).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += vote.confidence;
        }

        let total = votes.len() as u32;
        let max_count = choice_counts.values().map(|(c, _)| *c).max().unwrap_or(0);

        let winners: Vec<_> = choice_counts
            .iter()
            .filter(|(c, (count, _))| *count == max_count)
            .map(|(c, (_, conf))| (c.clone(), *conf / max_count as f32))
            .collect();

        let (decision, confidence) = if winners.len() == 1 {
            (Some(winners[0].0.clone()), winners[0].1)
        } else {
            // Tie - take highest confidence
            winners.into_iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).map(|(c, _)| (Some(c), 0.5)).unwrap_or((None, 0.0))
        };

        let reached = max_count as f32 / total as f32 >= self.consensus_threshold;
        let disagreement_score = if reached { 0.0 } else { 1.0 - max_count as f32 / total as f32 };

        ConsensusResult {
            reached,
            decision,
            confidence,
            votes,
            rounds: 1,
            disagreement_score,
        }
    }

    /// EMS: Efficient Majority-then-Stopping voting
    /// Based on arXiv:2604.02863
    fn ems_vote(&self, mut votes: Vec<Vote>) -> ConsensusResult {
        // Sort by confidence (descending)
        votes.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        let total = votes.len() as f32;
        let threshold = (total / 2.0).ceil() as usize;

        let mut choice_counts: HashMap<String, u32> = HashMap::new();
        let mut stopping_round = votes.len();

        for (i, vote) in votes.iter().enumerate() {
            *choice_counts.entry(vote.choice.clone()).or_insert(0) += 1;

            // Check if majority reached
            let max_count = *choice_counts.values().max().unwrap_or(&0);
            if max_count >= threshold {
                stopping_round = i + 1;
                break;
            }
        }

        // Find the leading choice
        let leading_choice = choice_counts
            .iter()
            .max_by(|a, b| a.1.cmp(b.1))
            .map(|(c, _)| c.clone());

        let confidence = if let Some(ref choice) = leading_choice {
            let count = *choice_counts.get(choice).unwrap_or(&0) as f32;
            count / total
        } else {
            0.0
        };

        let reached = max_count >= threshold;

        ConsensusResult {
            reached,
            decision: leading_choice,
            confidence,
            votes,
            rounds: stopping_round as u32,
            disagreement_score: if reached { 0.0 } else { 1.0 - confidence },
        }
    }

    /// Borda count voting (ranked preferences)
    fn borda_vote(&self, votes: Vec<Vote>) -> ConsensusResult {
        // Simple Borda: each vote gives points based on confidence as ranking proxy
        let mut scores: HashMap<String, f32> = HashMap::new();

        for vote in &votes {
            // Use confidence as implicit ranking
            let points = vote.confidence * 10.0;
            *scores.entry(vote.choice.clone()).or_insert(0.0) += points;
        }

        let winner = scores.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).map(|(c, _)| c.clone());
        let total_confidence: f32 = scores.values().sum();
        let confidence = winner.as_ref().map(|w| scores.get(w).unwrap_or(&0.0) / total_confidence.max(0.001)).unwrap_or(0.0);

        ConsensusResult {
            reached: confidence >= self.consensus_threshold,
            decision: winner,
            confidence,
            votes,
            rounds: 1,
            disagreement_score: 1.0 - confidence,
        }
    }

    /// Evidence-weighted voting
    fn evidence_weighted_vote(&self, votes: Vec<Vote>) -> ConsensusResult {
        let mut weighted_scores: HashMap<String, f32> = HashMap::new();

        for vote in &votes {
            let evidence_bonus = (vote.evidence.len() as f32 * 0.1).min(0.5);
            let weight = vote.confidence + evidence_bonus;
            *weighted_scores.entry(vote.choice.clone()).or_insert(0.0) += weight;
        }

        let total: f32 = weighted_scores.values().sum();
        let winner = weighted_scores.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).map(|(c, _)| c.clone());
        let confidence = winner.as_ref().map(|w| weighted_scores.get(w).unwrap_or(&0.0) / total.max(0.001)).unwrap_or(0.0);

        ConsensusResult {
            reached: confidence >= self.consensus_threshold,
            decision: winner,
            confidence,
            votes,
            rounds: 1,
            disagreement_score: 1.0 - confidence,
        }
    }

    /// Ranked choice voting (simplified)
    fn ranked_choice_vote(&self, votes: Vec<Vote>) -> ConsensusResult {
        // For ranked choice, we'd need ranked data
        // Fall back to majority with confidence
        self.majority_vote(votes)
    }

    /// Run deliberation rounds
    pub fn deliberate(&mut self, question: &str, mut positions: HashMap<String, String>) -> ConsensusResult {
        let agent_ids: Vec<_> = positions.keys().cloned().collect();
        let mut all_votes: Vec<Vote> = Vec::new();
        let mut rounds = 0u32;
        let mut deliberation_log: Vec<DeliberationTurn> = Vec::new();

        // Initial positions
        for (agent_id, position) in &positions {
            self.agents.get_mut(agent_id).map(|a| {
                a.position = Some(position.clone());
                a.history.push(position.clone());
            });

            all_votes.push(Vote {
                agent_id: agent_id.clone(),
                choice: position.clone(),
                confidence: 0.7, // Initial confidence
                reasoning: format!("Initial position on: {}", question),
                evidence: vec![],
            });

            deliberation_log.push(DeliberationTurn {
                agent_id: agent_id.clone(),
                position: position.clone(),
                reasoning: format!("Initial position on: {}", question),
                changed_mind: false,
            });
        }

        // Deliberation rounds
        for round in 1..=self.max_rounds {
            rounds = round;

            // Aggregate positions from previous round
            let aggregated: HashMap<String, Vec<String>> = deliberation_log
                .iter()
                .filter(|t| t.agent_id != "aggregator")
                .fold(HashMap::new(), |mut acc, turn| {
                    acc.entry(turn.position.clone()).or_default().push(turn.agent_id.clone());
                    acc
                });

            // Check for consensus
            if aggregated.len() == 1 {
                // All agents agree
                let decision = aggregated.keys().next().cloned();
                let confidence = 1.0 - (round as f32 / self.max_rounds as f32) * 0.3;
                return ConsensusResult {
                    reached: true,
                    decision,
                    confidence,
                    votes: all_votes,
                    rounds,
                    disagreement_score: 0.0,
                };
            }

            // Generate synthesis for next round
            let synthesis = self.synthesize_positions(&aggregated);
            if let Some(ref syn) = synthesis {
                deliberation_log.push(DeliberationTurn {
                    agent_id: "aggregator".to_string(),
                    position: syn.clone(),
                    reasoning: "Synthesis of current positions".to_string(),
                    changed_mind: false,
                });

                // Update agent positions based on synthesis
                for agent_id in &agent_ids {
                    if let Some(agent) = self.agents.get_mut(agent_id) {
                        if agent.position.as_ref() != Some(syn) {
                            deliberation_log.push(DeliberationTurn {
                                agent_id: agent_id.clone(),
                                position: syn.clone(),
                                reasoning: "Updated position after deliberation".to_string(),
                                changed_mind: true,
                            });
                            agent.position = Some(syn.clone());
                            agent.history.push(syn.clone());

                            all_votes.push(Vote {
                                agent_id: agent_id.clone(),
                                choice: syn.clone(),
                                confidence: 0.8 + (round as f32 * 0.03), // Increasing confidence
                                reasoning: format!("Position after round {} deliberation", round),
                                evidence: vec![],
                            });
                        }
                    }
                }
            }
        }

        // Final vote after deliberation
        let final_positions: HashMap<String, String> = self.agents
            .iter()
            .filter_map(|(id, a)| a.position.clone().map(|p| (id.clone(), p)))
            .collect();

        let final_votes: Vec<Vote> = final_positions
            .iter()
            .map(|(id, pos)| Vote {
                agent_id: id.clone(),
                choice: pos.clone(),
                confidence: 0.9,
                reasoning: "Final position after deliberation".to_string(),
                evidence: vec![],
            })
            .collect();

        let mut result = self.vote(final_votes);
        result.rounds = rounds;
        result
    }

    /// Synthesize multiple positions into a unified position
    fn synthesize_positions(&self, positions: &HashMap<String, Vec<String>>) -> Option<String> {
        // Find common elements across positions
        let mut all_elements: Vec<String> = positions.keys().cloned().collect();

        if all_elements.is_empty() {
            return None;
        }

        // Simple synthesis: find the most common elements
        if all_elements.len() == 1 {
            return Some(all_elements.remove(0));
        }

        // Find overlap - positions that appear in multiple groups
        let mut element_counts: HashMap<String, usize> = HashMap::new();
        for elem in positions.keys() {
            *element_counts.entry(elem.clone()).or_insert(0) += 1;
        }

        // Return the most common element
        element_counts
            .iter()
            .max_by(|a, b| a.1.cmp(b.1))
            .map(|(e, _)| e.clone())
    }

    /// Get consensus statistics
    pub fn get_stats(&self) -> ConsensusStats {
        let total_agents = self.agents.len();
        let avg_trust: f32 = if total_agents > 0 {
            self.agents.values().map(|a| a.trust_score).sum::<f32>() / total_agents as f32
        } else {
            0.0
        };

        ConsensusStats {
            registered_agents: total_agents,
            average_trust_score: avg_trust,
            mechanism: self.mechanism,
            consensus_threshold: self.consensus_threshold,
        }
    }
}

/// Statistics about the consensus handler
#[derive(Debug, Clone)]
pub struct ConsensusStats {
    pub registered_agents: usize,
    pub average_trust_score: f32,
    pub mechanism: VotingMechanism,
    pub consensus_threshold: f32,
}

// =============================================================================
// Evidence Auditor (Based on AgentAuditor arXiv:2602.09341)
// =============================================================================

/// Evidence for a decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Evidence ID
    pub id: String,
    /// The evidence content
    pub content: String,
    /// Source agent
    pub source: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Relevance score (0.0 - 1.0)
    pub relevance: f32,
}

/// An audit result
#[derive(Debug, Clone)]
pub struct AuditResult {
    /// Whether the decision is justified
    pub justified: bool,
    /// Confidence score
    pub confidence: f32,
    /// Supporting evidence
    pub supporting_evidence: Vec<Evidence>,
    /// Contradicting evidence
    pub contradicting_evidence: Vec<Evidence>,
    /// Gaps in reasoning
    pub reasoning_gaps: Vec<String>,
}

/// Evidence auditor for adjudicating disputes
pub struct EvidenceAuditor {
    /// Minimum evidence threshold
    min_evidence_threshold: usize,
    /// Minimum relevance score
    min_relevance: f32,
}

impl EvidenceAuditor {
    /// Create a new evidence auditor
    pub fn new() -> Self {
        Self {
            min_evidence_threshold: 1,
            min_relevance: 0.5,
        }
    }

    /// Audit a decision based on provided evidence
    pub fn audit(&self, decision: &str, evidence: Vec<Evidence>) -> AuditResult {
        // Filter by relevance
        let relevant_evidence: Vec<_> = evidence
            .into_iter()
            .filter(|e| e.relevance >= self.min_relevance)
            .collect();

        let supporting = relevant_evidence
            .iter()
            .filter(|e| e.content.to_lowercase().contains(&decision.to_lowercase()))
            .cloned()
            .collect();

        let contradicting = relevant_evidence
            .iter()
            .filter(|e| !e.content.to_lowercase().contains(&decision.to_lowercase()))
            .cloned()
            .collect();

        let justification_score = if supporting.len() >= self.min_evidence_threshold {
            supporting.len() as f32 / (supporting.len() + contradicting.len()).max(1) as f32
        } else {
            0.0
        };

        let reasoning_gaps = self.find_gaps(&supporting, &contradicting);

        AuditResult {
            justified: justification_score > 0.6,
            confidence: justification_score,
            supporting_evidence: supporting,
            contradicting_evidence: contradicting,
            reasoning_gaps,
        }
    }

    /// Find gaps in the reasoning
    fn find_gaps(&self, supporting: &[Evidence], contradicting: &[Evidence]) -> Vec<String> {
        let mut gaps = Vec::new();

        if supporting.is_empty() {
            gaps.push("No supporting evidence found".to_string());
        }

        if contradicting.len() > supporting.len() {
            gaps.push(format!("More contradicting evidence ({}) than supporting ({})",
                contradicting.len(), supporting.len()));
        }

        // Check for temporal gaps
        let sources: HashSet<_> = supporting.iter().map(|e| &e.source).collect();
        if sources.len() < 2 {
            gaps.push("Evidence from limited sources".to_string());
        }

        gaps
    }
}

impl Default for EvidenceAuditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_majority_vote() {
        let consensus = MultiAgentConsensus::new(VotingMechanism::Majority);

        let votes = vec![
            Vote { agent_id: "a1".to_string(), choice: "A".to_string(), confidence: 0.9, reasoning: "Reason A".to_string(), evidence: vec![] },
            Vote { agent_id: "a2".to_string(), choice: "B".to_string(), confidence: 0.8, reasoning: "Reason B".to_string(), evidence: vec![] },
            Vote { agent_id: "a3".to_string(), choice: "A".to_string(), confidence: 0.85, reasoning: "Reason A".to_string(), evidence: vec![] },
        ];

        let result = consensus.vote(votes);
        assert!(result.reached);
        assert_eq!(result.decision, Some("A".to_string()));
    }

    #[test]
    fn test_ems_vote() {
        let consensus = MultiAgentConsensus::new(VotingMechanism::MajorityThenStopping);

        let votes = vec![
            Vote { agent_id: "a1".to_string(), choice: "A".to_string(), confidence: 0.95, reasoning: "High confidence".to_string(), evidence: vec![] },
            Vote { agent_id: "a2".to_string(), choice: "A".to_string(), confidence: 0.9, reasoning: "Agree".to_string(), evidence: vec![] },
            Vote { agent_id: "a3".to_string(), choice: "B".to_string(), confidence: 0.7, reasoning: "Disagree".to_string(), evidence: vec![] },
        ];

        let result = consensus.vote(votes);
        assert!(result.reached);
        assert_eq!(result.rounds, 2); // Should stop after 2 votes for A
    }

    #[test]
    fn test_evidence_auditor() {
        let auditor = EvidenceAuditor::new();

        let evidence = vec![
            Evidence {
                id: "e1".to_string(),
                content: "Decision A is correct because X".to_string(),
                source: "agent1".to_string(),
                timestamp: Utc::now(),
                relevance: 0.9,
            },
            Evidence {
                id: "e2".to_string(),
                content: "However, Y suggests otherwise".to_string(),
                source: "agent2".to_string(),
                timestamp: Utc::now(),
                relevance: 0.6,
            },
        ];

        let result = auditor.audit("A", evidence);
        assert!(result.justified);
    }
}
