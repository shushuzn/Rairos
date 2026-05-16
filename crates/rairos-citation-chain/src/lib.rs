#![allow(dead_code)]
#![allow(
    clippy::needless_range_loop,

    clippy::vec_init_then_push,
)]
#![allow(clippy::too_many_arguments)]
use std::collections::{HashMap, HashSet};
use std::vec::Vec;

#[derive(Debug, Clone)]
pub struct CitationNode {
    pub paper_id: String,
    pub title: String,
    pub year: i32,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub citations: Vec<String>,
    pub cited_by: Vec<String>,
    pub citation_count: i32,
}

impl CitationNode {
    pub fn new(paper_id: String, title: String) -> Self {
        Self {
            paper_id,
            title,
            year: 0,
            authors: Vec::new(),
            abstract_text: String::new(),
            citations: Vec::new(),
            cited_by: Vec::new(),
            citation_count: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CitationChain {
    pub nodes: Vec<CitationNode>,
    pub edges: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct ResearchFamily {
    pub family_id: String,
    pub ancestor_id: String,
    pub ancestor_title: String,
    pub papers: Vec<HashMap<String, serde_json::Value>>,
    pub common_theme: String,
    pub size: i32,
}

pub struct CitationChainBuilder {
    pub nodes: HashMap<String, CitationNode>,
}

impl CitationChainBuilder {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn add_paper(
        &mut self,
        paper_id: String,
        title: String,
        year: i32,
        authors: Vec<String>,
        references: Vec<String>,
        abstract_text: String,
        citation_count: i32,
    ) -> &CitationNode {
        if !self.nodes.contains_key(&paper_id) {
            self.nodes.insert(
                paper_id.clone(),
                CitationNode {
                    paper_id: paper_id.clone(),
                    title,
                    year,
                    authors,
                    abstract_text,
                    citations: references,
                    cited_by: Vec::new(),
                    citation_count,
                },
            );
        }
        self.nodes.get(&paper_id).unwrap()
    }

    pub fn link_citations(&mut self, from_id: &str, to_id: &str) {
        if let Some(from_node) = self.nodes.get_mut(from_id) {
            if !from_node.citations.contains(&to_id.to_string()) {
                from_node.citations.push(to_id.to_string());
            }
        }
        if let Some(to_node) = self.nodes.get_mut(to_id) {
            if !to_node.cited_by.contains(&from_id.to_string()) {
                to_node.cited_by.push(from_id.to_string());
            }
        }
    }

    pub fn build_from_db(&mut self, paper_id: &str, depth: i32) -> CitationChain {
        self.nodes.clear();
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: Vec<(String, i32)> = vec![(paper_id.to_string(), 0)];

        while let Some((pid, d)) = queue.pop() {
            if visited.contains(&pid) || d > depth {
                continue;
            }
            visited.insert(pid.clone());

            let node = self.nodes.get(&pid);
            if let Some(n) = node {
                let refs = n.citations.clone();
                if d < depth {
                    for ref_id in &refs {
                        if !visited.contains(ref_id) {
                            queue.push((ref_id.clone(), d + 1));
                        }
                    }
                }
            }
        }

        let mut edges: Vec<(String, String)> = Vec::new();
        for node in self.nodes.values() {
            for cited in &node.citations {
                if self.nodes.contains_key(cited) {
                    edges.push((node.paper_id.clone(), cited.clone()));
                }
            }
        }

        CitationChain {
            nodes: self.nodes.values().cloned().collect(),
            edges,
        }
    }

    pub fn find_path(&self, from_id: &str, to_id: &str) -> Option<Vec<String>> {
        if !self.nodes.contains_key(from_id) || !self.nodes.contains_key(to_id) {
            return None;
        }
        if from_id == to_id {
            return Some(vec![from_id.to_string()]);
        }

        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: Vec<Vec<String>> = vec![vec![from_id.to_string()]];

        while let Some(path) = queue.pop() {
            let current = path.last().unwrap();
            let neighbors: Vec<String> = {
                let mut n = Vec::new();
                if let Some(node) = self.nodes.get(current) {
                    n.extend(node.citations.clone());
                    n.extend(node.cited_by.clone());
                }
                n
            };

            for neighbor in neighbors {
                if neighbor == to_id {
                    let mut result = path.clone();
                    result.push(neighbor);
                    return Some(result);
                }
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor.clone());
                    let mut new_path = path.clone();
                    new_path.push(neighbor);
                    queue.push(new_path);
                }
            }
        }
        None
    }

    pub fn find_influencers(&self, paper_id: &str, depth: i32) -> Vec<CitationNode> {
        if !self.nodes.contains_key(paper_id) {
            return Vec::new();
        }

        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: Vec<(String, i32)> = vec![(paper_id.to_string(), 0)];
        let mut ancestors: Vec<CitationNode> = Vec::new();

        while let Some((pid, d)) = queue.pop() {
            if d > depth {
                continue;
            }
            if let Some(node) = self.nodes.get(&pid) {
                for ancestor_id in &node.cited_by {
                    if !visited.contains(ancestor_id) {
                        visited.insert(ancestor_id.clone());
                        if let Some(ancestor) = self.nodes.get(ancestor_id) {
                            ancestors.push(ancestor.clone());
                        }
                        queue.push((ancestor_id.clone(), d + 1));
                    }
                }
            }
        }
        ancestors
    }

    pub fn find_impact(&self, paper_id: &str, depth: i32) -> Vec<CitationNode> {
        if !self.nodes.contains_key(paper_id) {
            return Vec::new();
        }

        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: Vec<(String, i32)> = vec![(paper_id.to_string(), 0)];
        let mut descendants: Vec<CitationNode> = Vec::new();

        while let Some((pid, d)) = queue.pop() {
            if d > depth {
                continue;
            }
            if let Some(node) = self.nodes.get(&pid) {
                for descendant_id in &node.citations {
                    if !visited.contains(descendant_id) {
                        visited.insert(descendant_id.clone());
                        if let Some(descendant) = self.nodes.get(descendant_id) {
                            descendants.push(descendant.clone());
                        }
                        queue.push((descendant_id.clone(), d + 1));
                    }
                }
            }
        }
        descendants
    }

    pub fn cluster_families(&self) -> Vec<ResearchFamily> {
        let mut families: Vec<ResearchFamily> = Vec::new();
        let mut ref_to_papers: HashMap<String, HashSet<String>> = HashMap::new();

        for node in self.nodes.values() {
            for reference in &node.citations {
                ref_to_papers
                    .entry(reference.clone())
                    .or_default()
                    .insert(node.paper_id.clone());
            }
        }

        let mut seen_pairs: HashSet<(String, String)> = HashSet::new();

        for node in self.nodes.values() {
            if node.citations.is_empty() {
                continue;
            }

            let mut family_members: HashMap<String, HashSet<String>> = HashMap::new();
            for reference in &node.citations {
                if let Some(other_pids) = ref_to_papers.get(reference) {
                    for other_pid in other_pids {
                        if other_pid != &node.paper_id {
                            family_members
                                .entry(other_pid.clone())
                                .or_default()
                                .insert(reference.clone());
                        }
                    }
                }
            }

            for (other_pid, shared_refs) in family_members {
                if shared_refs.len() >= 2 {
                    let pair_key = (node.paper_id.clone(), other_pid.clone());
                    let sorted_pair = (pair_key.0.clone(), pair_key.1.clone());
                    if seen_pairs.contains(&sorted_pair) {
                        continue;
                    }
                    seen_pairs.insert(sorted_pair);

                    let other_node = self.nodes.get(&other_pid);
                    let shared_refs_vec: Vec<String> =
                        shared_refs.iter().take(3).cloned().collect();
                    let family_id = format!("{:06}", families.len() + 1);

                    families.push(ResearchFamily {
                        family_id,
                        ancestor_id: node.paper_id.clone(),
                        ancestor_title: format!("Family sharing: {}", shared_refs_vec.join(", ")),
                        papers: vec![
                            {
                                let mut m = HashMap::new();
                                m.insert("paper_id".to_string(), serde_json::json!(node.paper_id));
                                m.insert(
                                    "title".to_string(),
                                    serde_json::json!(node.title.clone()),
                                );
                                m.insert("year".to_string(), serde_json::json!(node.year));
                                m
                            },
                            {
                                let mut m = HashMap::new();
                                m.insert(
                                    "paper_id".to_string(),
                                    serde_json::json!(other_pid.clone()),
                                );
                                m.insert(
                                    "title".to_string(),
                                    serde_json::json!(other_node
                                        .map(|n| &n.title)
                                        .unwrap_or(&other_pid)
                                        .clone()),
                                );
                                m.insert(
                                    "year".to_string(),
                                    serde_json::json!(other_node.map(|n| n.year).unwrap_or(0)),
                                );
                                m
                            },
                        ],
                        common_theme: format!("Shared references: {}", shared_refs_vec.join(", ")),
                        size: 2,
                    });
                }
            }
        }

        families.truncate(10);
        families
    }

    pub fn detect_silent_citations(&self) -> Vec<HashMap<String, serde_json::Value>> {
        let method_terms: HashSet<&str> = [
            "transformer",
            "attention",
            "neural",
            "network",
            "embedding",
            "latent",
            "fine-tuning",
            "pretraining",
            "gradient",
            "loss",
            "optimization",
            "encoder",
            "decoder",
            "architecture",
            "layer",
            "token",
            "rag",
            "retrieval",
            "knowledge",
            "distillation",
            "quantization",
            "chain-of-thought",
            "prompting",
            "few-shot",
            "zero-shot",
            "in-context",
            "reinforcement",
            "reward",
            "policy",
            "rlhf",
            "dpo",
            "graph",
            "convolutional",
            "recurrent",
            "generative",
            "diffusion",
            "gan",
            "vae",
            "autoencoder",
        ]
        .iter()
        .cloned()
        .collect();

        let mut silent: Vec<HashMap<String, serde_json::Value>> = Vec::new();
        let nodes: Vec<&CitationNode> = self.nodes.values().collect();

        for i in 0..nodes.len() {
            let node = nodes[i];
            let node_lower = node.abstract_text.to_lowercase();
            let node_terms: HashSet<String> = node_lower
                .split(|c: char| !c.is_alphanumeric())
                .filter(|w| w.len() > 3 && method_terms.contains(w))
                .map(|w| w.to_string())
                .collect();

            if node_terms.is_empty() {
                continue;
            }

            for j in (i + 1)..nodes.len() {
                let other = nodes[j];

                if node.citations.contains(&other.paper_id)
                    || other.citations.contains(&node.paper_id)
                {
                    continue;
                }

                let other_lower = other.abstract_text.to_lowercase();
                let other_terms: HashSet<String> = other_lower
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|w| w.len() > 3 && method_terms.contains(w))
                    .map(|w| w.to_string())
                    .collect();

                if other_terms.is_empty() {
                    continue;
                }

                let shared: Vec<String> = node_terms.intersection(&other_terms).cloned().collect();

                if shared.len() >= 4 {
                    let newer = if other.year > node.year { other } else { node };
                    let older = if other.year > node.year { node } else { other };
                    let confidence = (shared.len() as f32 / 10.0).min(0.95);

                    silent.push({
                        let mut m = HashMap::new();
                        m.insert(
                            "newer_arxiv_id".to_string(),
                            serde_json::json!(newer.paper_id),
                        );
                        m.insert(
                            "newer_title".to_string(),
                            serde_json::json!(newer.title.clone()),
                        );
                        m.insert("newer_year".to_string(), serde_json::json!(newer.year));
                        m.insert(
                            "older_arxiv_id".to_string(),
                            serde_json::json!(older.paper_id),
                        );
                        m.insert(
                            "older_title".to_string(),
                            serde_json::json!(older.title.clone()),
                        );
                        m.insert("older_year".to_string(), serde_json::json!(older.year));
                        m.insert("shared_methods".to_string(), serde_json::json!(shared));
                        m.insert("confidence".to_string(), serde_json::json!(confidence));
                        m.insert(
                            "note".to_string(),
                            serde_json::json!(format!(
                                "{} uses similar methods to {} but may not cite it",
                                &newer.paper_id[..8.min(newer.paper_id.len())],
                                &older.paper_id[..8.min(older.paper_id.len())]
                            )),
                        );
                        m
                    });
                }
            }
        }

        silent.sort_by(|a, b| {
            let conf_a = a.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let conf_b = b.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
            conf_b.partial_cmp(&conf_a).unwrap()
        });
        silent.truncate(10);
        silent
    }

    pub fn render_text(&self, chain: &CitationChain, max_nodes: usize) -> String {
        if chain.nodes.is_empty() {
            return "No citation chain.".to_string();
        }

        let mut lines: Vec<String> = Vec::new();
        lines.push("=".repeat(60));
        lines.push("Citation Chain".to_string());
        lines.push("=".repeat(60));
        lines.push(String::new());

        let mut sorted_nodes = chain.nodes.clone();
        sorted_nodes.sort_by_key(|x| std::cmp::Reverse(x.year));

        for node in sorted_nodes.iter().take(max_nodes) {
            lines.push(format!(
                "[{}] {}",
                &node.paper_id[..node.paper_id.len().min(8)],
                &node.title[..node.title.len().min(50)]
            ));
            lines.push(format!(
                "  Year: {} | Cites: {} | Cited by: {}",
                if node.year > 0 {
                    node.year.to_string()
                } else {
                    "?".to_string()
                },
                node.citations.len(),
                node.cited_by.len()
            ));
            lines.push(String::new());
        }

        if chain.nodes.len() > max_nodes {
            lines.push(format!(
                "... and {} more papers",
                chain.nodes.len() - max_nodes
            ));
        }

        lines.push(String::new());
        lines.push(format!(
            "Total: {} papers, {} connections",
            chain.nodes.len(),
            chain.edges.len()
        ));
        lines.push("=".repeat(60));

        lines.join("\n")
    }

    pub fn render_graphviz(&self, chain: &CitationChain) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push("digraph citations {".to_string());
        lines.push("  rankdir=LR;".to_string());
        lines.push("  node [shape=box];".to_string());

        for node in &chain.nodes {
            let label = if node.year > 0 {
                format!(
                    "{}...\\n({})",
                    &node.title[..node.title.len().min(30)],
                    node.year
                )
            } else {
                node.title[..node.title.len().min(30)].to_string()
            };
            lines.push(format!("  \"{}\" [label=\"{}\"];", node.paper_id, label));
        }

        for (from_id, to_id) in &chain.edges {
            lines.push(format!("  \"{}\" -> \"{}\";", from_id, to_id));
        }

        lines.push("}".to_string());
        lines.join("\n")
    }

