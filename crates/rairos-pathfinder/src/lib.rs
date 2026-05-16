//! rairos-pathfinder — Research Reading Path Planner.
//!
//! Ported from `llm/research/research_path.py` + `cli/cmd/path.py`.
//! Generates optimal research reading paths from citation graph using
//! PageRank + topological sort + intent detection.
//!
//! Zero persistent state — pure computation over KG + DB data.

#![allow(clippy::print_literal)]

use rairos_core::Database;
use chrono::Datelike;
use rairos_kg::KnowledgeGraph;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// Reading level / depth preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadingLevel {
    #[serde(rename = "intro")]
    Intro,
    #[serde(rename = "intermediate")]
    Intermediate,
    #[serde(rename = "advanced")]
    Advanced,
}

impl ReadingLevel {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "intro" | "beginner" | "入门" => Some(ReadingLevel::Intro),
            "intermediate" | "进阶" => Some(ReadingLevel::Intermediate),
            "advanced" | "深入" | "deep" => Some(ReadingLevel::Advanced),
            _ => None,
        }
    }
}

/// Represents a paper in the reading path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperNode {
    pub paper_id: String,
    pub title: String,
    pub year: i32,
    pub authors: Vec<String>,
    /// Papers citing this paper (by paper_id).
    pub cited_by: Vec<String>,
    /// Papers this paper cites (by paper_id).
    pub cites: Vec<String>,
    pub relevance_score: f64,
    pub pagerank: f64,
    pub is_foundational: bool,
    pub is_milestone: bool,
}

/// A single step in the reading path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingStep {
    pub order: usize,
    pub paper: PaperNode,
    pub role: String,
    pub reason: String,
    pub estimated_read_time_minutes: u32,
}

/// A complete reading path recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingPath {
    pub topic: String,
    pub level: ReadingLevel,
    pub total_papers: usize,
    pub total_reading_time_minutes: u32,
    pub steps: Vec<ReadingStep>,
    #[serde(default)]
    pub skipped_papers: Vec<String>,
}

// ============================================================================
// ResearchPathPlanner
// ============================================================================

/// Generate optimal research reading paths from citation graph.
pub struct ResearchPathPlanner<'a> {
    pub kg: Option<&'a KnowledgeGraph>,
    pub db: Option<&'a Database>,
}

impl<'a> ResearchPathPlanner<'a> {
    pub fn new(kg: Option<&'a KnowledgeGraph>, db: Option<&'a Database>) -> Self {
        Self { kg, db }
    }

    /// Generate an optimal reading path for a topic.
    pub fn plan_path(
        &self,
        topic: &str,
        level: ReadingLevel,
        max_papers: usize,
        min_year: Option<i32>,
        max_year: Option<i32>,
    ) -> ReadingPath {
        // 1. Find papers related to topic
        let papers = self.find_topic_papers(topic, min_year, max_year);
        if papers.is_empty() {
            return empty_path(topic, level);
        }

        // 2. Build citation graph
        let mut graph = self.build_citation_graph(papers);

        // 3. Calculate PageRank
        calculate_pagerank(&mut graph);

        // 4. Identify foundational and milestone papers
        identify_key_papers(&mut graph);

        // 5. Topological sort for optimal reading order
        let ordered = topological_sort(&graph);

        // 6. Generate reading steps
        let all_steps = generate_steps(&ordered, topic);

        // Limit to max_papers
        let (steps, skipped) = if all_steps.len() > max_papers {
            let (keep, skip) = all_steps.split_at(max_papers);
            (keep.to_vec(), skip.iter().map(|s| s.paper.title.clone()).collect())
        } else {
            (all_steps, Vec::new())
        };

        let total_time: u32 = steps.iter().map(|s| s.estimated_read_time_minutes).sum();

        ReadingPath {
            topic: topic.to_string(),
            level,
            total_papers: steps.len(),
            total_reading_time_minutes: total_time,
            steps,
            skipped_papers: skipped,
        }
    }

