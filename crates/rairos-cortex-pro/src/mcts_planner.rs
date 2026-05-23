//! ToolTree-style MCTS planning for intelligent tool selection.
//!
//! Based on arXiv:2603.12740 - "ToolTree: Monte Carlo Tree Search for Tool Planning"
//!
//! ## Architecture
//!
//! ```text
//! Query → MCTS Planner
//!            │
//!            ├── Selection (UCB1)
//!            ├── Expansion (generate tool candidates)
//!            ├── Simulation (evaluate tool sequences)
//!            └── Backpropagation (update Q-values)
//!            │
//!            ▼
//!         ToolTree
//!       /        \
//!      ▼          ▼
//!   Tool A      Tool B
//!   /    \        |
//!  ▼     ▼        ▼
//! leaf  leaf   leaf
//! ```

use smallvec::SmallVec;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;

/// Maximum depth for MCTS search
const MAX_DEPTH: usize = 5;

/// Maximum iterations per query
const MAX_ITERATIONS: usize = 100;

/// UCB1 exploration constant
const UCB_CONSTANT: f64 = 1.4142135623730951; // sqrt(2)

/// A tool that can be used in the research pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub input_schema: HashMap<String, serde_json::Value>,
    pub estimated_cost: f32, // 0.0 to 1.0
    /// Cached lowercase description for fast case-insensitive matching
    #[serde(skip)]
    pub description_lower: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    Literature,
    Simulation,
    Database,
    Analysis,
    Visualization,
    Validation,
}

/// A node in the MCTS search tree
#[derive(Debug, Clone)]
pub struct MctsNode {
    /// Tool selected at this node (None for root)
    pub tool: Option<Tool>,
    /// Children nodes (SmallVec for stack allocation of small children lists)
    pub children: SmallVec<[MctsNode; 4]>,
    /// Visit count
    pub visit_count: u32,
    /// Total Q-value (cumulative reward)
    pub q_value: f64,
    /// Parent node index (None for root)
    pub parent: Option<usize>,
    /// Depth in tree
    pub depth: usize,
    /// Whether this node is fully expanded
    pub is_expanded: bool,
    /// FORESIGHT: Pre-execution predicted score (ToolTree innovation)
    pub foresight_score: f64,
    /// HINDSIGHT: Post-execution actual score (ToolTree innovation)
    pub hindsight_score: f64,
    /// Whether this node has been executed
    pub is_executed: bool,
}

impl MctsNode {
    /// Create a new root node
    pub fn root() -> Self {
        Self {
            tool: None,
            children: SmallVec::new(),
            visit_count: 0,
            q_value: 0.0,
            parent: None,
            depth: 0,
            is_expanded: false,
            foresight_score: 0.0,
            hindsight_score: 0.0,
            is_executed: false,
        }
    }

    /// Create a child node
    pub fn child(parent_idx: usize, tool: Tool, depth: usize) -> Self {
        Self {
            tool: Some(tool),
            children: SmallVec::new(),
            visit_count: 0,
            q_value: 0.0,
            parent: Some(parent_idx),
            depth,
            is_expanded: false,
            foresight_score: 0.0,
            hindsight_score: 0.0,
            is_executed: false,
        }
    }

    /// Calculate UCB1 score
    pub fn ucb1(&self, parent_visits: u32) -> f64 {
        if self.visit_count == 0 {
            return f64::MAX;
        }
        let exploitation = self.q_value / self.visit_count as f64;
        let exploration = UCB_CONSTANT * (parent_visits as f64 / self.visit_count as f64).sqrt();
        exploitation + exploration
    }

    /// Check if this node is fully expanded (all tools tried)
    pub fn is_fully_expanded(&self, available_tools: &[Tool]) -> bool {
        self.children.len() >= available_tools.len()
    }

    /// Check if this is a leaf node
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

/// Tool selection result from MCTS planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSelection {
    pub tool_name: String,
    pub confidence: f64,
    pub reasoning: String,
    pub alternative_tools: Vec<(String, f64)>, // name, confidence
}

/// Monte Carlo Tree Search planner for tool selection
pub struct MctsPlanner {
    /// Available tools
    tools: RwLock<Vec<Tool>>,
    /// Search tree
    tree: RwLock<Vec<MctsNode>>,
    /// Tool effectiveness history
    tool_effectiveness: RwLock<HashMap<String, f32>>,
}