    pub fn render_mermaid(&self, chain: &CitationChain) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push("```mermaid".to_string());
        lines.push("flowchart LR".to_string());

        for node in &chain.nodes {
            let year_str = if node.year > 0 {
                format!("({})", node.year)
            } else {
                String::new()
            };
            lines.push(format!(
                "    {}[{}{}]",
                &node.paper_id[..node.paper_id.len().min(8)],
                &node.title[..node.title.len().min(30)],
                year_str
            ));
        }

        for (from_id, to_id) in &chain.edges {
            lines.push(format!(
                "    {} --> {}",
                &from_id[..from_id.len().min(8)],
                &to_id[..to_id.len().min(8)]
            ));
        }

        lines.push("```".to_string());
        lines.join("\n")
    }

    pub fn render_families(&self, families: &[ResearchFamily]) -> String {
        if families.is_empty() {
            return "No research families detected.".to_string();
        }

        let mut lines: Vec<String> = Vec::new();
        lines.push("=".repeat(60));
        lines.push("Research Families".to_string());
        lines.push("=".repeat(60));
        lines.push(String::new());

        for (i, fam) in families.iter().enumerate().take(10) {
            lines.push(format!("[{}] Family: {}", i + 1, fam.common_theme));
            lines.push(format!("  Size: {} papers", fam.size));
            for p in fam.papers.iter().take(5) {
                let paper_id = p.get("paper_id").and_then(|v| v.as_str()).unwrap_or("");
                let title = p.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let year = p.get("year").and_then(|v| v.as_i64()).unwrap_or(0);
                lines.push(format!(
                    "  - [{}] {} ({})",
                    &paper_id[..paper_id.len().min(8)],
                    &title[..title.len().min(50)],
                    year
                ));
            }
            lines.push(String::new());
        }

        lines.join("\n")
    }

    pub fn render_silent_citations(&self, silent: &[HashMap<String, serde_json::Value>]) -> String {
        if silent.is_empty() {
            return "No silent citations detected.".to_string();
        }

        let mut lines: Vec<String> = Vec::new();
        lines.push("=".repeat(60));
        lines.push("Silent Citations (suspected)".to_string());
        lines.push("=".repeat(60));
        lines.push(String::new());

        for s in silent {
            let confidence = s.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let newer_id = s
                .get("newer_arxiv_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let newer_title = s.get("newer_title").and_then(|v| v.as_str()).unwrap_or("");
            let newer_year = s.get("newer_year").and_then(|v| v.as_i64()).unwrap_or(0);
            let older_title = s.get("older_title").and_then(|v| v.as_str()).unwrap_or("");
            let older_year = s.get("older_year").and_then(|v| v.as_i64()).unwrap_or(0);
            let shared = s
                .get("shared_methods")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .take(5)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();

            lines.push(format!(
                "[{:.*}% confidence] {}",
                0,
                (confidence * 100.0) as i32,
                &newer_id[..newer_id.len().min(8)]
            ));
            lines.push(format!(
                "  NEWER: {} ({})",
                &newer_title[..newer_title.len().min(60)],
                newer_year
            ));
            lines.push(format!(
                "  OLDER: {} ({})",
                &older_title[..older_title.len().min(60)],
                older_year
            ));
            lines.push(format!("  SHARED: {}", shared));
            lines.push(String::new());
        }

        lines.join("\n")
    }
}