    /// Find papers related to topic from KG and DB.
    fn find_topic_papers(
        &self,
        topic: &str,
        min_year: Option<i32>,
        max_year: Option<i32>,
    ) -> Vec<PaperNode> {
        let mut seen = std::collections::HashSet::new();
        let mut papers: Vec<PaperNode> = Vec::new();
        let topic_lower = topic.to_lowercase();

        // 1. Search KG by iterating Paper nodes and matching label
        if let Some(kg) = self.kg {
            for node in kg.nodes().values() {
                if node.node_type != "paper" {
                    continue;
                }
                if !node.label.to_lowercase().contains(&topic_lower) {
                    continue;
                }
                if seen.contains(&node.entity_id) {
                    continue;
                }
                seen.insert(node.entity_id.clone());

                let year = extract_year_from_props(&node.properties);
                if min_year.is_none_or(|my| year >= my)
                    && max_year.is_none_or(|my| year <= my)
                {
                    let mut pn = PaperNode {
                        paper_id: node.entity_id.clone(),
                        title: node.label.clone(),
                        year,
                        authors: extract_authors_from_props(&node.properties),
                        cited_by: Vec::new(),
                        cites: Vec::new(),
                        relevance_score: 0.6,
                        pagerank: 0.0,
                        is_foundational: false,
                        is_milestone: false,
                    };

                    // Enrich citation data from KG
                    for citing in kg.get_citing(&node.id) {
                        if seen.contains(&citing.entity_id) {
                            pn.cited_by.push(citing.entity_id.clone());
                        }
                    }
                    for refd in kg.get_references(&node.id) {
                        if seen.contains(&refd.entity_id) {
                            pn.cites.push(refd.entity_id.clone());
                        }
                    }

                    papers.push(pn);
                }
            }
        }

        // 2. Search DB by FTS
        if let Some(db) = self.db {
            if let Ok(rows) = db.search_papers(topic, 50) {
                for row in &rows {
                    if seen.contains(&row.id) {
                        continue;
                    }
                    seen.insert(row.id.clone());

                    let year_val = row.published.year();
                    if min_year.is_none_or(|my| year_val >= my)
                        && max_year.is_none_or(|my| year_val <= my)
                    {
                        papers.push(PaperNode {
                            paper_id: row.id.clone(),
                            title: row.title.clone(),
                            year: year_val,
                            authors: row.authors.clone(),
                            cited_by: Vec::new(),
                            cites: Vec::new(),
                            relevance_score: 0.5,
                            pagerank: 0.0,
                            is_foundational: false,
                            is_milestone: false,
                        });
                    }
                }
            }
        }

        papers
    }

    /// Build citation graph, enriching with KG citation data.
    fn build_citation_graph(&self, papers: Vec<PaperNode>) -> HashMap<String, PaperNode> {
        let mut graph: HashMap<String, PaperNode> = papers.into_iter()
            .map(|p| (p.paper_id.clone(), p))
            .collect();

        // Enrich citation edges from KG
        if let Some(kg) = self.kg {
            // Collect citation edges first (separate from mutable borrow)
            let mut citation_edges: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();
            for kg_node in kg.nodes().values() {
                if !graph.contains_key(&kg_node.entity_id) {
                    continue;
                }
                let mut cited_by = Vec::new();
                let mut cites = Vec::new();
                for citing in kg.get_citing(&kg_node.id) {
                    if graph.contains_key(&citing.entity_id) {
                        cited_by.push(citing.entity_id.clone());
                    }
                }
                for refd in kg.get_references(&kg_node.id) {
                    if graph.contains_key(&refd.entity_id) {
                        cites.push(refd.entity_id.clone());
                    }
                }
                if !cited_by.is_empty() || !cites.is_empty() {
                    citation_edges.push((kg_node.entity_id.clone(), cited_by, cites));
                }
            }

            // Apply citation edges
            for (entity_id, cited_by, cites) in citation_edges {
                if let Some(paper) = graph.get_mut(&entity_id) {
                    paper.cited_by.extend(cited_by);
                    paper.cites.extend(cites);
                }
            }
        }

        graph
    }
}

// ============================================================================
// Algorithms
// ============================================================================