impl Default for MctsPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl MctsPlanner {
    /// Create a new MCTS planner
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(Vec::new()),
            tree: RwLock::new(vec![MctsNode::root()]),
            tool_effectiveness: RwLock::new(HashMap::new()),
        }
    }

    /// Register a tool with the planner
    pub fn register_tool(&self, tool: Tool) {
        self.tools.write().push(tool);
    }

    /// Register multiple tools
    pub fn register_tools(&self, tools: Vec<Tool>) {
        let mut tools_lock = self.tools.write();
        tools_lock.extend(tools);
    }

    /// Update tool effectiveness based on execution result
    pub fn update_effectiveness(&self, tool_name: &str, effectiveness: f32) {
        let mut eff = self.tool_effectiveness.write();
        // EMA-style update
        let prev = eff.get(tool_name).copied().unwrap_or(0.5);
        let new = 0.7 * effectiveness + 0.3 * prev;
        eff.insert(tool_name.to_string(), new);
    }

    /// Select the best tool using MCTS
    pub fn select_tools(&self, query: &str, context: &str) -> ToolSelection {
        let tools_guard = self.tools.read();
        if tools_guard.is_empty() {
            return ToolSelection {
                tool_name: String::new(),
                confidence: 0.0,
                reasoning: "No tools available".to_string(),
                alternative_tools: vec![],
            };
        }

        // Run MCTS iterations - iterate directly over the guard, no clone needed
        for _ in 0..MAX_ITERATIONS {
            self.mcts_iteration(&tools_guard, query, context);
        }

        // Get best tool from root's children
        let tree = self.tree.read();
        let root = &tree[0];
        let mut child_scores: Vec<_> = Vec::with_capacity(root.children.len());
        for (idx, child) in root.children.iter().enumerate() {
            child_scores.push((idx, child));
        }
        child_scores.sort_by(|a, b| {
            let score_a = if a.1.visit_count > 0 {
                a.1.q_value / a.1.visit_count as f64
            } else {
                0.0
            };
            let score_b = if b.1.visit_count > 0 {
                b.1.q_value / b.1.visit_count as f64
            } else {
                0.0
            };
            score_b.partial_cmp(&score_a).unwrap()
        });

        if child_scores.is_empty() {
            return ToolSelection {
                tool_name: tools[0].name.clone(),
                confidence: 0.5,
                reasoning: "Fallback to first available tool".to_string(),
                alternative_tools: vec![],
            };
        }

        let best_idx = child_scores[0].0;
        let best_child = &child_scores[0].1;
        let best_tool = best_child.tool.as_ref().unwrap();

        // Calculate confidence based on visit count
        let total_visits: u32 = root.children.iter().map(|c| c.visit_count).sum();
        let confidence = if total_visits > 0 {
            best_child.visit_count as f64 / total_visits as f64
        } else {
            0.5
        };

        // Build alternatives
        let alternative_tools: Vec<_> = child_scores
            .iter()
            .skip(1)
            .take(3)
            .map(|(_, c)| {
                let t = c.tool.as_ref().unwrap();
                let score = if c.visit_count > 0 {
                    c.q_value / c.visit_count as f64
                } else {
                    0.0
                };
                (t.name.clone(), score)
            })
            .collect();

        // Generate reasoning
        let effectiveness = self.tool_effectiveness.read();
        let eff_score = effectiveness.get(&best_tool.name).copied().unwrap_or(0.5);

        let reasoning = format!(
            "Selected {} (category: {:?}) based on {} MCTS iterations. Historical effectiveness: {:.2}",
            best_tool.name,
            best_tool.category,
            total_visits,
            eff_score
        );

        ToolSelection {
            tool_name: best_tool.name.clone(),
            confidence,
            reasoning,
            alternative_tools,
        }
    }

    /// Single MCTS iteration: Selection → Expansion → Simulation → Backpropagation
    fn mcts_iteration(&self, tools: &[Tool], query: &str, context: &str) {
        // Selection: find best leaf using UCB1
        let mut node_idx = 0;
        let mut path = vec![node_idx];

        // Selection phase - traverse until we find unexpanded node or leaf
        loop {
let tree = self.tree.read();
            let node = &tree[node_idx];

            // If leaf or not fully expanded, stop
            if node.is_leaf() || !node.is_fully_expanded(tools) {
                break;
            }

            // Find best child by UCB1
            let visit_count = node.visit_count;
            let best_child_idx = node
                .children
                .iter()
                .enumerate()
                .map(|(i, c)| (i, c.ucb1(visit_count)))
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .map(|(i, _)| node.children[i].tool.as_ref().unwrap().name.clone());

            // For simplicity, just pick the first unexplored
            if let Some(best) = best_child_idx {
                if let Some(idx) = node.children.iter().position(|c| {
                    c.tool.as_ref().map(|t| &t.name == &best).unwrap_or(false)
                }) {
                    node_idx = idx;
                    path.push(node_idx);
                } else {
                    break;
                }
            } else {
                break;
            }

            if tree[node_idx].depth >= MAX_DEPTH {
                break;
            }
        }

        // Simulation: evaluate the selected node BEFORE writing (needs consistent tree state)
        let reward = self.simulate_reward(query, context, &path);

        // Expansion + Backpropagation: single write lock for both phases
        // Consolidate: reduced from 2 write locks to 1
        let mut tree = self.tree.write();

        // Expansion: add new child if not at max depth
        if tree[node_idx].depth < MAX_DEPTH && !tree[node_idx].is_fully_expanded(tools) {
            // Find unexplored tools
            let explored_tools: HashSet<_> = tree[node_idx].children.iter()
                .filter_map(|c| c.tool.as_ref())
                .map(|t| t.name.clone())
                .collect();

            let unexplored: Vec<_> = tools
                .iter()
                .filter(|t| !explored_tools.contains(&t.name))
                .collect();

            if !unexplored.is_empty() {
                // Add one new child
                let new_tool = unexplored[0].clone();

                let current_depth = tree[node_idx].depth;
                let new_node = MctsNode::child(node_idx, new_tool.clone(), current_depth + 1);
                let new_idx = tree.len();
                tree.push(new_node);
                tree[node_idx].children.push(MctsNode::child(node_idx, new_tool, current_depth + 1));
                tree[node_idx].is_expanded = true;
                path.push(new_idx);
            }
        }

        // Backpropagation: update Q-values (same write lock, no lock acquisition needed)
        for idx in &path {
            tree[*idx].visit_count += 1;
            tree[*idx].q_value += reward;
        }
    }

    /// Simulate reward for a tool sequence
    fn simulate_reward(&self, query: &str, context: &str, path: &[usize]) -> f64 {
        let tree = self.tree.read();
        let effectiveness = self.tool_effectiveness.read();

        // Hoist to_lowercase outside loop - query is constant per call
        let query_lower = query.to_lowercase();

        let mut total_reward = 0.0;
        let mut prev_category: Option<ToolCategory> = None;

        for &idx in path {
            let node = &tree[idx];
            if let Some(ref tool) = node.tool {
                // Base effectiveness from history
                let hist_eff = effectiveness.get(&tool.name).copied().unwrap_or(0.5);

                // Category diversity bonus (avoid same category twice)
                let diversity_bonus = if prev_category.as_ref() == Some(&tool.category) {
                    -0.1
                } else {
                    0.1
                };

                // Query-tool relevance (simple keyword matching) - use cached lowercase if available
                let desc_lower_owned;
                let desc_lower: &str = match &tool.description_lower {
                    Some(dl) => dl,
                    None => {
                        // Fallback: compute on the fly (should be rare after warmup)
                        desc_lower_owned = tool.description.to_lowercase();
                        &desc_lower_owned
                    }
                };
                let query_relevance = if desc_lower.contains(&query_lower) {
                    0.2
                } else {
                    0.0
                };

                let reward = (hist_eff as f64) + diversity_bonus + query_relevance;
                total_reward += reward;
                prev_category = Some(tool.category.clone());
            }
        }

        total_reward / path.len() as f64
    }

    /// Calculate FORESIGHT score (pre-execution prediction) - ToolTree innovation
    /// This predicts how useful a tool will be before actually using it
    fn calculate_foresight(&self, tool: &Tool, query: &str, context: &str) -> f64 {
        let effectiveness = self.tool_effectiveness.read();

        // Historical effectiveness
        let hist_eff = effectiveness.get(&tool.name).copied().unwrap_or(0.5) as f64;

        // Query relevance (forward-looking) - use cached lowercase if available
        let query_lower = query.to_lowercase();
        let desc_lower_owned;
        let desc_lower: &str = match &tool.description_lower {
            Some(dl) => dl,
            None => {
                desc_lower_owned = tool.description.to_lowercase();
                &desc_lower_owned
            }
        };
        let query_relevance = if desc_lower.contains(&query_lower) || query_lower.contains(&desc_lower[..10.min(desc_lower.len())]) {
            0.3
        } else {
            0.0
        };

        // Context relevance (how well this tool handles the current context)
        let context_relevance = if context.len() > 100 {
            // Larger context = higher chance tool will help
            0.1
        } else {
            0.05
        };

        // Tool cost efficiency
        let cost_efficiency = 1.0 - (tool.estimated_cost as f64 * 0.5);

        // Combine into foresight score (predicted reward)
        0.4 * hist_eff + 0.3 * query_relevance + 0.2 * context_relevance + 0.1 * cost_efficiency
    }

    /// Update HINDSIGHT score (post-execution evaluation) - ToolTree innovation
    /// This updates the node with actual observed performance
    fn update_hindsight(&self, node_idx: usize, actual_reward: f64) {
        let mut tree = self.tree.write();
        if let Some(node) = tree.get_mut(node_idx) {
            node.hindsight_score = actual_reward;
            node.is_executed = true;
        }
    }

    /// Calculate combined score using FORESIGHT + HINDSIGHT dual evaluation
    /// This is the ToolTree innovation: bidirectional pruning
    fn calculate_dual_score(&self, node_idx: usize) -> f64 {
        let tree = self.tree.read();
        if let Some(node) = tree.get(node_idx) {
            if node.is_executed {
                // Use hindsight (actual observed)
                0.7 * node.hindsight_score + 0.3 * node.foresight_score
            } else {
                // Use foresight (predicted) for unexplored nodes
                node.foresight_score
            }
        } else {
            0.0
        }
    }

    /// Report actual execution result for a tool (used for hindsight learning)
    pub fn report_execution_result(&self, tool_name: &str, result: &str, success: bool) {
        // Update tool effectiveness based on actual result
        let reward = if success {
            // Parse result quality (simplified)
            if result.len() > 100 {
                0.9 // Good detailed result
            } else if result.len() > 50 {
                0.7 // Medium result
            } else {
                0.5 // Minimal result
            }
        } else {
            0.2 // Failed execution
        };

        self.update_effectiveness(tool_name, reward as f32);

        // Update nodes that used this tool with hindsight
        let mut tree = self.tree.write();
        for node in tree.iter_mut() {
            if let Some(ref tool) = node.tool {
                if tool.name == tool_name {
                    node.hindsight_score = reward;
                    node.is_executed = true;
                }
            }
        }
    }

    /// Get the full search tree for visualization
    pub fn get_tree(&self) -> Vec<MctsNode> {
        self.tree.read().clone()
    }

    /// Clear the search tree (but keep tools)
    pub fn reset(&self) {
        let mut tree = self.tree.write();
        tree.clear();
        tree.push(MctsNode::root());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: "materials_project".to_string(),
                description: "Query materials database for thermoelectric properties".to_string(),
                category: ToolCategory::Database,
                input_schema: HashMap::new(),
                estimated_cost: 0.2,
            },
            Tool {
                name: "cgcnn_predict".to_string(),
                description: "Use graph neural network to predict crystal properties".to_string(),
                category: ToolCategory::Simulation,
                input_schema: HashMap::new(),
                estimated_cost: 0.5,
            },
            Tool {
                name: "文献检索".to_string(),
                description: "Search literature for related work".to_string(),
                category: ToolCategory::Literature,
                input_schema: HashMap::new(),
                estimated_cost: 0.3,
            },
        ]
    }

    #[test]
    fn test_mcts_planner_creation() {
        let planner = MctsPlanner::new();
        assert!(planner.tools.read().is_empty());
    }

    #[test]
    fn test_register_tools() {
        let planner = MctsPlanner::new();
        planner.register_tools(sample_tools());
        assert_eq!(planner.tools.read().len(), 3);
    }

    #[test]
    fn test_select_tools() {
        let planner = MctsPlanner::new();
        planner.register_tools(sample_tools());

        let selection = planner.select_tools("thermoelectric", "Bi2Te3 doping");
        assert!(!selection.tool_name.is_empty());
        assert!(selection.confidence >= 0.0 && selection.confidence <= 1.0);
    }

    #[test]
    fn test_update_effectiveness() {
        let planner = MctsPlanner::new();
        planner.register_tools(sample_tools());

        planner.update_effectiveness("materials_project", 0.8);
        let eff = planner.tool_effectiveness.read();
        assert!(*eff.get("materials_project").unwrap() > 0.7);
    }

    #[test]
    fn test_reset() {
        let planner = MctsPlanner::new();
        planner.register_tools(sample_tools());
        planner.select_tools("test", "context");

        planner.reset();
        assert_eq!(planner.tree.read().len(), 1); // Only root
    }
}