impl Default for CitationChainBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_citation_node_creation() {
        let node = CitationNode::new("paper1".to_string(), "Test Paper".to_string());
        assert_eq!(node.paper_id, "paper1");
        assert_eq!(node.title, "Test Paper");
        assert_eq!(node.year, 0);
        assert!(node.citations.is_empty());
    }

    #[test]
    fn test_builder_add_paper() {
        let mut builder = CitationChainBuilder::new();
        builder.add_paper(
            "paper1".to_string(),
            "Paper One".to_string(),
            2023,
            vec!["Author A".to_string()],
            vec!["paper2".to_string()],
            "Abstract".to_string(),
            10,
        );
        assert!(builder.nodes.contains_key("paper1"));
        assert_eq!(builder.nodes.get("paper1").unwrap().year, 2023);
    }

    #[test]
    fn test_link_citations() {
        let mut builder = CitationChainBuilder::new();
        builder.add_paper(
            "p1".to_string(),
            "Paper 1".to_string(),
            2020,
            vec![],
            vec![],
            String::new(),
            0,
        );
        builder.add_paper(
            "p2".to_string(),
            "Paper 2".to_string(),
            2021,
            vec![],
            vec![],
            String::new(),
            0,
        );
        builder.link_citations("p1", "p2");

        let p1 = builder.nodes.get("p1").unwrap();
        let p2 = builder.nodes.get("p2").unwrap();
        assert!(p1.citations.contains(&"p2".to_string()));
        assert!(p2.cited_by.contains(&"p1".to_string()));
    }

    #[test]
    fn test_find_path_no_path() {
        let builder = CitationChainBuilder::new();
        assert!(builder.find_path("nonexistent", "p1").is_none());
    }

    #[test]
    fn test_find_influencers_empty() {
        let builder = CitationChainBuilder::new();
        assert!(builder.find_influencers("p1", 2).is_empty());
    }

    #[test]
    fn test_find_impact_empty() {
        let builder = CitationChainBuilder::new();
        assert!(builder.find_impact("p1", 2).is_empty());
    }

    #[test]
    fn test_cluster_families_empty() {
        let builder = CitationChainBuilder::new();
        assert!(builder.cluster_families().is_empty());
    }

    #[test]
    fn test_detect_silent_citations_empty() {
        let builder = CitationChainBuilder::new();
        assert!(builder.detect_silent_citations().is_empty());
    }

    #[test]
    fn test_render_text_empty() {
        let builder = CitationChainBuilder::new();
        let chain = CitationChain {
            nodes: vec![],
            edges: vec![],
        };
        let result = builder.render_text(&chain, 20);
        assert_eq!(result, "No citation chain.");
    }

    #[test]
    fn test_render_graphviz_empty() {
        let builder = CitationChainBuilder::new();
        let chain = CitationChain {
            nodes: vec![],
            edges: vec![],
        };
        let result = builder.render_graphviz(&chain);
        assert!(result.contains("digraph citations"));
    }

    #[test]
    fn test_render_mermaid_empty() {
        let builder = CitationChainBuilder::new();
        let chain = CitationChain {
            nodes: vec![],
            edges: vec![],
        };
        let result = builder.render_mermaid(&chain);
        assert!(result.contains("```mermaid"));
    }

    #[test]
    fn test_render_families_empty() {
        let builder = CitationChainBuilder::new();
        let families: Vec<ResearchFamily> = vec![];
        let result = builder.render_families(&families);
        assert_eq!(result, "No research families detected.");
    }

    #[test]
    fn test_render_silent_citations_empty() {
        let builder = CitationChainBuilder::new();
        let silent: Vec<HashMap<String, serde_json::Value>> = vec![];
        let result = builder.render_silent_citations(&silent);
        assert_eq!(result, "No silent citations detected.");
    }
}