/// Calculate PageRank scores for papers in the citation graph.
fn calculate_pagerank(graph: &mut HashMap<String, PaperNode>) {
    let n = graph.len();
    if n == 0 {
        return;
    }

    let paper_ids: Vec<String> = graph.keys().cloned().collect();
    let idx: HashMap<String, usize> = paper_ids.iter().cloned().enumerate()
        .map(|(i, id)| (id, i))
        .collect();

    // Build adjacency matrix — adj[j][i] = 1 if paper j cites paper i
    // paper j (citing) has outgoing link to paper i (cited)
    let mut adj = vec![vec![0.0f64; n]; n];
    for paper in graph.values() {
        let Some(&i) = idx.get(&paper.paper_id) else { continue };
        for cited_by_pid in &paper.cited_by {
            if let Some(&j) = idx.get(cited_by_pid) {
                adj[j][i] = 1.0; // paper j cites paper i
            }
        }
    }

    // Normalize by out-degree
    for j in 0..n {
        let out_deg: f64 = adj[j].iter().sum();
        if out_deg > 0.0 {
            for i in 0..n {
                adj[j][i] /= out_deg;
            }
        }
    }

    // PageRank iteration
    let damping = 0.85;
    let max_iter = 30;
    let tol = 1e-6;
    let mut pr = vec![1.0 / n as f64; n];

    for _ in 0..max_iter {
        let mut new_pr = vec![(1.0 - damping) / n as f64; n];
        for i in 0..n {
            for j in 0..n {
                new_pr[i] += damping * adj[j][i] * pr[j];
            }
        }
        let diff: f64 = pr.iter().zip(new_pr.iter()).map(|(a, b)| (a - b).abs()).sum();
        pr = new_pr;
        if diff < tol {
            break;
        }
    }

    for (i, pid) in paper_ids.iter().enumerate() {
        if let Some(paper) = graph.get_mut(pid) {
            paper.pagerank = pr[i];
        }
    }
}

/// Identify foundational and milestone papers.
fn identify_key_papers(graph: &mut HashMap<String, PaperNode>) {
    let mut papers: Vec<&mut PaperNode> = graph.values_mut().collect();
    if papers.is_empty() {
        return;
    }

    papers.sort_by(|a, b| b.pagerank.partial_cmp(&a.pagerank).unwrap_or(std::cmp::Ordering::Equal));

    let top_count = std::cmp::max(1, papers.len() / 4);
    for p in papers.iter_mut().take(top_count) {
        p.is_foundational = true;
    }

    // Earliest papers with decent PageRank are milestones
    let with_year: Vec<(i32, f64, usize)> = papers.iter().enumerate()
        .filter(|(_, p)| p.year > 0)
        .map(|(i, p)| (p.year, p.pagerank, i))
        .collect();

    let mid_rank = papers.len() / 2;
    for (_year, pr, idx) in with_year.iter().take(mid_rank) {
        if *pr > 0.5 / papers.len() as f64 {
            papers[*idx].is_milestone = true;
        }
    }
}

/// Topological sort for reading order.
fn topological_sort(graph: &HashMap<String, PaperNode>) -> Vec<PaperNode> {
    if graph.is_empty() {
        return Vec::new();
    }

    // Build in-degree: how many papers in the graph cite this paper
    let mut in_degree: HashMap<String, usize> = graph.keys().map(|k| (k.clone(), 0)).collect();
    for paper in graph.values() {
        for cited in &paper.cited_by {
            if graph.contains_key(cited) {
                *in_degree.entry(paper.paper_id.clone()).or_insert(0) += 1;
            }
        }
    }

    let mut queue: Vec<String> = in_degree.iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(pid, _)| pid.clone())
        .collect();

    let mut result: Vec<PaperNode> = Vec::new();

    while !queue.is_empty() {
        queue.sort_by(|a, b| {
            let ga = &graph[a];
            let gb = &graph[b];
            if ga.is_foundational != gb.is_foundational {
                return gb.is_foundational.cmp(&ga.is_foundational);
            }
            let pr_cmp = gb.pagerank.partial_cmp(&ga.pagerank).unwrap_or(std::cmp::Ordering::Equal);
            if pr_cmp != std::cmp::Ordering::Equal {
                return pr_cmp;
            }
            ga.year.cmp(&gb.year)
        });

        let pid = queue.remove(0);
        result.push(graph[&pid].clone());

        for cited_pid in &graph[&pid].cites {
            if let Some(deg) = in_degree.get_mut(cited_pid) {
                if *deg > 0 {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(cited_pid.clone());
                    }
                }
            }
        }
    }

    // Handle cycles / disconnected nodes
    if result.len() < graph.len() {
        let mut remaining: Vec<&PaperNode> = graph.values()
            .filter(|p| !result.iter().any(|r| r.paper_id == p.paper_id))
            .collect();
        remaining.sort_by(|a, b| {
            let pr_cmp = b.pagerank.partial_cmp(&a.pagerank).unwrap_or(std::cmp::Ordering::Equal);
            if pr_cmp != std::cmp::Ordering::Equal {
                return pr_cmp;
            }
            a.year.cmp(&b.year)
        });
        for p in remaining {
            result.push(p.clone());
        }
    }

    result
}

/// Generate reading steps with roles and reasons.
fn generate_steps(papers: &[PaperNode], _topic: &str) -> Vec<ReadingStep> {
    let mut steps = Vec::new();
    let mut seen_years = std::collections::HashSet::new();

    for (i, paper) in papers.iter().enumerate() {
        let (role, reason) = assign_role(paper, i, &seen_years);
        steps.push(ReadingStep {
            order: i + 1,
            paper: paper.clone(),
            role,
            reason,
            estimated_read_time_minutes: estimate_read_time(paper),
        });
        if paper.year > 0 {
            seen_years.insert(paper.year);
        }
    }

    steps
}

fn assign_role(paper: &PaperNode, position: usize, _seen_years: &std::collections::HashSet<i32>) -> (String, String) {
    let year = paper.year;

    if paper.is_foundational && (year == 0 || year < 2018) {
        return ("foundation".to_string(), "开创性工作，奠定了该领域的基础".to_string());
    }
    if paper.is_foundational {
        return ("core".to_string(), "高影响力核心论文，必读".to_string());
    }
    if position == 0 {
        return ("core".to_string(), "作为入口论文，适合建立整体认知".to_string());
    }
    if year >= 2022 {
        return ("latest".to_string(), format!("最新进展（{year}年）"));
    }
    if paper.cited_by.len() > 2 {
        return ("improvement".to_string(), "被多篇后续论文引用，影响力较高".to_string());
    }
    ("improvement".to_string(), "该领域的改进/应用".to_string())
}

fn estimate_read_time(paper: &PaperNode) -> u32 {
    let mut base = 15u32;
    base += paper.title.len() as u32 / 50;
    if paper.cited_by.len() > 5 {
        base += 10;
    } else if paper.cited_by.len() > 2 {
        base += 5;
    }
    base.min(45)
}

// ============================================================================
// Rendering
// ============================================================================

/// Render reading path as formatted string.
pub fn render_path(path: &ReadingPath) -> String {
    if path.steps.is_empty() {
        return format!("📚 未找到关于「{}」的相关论文", path.topic);
    }

    let level_label = match path.level {
        ReadingLevel::Intro => "入门",
        ReadingLevel::Intermediate => "进阶",
        ReadingLevel::Advanced => "深入",
    };

    let mut lines = vec![
        format!("📚 《{}》阅读路径推荐", path.topic),
        format!("   难度: {} | 共 {} 篇 | 预计 {} 分钟", level_label, path.total_papers, path.total_reading_time_minutes),
        String::new(),
    ];

    let role_icons: HashMap<&str, &str> = [
        ("foundation", "🏛️"), ("core", "📖"), ("improvement", "⚡"),
        ("variant", "🔄"), ("latest", "✨"),
    ].iter().cloned().collect();

    for step in &path.steps {
        let icon = role_icons.get(step.role.as_str()).unwrap_or(&"📄");
        let year_str = if step.paper.year > 0 { format!("[{}]", step.paper.year) } else { String::new() };
        let title = if step.paper.title.len() > 50 {
            format!("{}...", &step.paper.title[..50])
        } else {
            step.paper.title.clone()
        };

        lines.push(format!("{}. {} {} {}", step.order, icon, year_str, title));
        lines.push(format!("   💡 {}", step.reason));
        if !step.paper.authors.is_empty() {
            let authors_str = if step.paper.authors.len() > 2 {
                format!("{}, {} et al.", step.paper.authors[0], step.paper.authors[1])
            } else {
                step.paper.authors.join(", ")
            };
            lines.push(format!("   👥 {}", authors_str));
        }
        lines.push(format!("   ⏱️ {} min", step.estimated_read_time_minutes));
        lines.push(String::new());
    }

    if !path.skipped_papers.is_empty() {
        lines.push(format!("💡 还有 {} 篇相关论文未显示", path.skipped_papers.len()));
    }

    lines.join("\n")
}

/// Render reading path as Mermaid graph.
pub fn render_mermaid(path: &ReadingPath) -> String {
    if path.steps.is_empty() {
        return r#"graph TD
    Empty["No papers found"]"#.to_string();
    }

    let mut lines = vec!["graph TD".to_string()];
    lines.push(r#"    subgraph "Reading Path""#.to_string());

    for step in &path.steps {
        let pid = step.paper.paper_id.replace('-', "_");
        let title = if step.paper.title.len() > 30 {
            format!("{}...", &step.paper.title[..30])
        } else {
            step.paper.title.clone()
        };
        lines.push(format!(r#"    {}_{}["{}. {}"]:::"{}""#, step.order, pid, step.order, title.replace('"', "'"), step.role));
    }

    for i in 0..path.steps.len().saturating_sub(1) {
        let curr = path.steps[i].paper.paper_id.replace('-', "_");
        let next_pid = path.steps[i + 1].paper.paper_id.replace('-', "_");
        lines.push(format!("    {}_{} --> {}_{}", i + 1, curr, i + 2, next_pid));
    }

    lines.push("    end".to_string());
    lines.push(String::new());
    lines.push("    classDef foundation fill:#f9f,stroke:#333".to_string());
    lines.push("    classDef core fill:#ff9,stroke:#333".to_string());
    lines.push("    classDef improvement fill:#9f9,stroke:#333".to_string());
    lines.push("    classDef latest fill:#9ff,stroke:#333".to_string());

    lines.join("\n")
}

// ============================================================================
// Helpers
// ============================================================================

fn empty_path(topic: &str, level: ReadingLevel) -> ReadingPath {
    ReadingPath {
        topic: topic.to_string(),
        level,
        total_papers: 0,
        total_reading_time_minutes: 0,
        steps: Vec::new(),
        skipped_papers: Vec::new(),
    }
}

fn extract_year_from_props(props: &serde_json::Value) -> i32 {
    if let Some(year_val) = props.get("year") {
        if let Some(y) = year_val.as_i64() { return y as i32; }
        if let Some(s) = year_val.as_str() { if s.len() >= 4 { if let Ok(y) = s[..4].parse() { return y; } } }
    }
    if let Some(pub_val) = props.get("published") {
        if let Some(s) = pub_val.as_str() { if s.len() >= 4 { if let Ok(y) = s[..4].parse() { return y; } } }
    }
    0
}

fn extract_authors_from_props(props: &serde_json::Value) -> Vec<String> {
    if let Some(authors_val) = props.get("authors") {
        if let Some(arr) = authors_val.as_array() {
            return arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
        }
        if let Some(s) = authors_val.as_str() {
            if !s.is_empty() {
                return s.split(',').map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect();
            }
        }
    }
    Vec::new()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_paper(id: &str, title: &str, year: i32) -> PaperNode {
        PaperNode {
            paper_id: id.to_string(),
            title: title.to_string(),
            year,
            authors: Vec::new(),
            cited_by: Vec::new(),
            cites: Vec::new(),
            relevance_score: 0.5,
            pagerank: 0.0,
            is_foundational: false,
            is_milestone: false,
        }
    }

    #[test]
    fn test_reading_level_from_str() {
        assert_eq!(ReadingLevel::from_str("intro"), Some(ReadingLevel::Intro));
        assert_eq!(ReadingLevel::from_str("intermediate"), Some(ReadingLevel::Intermediate));
        assert_eq!(ReadingLevel::from_str("advanced"), Some(ReadingLevel::Advanced));
        assert_eq!(ReadingLevel::from_str("入门"), Some(ReadingLevel::Intro));
        assert_eq!(ReadingLevel::from_str("进阶"), Some(ReadingLevel::Intermediate));
        assert_eq!(ReadingLevel::from_str("invalid"), None);
    }

    #[test]
    fn test_empty_path() {
        let path = empty_path("test", ReadingLevel::Intermediate);
        assert_eq!(path.topic, "test");
        assert_eq!(path.total_papers, 0);
    }

    #[test]
    fn test_render_empty_path() {
        let output = render_path(&empty_path("量子计算", ReadingLevel::Intro));
        assert!(output.contains("未找到"));
        assert!(output.contains("量子计算"));
    }

    #[test]
    fn test_render_mermaid_empty() {
        let output = render_mermaid(&empty_path("test", ReadingLevel::Intermediate));
        assert!(output.contains("No papers found"));
    }

    #[test]
    fn test_pagerank_three_papers() {
        let mut graph = HashMap::new();
        graph.insert("A".to_string(), make_paper("A", "Paper A", 2020));
        graph.insert("B".to_string(), make_paper("B", "Paper B", 2021));
        graph.insert("C".to_string(), make_paper("C", "Paper C", 2022));

        // B and C both cite A
        graph.get_mut("A").unwrap().cited_by.push("B".to_string());
        graph.get_mut("A").unwrap().cited_by.push("C".to_string());
        // C cites B
        graph.get_mut("B").unwrap().cited_by.push("C".to_string());

        calculate_pagerank(&mut graph);

        assert!(graph["A"].pagerank > graph["B"].pagerank);
        assert!(graph["B"].pagerank > graph["C"].pagerank);
        assert!(graph["A"].pagerank > 0.0);
    }

    #[test]
    fn test_pagerank_empty() {
        let mut graph = HashMap::new();
        calculate_pagerank(&mut graph);
    }

    #[test]
    fn test_pagerank_single() {
        let mut graph = HashMap::new();
        graph.insert("A".to_string(), make_paper("A", "Solo", 2020));
        calculate_pagerank(&mut graph);
        // Single node with no edges: rank converges to (1-damping)/n
        assert!((graph["A"].pagerank - 0.15).abs() < 1e-6);
    }

    #[test]
    fn test_topological_sort_simple() {
        let mut graph = HashMap::new();
        graph.insert("A".to_string(), { let mut p = make_paper("A", "Foundational", 2018); p.is_foundational = true; p.pagerank = 0.5; p });
        graph.insert("B".to_string(), { let mut p = make_paper("B", "Improves A", 2020); p.cites.push("A".to_string()); p.pagerank = 0.3; p });
        graph.insert("C".to_string(), { let mut p = make_paper("C", "Latest", 2022); p.cites.push("B".to_string()); p.cited_by.push("A".to_string()); p.pagerank = 0.2; p });

        // Fix: B is cited by C
        graph.get_mut("B").unwrap().cited_by.push("C".to_string());

        let sorted = topological_sort(&graph);
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].paper_id, "A", "Foundational paper should be first");

        let pos_b = sorted.iter().position(|p| p.paper_id == "B").unwrap();
        let pos_c = sorted.iter().position(|p| p.paper_id == "C").unwrap();
        assert!(pos_b < pos_c, "B before C in topological order");
    }

    #[test]
    fn test_generate_steps_empty() {
        assert!(generate_steps(&[], "test").is_empty());
    }

    #[test]
    fn test_generate_steps_single() {
        let steps = generate_steps(&[make_paper("p1", "Test Paper", 2023)], "test");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].order, 1);
    }

    #[test]
    fn test_assign_role_foundational_old() {
        let p = PaperNode { is_foundational: true, year: 2017, ..make_paper("1", "Old", 2017) };
        assert_eq!(assign_role(&p, 0, &std::collections::HashSet::new()).0, "foundation");
    }

    #[test]
    fn test_assign_role_latest() {
        let p = make_paper("1", "New", 2023);
        assert_eq!(assign_role(&p, 2, &std::collections::HashSet::new()).0, "latest");
    }

    #[test]
    fn test_estimate_read_time() {
        assert!(estimate_read_time(&make_paper("1", "Short", 2020)) >= 15);
        assert!(estimate_read_time(&make_paper("1", "Short", 2020)) <= 45);
    }

    #[test]
    fn test_estimate_read_time_highly_cited() {
        let p = PaperNode { cited_by: ["a","b","c","d","e","f"].iter().map(|s| s.to_string()).collect(), ..make_paper("1", "Popular", 2020) };
        assert!(estimate_read_time(&p) >= 25);
    }

    #[test]
    fn test_identify_key_papers() {
        let mut graph = HashMap::new();
        graph.insert("A".to_string(), { let mut p = make_paper("A", "High Rank", 2018); p.pagerank = 0.5; p });
        graph.insert("B".to_string(), { let mut p = make_paper("B", "Low Rank", 2020); p.pagerank = 0.1; p });
        identify_key_papers(&mut graph);
        assert!(graph["A"].is_foundational);
        assert!(!graph["B"].is_foundational);
    }

    #[test]
    fn test_render_path_with_steps() {
        let step = ReadingStep {
            order: 1,
            paper: make_paper("2301.00001", "Attention Is All You Need", 2017),
            role: "foundation".to_string(),
            reason: "开创性工作".to_string(),
            estimated_read_time_minutes: 30,
        };
        let path = ReadingPath { topic: "Transformer".to_string(), level: ReadingLevel::Intermediate, total_papers: 1, total_reading_time_minutes: 30, steps: vec![step], skipped_papers: Vec::new() };
        let output = render_path(&path);
        assert!(output.contains("Transformer"));
        assert!(output.contains("Attention Is All You Need"));
    }

    #[test]
    fn test_render_mermaid_with_steps() {
        let step = ReadingStep { order: 1, paper: make_paper("2301.00001", "Test", 2023), role: "core".to_string(), reason: "必读".to_string(), estimated_read_time_minutes: 15 };
        let path = ReadingPath { topic: "Test".to_string(), level: ReadingLevel::Advanced, total_papers: 1, total_reading_time_minutes: 15, steps: vec![step], skipped_papers: Vec::new() };
        let output = render_mermaid(&path);
        assert!(output.contains("graph TD"));
        assert!(output.contains("core"));
    }

    #[test]
    fn test_extract_year() {
        assert_eq!(extract_year_from_props(&serde_json::json!({"published": "2023-04-15"})), 2023);
        assert_eq!(extract_year_from_props(&serde_json::json!({"year": 2022})), 2022);
        assert_eq!(extract_year_from_props(&serde_json::json!({})), 0);
    }

    #[test]
    fn test_extract_authors() {
        assert_eq!(extract_authors_from_props(&serde_json::json!({"authors": ["Alice", "Bob"]})).len(), 2);
        assert_eq!(extract_authors_from_props(&serde_json::json!({"authors": "Alice, Bob"})).len(), 2);
        assert_eq!(extract_authors_from_props(&serde_json::json!({})).len(), 0);
    }

    #[test]
    fn test_serde_roundtrip() {
        let path = ReadingPath {
            topic: "AI".to_string(),
            level: ReadingLevel::Advanced,
            total_papers: 1,
            total_reading_time_minutes: 15,
            steps: vec![ReadingStep { order: 1, paper: make_paper("p1", "Test", 2023), role: "core".to_string(), reason: "必读".to_string(), estimated_read_time_minutes: 15 }],
            skipped_papers: Vec::new(),
        };
        let json = serde_json::to_string(&path).unwrap();
        let deser: ReadingPath = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.steps[0].paper.paper_id, "p1");
    }

    #[test]
    fn test_topological_sort_cycle() {
        let mut graph = HashMap::new();
        graph.insert("A".to_string(), { let mut p = make_paper("A", "A", 2020); p.cited_by.push("B".to_string()); p.cites.push("B".to_string()); p.pagerank = 0.3; p });
        graph.insert("B".to_string(), { let mut p = make_paper("B", "B", 2021); p.cited_by.push("A".to_string()); p.cites.push("A".to_string()); p.pagerank = 0.3; p });
        let sorted = topological_sort(&graph);
        assert_eq!(sorted.len(), 2);
    }
}
