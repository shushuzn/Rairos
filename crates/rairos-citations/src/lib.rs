//! Rairos Citations — Citation Chain Builder
//!
//! Core citation chain structures and algorithms: building chains, finding paths,
//! research family clustering, and silent citation detection.

#![allow(
    clippy::vec_init_then_push,
    clippy::unnecessary_sort_by,
    clippy::needless_range_loop
)]

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationNode {
    pub paper_id: String,
    pub title: String,
    pub year: i32,
    pub authors: Vec<String>,
    #[serde(default)]
    pub abstract_text: String,
    #[serde(default)]
    pub citations: Vec<String>,
    #[serde(default)]
    pub cited_by: Vec<String>,
    #[serde(default)]
    pub citation_count: i32,
}

#[derive(Debug, Clone, Default)]
pub struct CitationChain {
    pub nodes: HashMap<String, CitationNode>,
    pub edges: Vec<(String, String)>,
}

impl CitationChain {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchFamily {
    pub family_id: String,
    pub ancestor_id: String,
    pub ancestor_title: String,
    #[serde(default)]
    pub papers: Vec<PaperInfo>,
    pub common_theme: String,
    pub size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperInfo {
    pub paper_id: String,
    pub title: String,
    pub year: i32,
}

pub struct CitationChainBuilder {
    nodes: HashMap<String, CitationNode>,
}

impl Default for CitationChainBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CitationChainBuilder {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_paper(
        &mut self,
        paper_id: &str,
        title: &str,
        year: i32,
        authors: Vec<String>,
        references: Vec<String>,
        abstract_text: String,
        citation_count: i32,
    ) -> &mut CitationNode {
        self.nodes
            .entry(paper_id.to_string())
            .or_insert_with(|| CitationNode {
                paper_id: paper_id.to_string(),
                title: title.to_string(),
                year,
                authors,
                abstract_text,
                citations: references.clone(),
                cited_by: Vec::new(),
                citation_count,
            });
        let node = self.nodes.get_mut(paper_id).unwrap();
        node.citations = references;
        node
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

    pub fn build_chain(self) -> CitationChain {
        let mut edges = Vec::new();
        for (pid, node) in &self.nodes {
            for cited in &node.citations {
                if self.nodes.contains_key(cited) {
                    edges.push((pid.clone(), cited.clone()));
                }
            }
        }
        CitationChain {
            nodes: self.nodes,
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

        let mut visited: HashSet<&str> = HashSet::new();
        let mut queue: VecDeque<(String, Vec<String>)> = VecDeque::new();
        queue.push_back((from_id.to_string(), vec![from_id.to_string()]));
        visited.insert(from_id);

        while let Some((current, path)) = queue.pop_front() {
            let node = match self.nodes.get(&current) {
                Some(n) => n,
                None => continue,
            };

            for neighbor in &node.citations {
                if neighbor == to_id {
                    let mut result = path.clone();
                    result.push(neighbor.clone());
                    return Some(result);
                }
                if !visited.contains(neighbor.as_str()) {
                    visited.insert(neighbor.as_str());
                    let mut new_path = path.clone();
                    new_path.push(neighbor.clone());
                    queue.push_back((neighbor.clone(), new_path));
                }
            }

            for neighbor in &node.cited_by {
                if neighbor == to_id {
                    let mut result = path.clone();
                    result.push(neighbor.clone());
                    return Some(result);
                }
                if !visited.contains(neighbor.as_str()) {
                    visited.insert(neighbor.as_str());
                    let mut new_path = path.clone();
                    new_path.push(neighbor.clone());
                    queue.push_back((neighbor.clone(), new_path));
                }
            }
        }
        None
    }

    pub fn find_influencers(&self, paper_id: &str, depth: i32) -> Vec<&CitationNode> {
        let mut result = Vec::new();
        let mut visited: HashSet<&str> = HashSet::new();
        let mut queue: VecDeque<(&str, i32)> = VecDeque::new();
        queue.push_back((paper_id, 0));
        visited.insert(paper_id);

        while let Some((pid, d)) = queue.pop_front() {
            if d > depth {
                continue;
            }
            let node = match self.nodes.get(pid) {
                Some(n) => n,
                None => continue,
            };
            for ancestor_id in &node.cited_by {
                if !visited.contains(ancestor_id.as_str()) {
                    visited.insert(ancestor_id.as_str());
                    if let Some(ancestor) = self.nodes.get(ancestor_id) {
                        result.push(ancestor);
                    }
                    queue.push_back((ancestor_id.as_str(), d + 1));
                }
            }
        }
        result
    }

    pub fn find_impact(&self, paper_id: &str, depth: i32) -> Vec<&CitationNode> {
        let mut result = Vec::new();
        let mut visited: HashSet<&str> = HashSet::new();
        let mut queue: VecDeque<(&str, i32)> = VecDeque::new();
        queue.push_back((paper_id, 0));
        visited.insert(paper_id);

        while let Some((pid, d)) = queue.pop_front() {
            if d > depth {
                continue;
            }
            let node = match self.nodes.get(pid) {
                Some(n) => n,
                None => continue,
            };
            for descendant_id in &node.citations {
                if !visited.contains(descendant_id.as_str()) {
                    visited.insert(descendant_id.as_str());
                    if let Some(descendant) = self.nodes.get(descendant_id) {
                        result.push(descendant);
                    }
                    queue.push_back((descendant_id.as_str(), d + 1));
                }
            }
        }
        result
    }

    pub fn cluster_families(&self) -> Vec<ResearchFamily> {
        let mut families = Vec::new();
        let node_values: Vec<&CitationNode> = self.nodes.values().collect();

        let mut ref_to_papers: HashMap<&str, HashSet<&str>> = HashMap::new();
        for node in &node_values {
            for r in &node.citations {
                ref_to_papers
                    .entry(r)
                    .or_default()
                    .insert(node.paper_id.as_str());
            }
        }

        let mut seen_pairs: HashSet<(String, String)> = HashSet::new();

        for node in &node_values {
            if node.citations.is_empty() {
                continue;
            }

            let mut family_members: HashMap<&str, HashSet<&str>> = HashMap::new();
            for reference in &node.citations {
                for other_pid in ref_to_papers
                    .get(reference.as_str())
                    .unwrap_or(&HashSet::new())
                {
                    if *other_pid != node.paper_id.as_str() {
                        family_members
                            .entry(other_pid)
                            .or_default()
                            .insert(reference.as_str());
                    }
                }
            }

            for (other_pid, shared_refs) in family_members {
                if shared_refs.len() >= 2 {
                    let other_node = self.nodes.get(other_pid);
                    let pair_key = (node.paper_id.clone(), other_pid.to_string());
                    let sorted_key = (
                        pair_key.0.clone().min(pair_key.1.clone()),
                        pair_key.0.clone().max(pair_key.1.clone()),
                    );
                    if seen_pairs.contains(&sorted_key) {
                        continue;
                    }
                    seen_pairs.insert(sorted_key);

                    let shared_refs_vec: Vec<&str> = shared_refs.iter().take(3).copied().collect();
                    let family_id = format!("{:x}", md5_simple(&node.paper_id));

                    families.push(ResearchFamily {
                        family_id,
                        ancestor_id: node.paper_id.clone(),
                        ancestor_title: format!("Family sharing: {}", shared_refs_vec.join(", ")),
                        papers: vec![
                            PaperInfo {
                                paper_id: node.paper_id.clone(),
                                title: node.title.clone(),
                                year: node.year,
                            },
                            PaperInfo {
                                paper_id: other_pid.to_string(),
                                title: other_node
                                    .map(|n| n.title.clone())
                                    .unwrap_or_else(|| other_pid.to_string()),
                                year: other_node.map(|n| n.year).unwrap_or(0),
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

    pub fn detect_silent_citations(&self) -> Vec<SilentCitation> {
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
            "neural network",
            "convolutional",
            "recurrent",
            "generative",
            "diffusion",
            "gan",
            "vae",
            "autoencoder",
        ]
        .into_iter()
        .collect();

        let re_word = Regex::new(r"\b[a-z][a-z0-9-]{3,}\b").unwrap();

        fn extract_terms(text: &str, terms: &HashSet<&str>, re: &Regex) -> HashSet<String> {
            re.find_iter(text.to_lowercase().as_str())
                .map(|m| m.as_str().to_string())
                .filter(|w| terms.contains(w.as_str()))
                .collect()
        }

        let nodes: Vec<&CitationNode> = self.nodes.values().collect();
        let mut node_terms: HashMap<&str, (HashSet<String>, HashSet<&str>)> = HashMap::new();

        for node in &nodes {
            let terms = if !node.abstract_text.is_empty() {
                extract_terms(&node.abstract_text, &method_terms, &re_word)
            } else {
                HashSet::new()
            };
            let citations: HashSet<&str> = node.citations.iter().map(|s| s.as_str()).collect();
            node_terms.insert(node.paper_id.as_str(), (terms, citations));
        }

        let mut silent = Vec::new();
        for i in 0..nodes.len() {
            let node = nodes[i];
            let (terms_set, citations_set) = match node_terms.get(node.paper_id.as_str()) {
                Some(t) => t.clone(),
                None => (HashSet::new(), HashSet::new()),
            };
            if terms_set.is_empty() {
                continue;
            }

            for j in (i + 1)..nodes.len() {
                let other = nodes[j];
                let (other_terms, other_citations) = match node_terms.get(other.paper_id.as_str()) {
                    Some(t) => t.clone(),
                    None => (HashSet::new(), HashSet::new()),
                };
                if other_terms.is_empty() {
                    continue;
                }

                if other.paper_id.as_str() == node.paper_id.as_str() {
                    continue;
                }

                if citations_set.contains(other.paper_id.as_str())
                    || other_citations.contains(node.paper_id.as_str())
                {
                    continue;
                }

                let shared: HashSet<&str> = terms_set
                    .iter()
                    .map(|s| s.as_str())
                    .filter(|t| other_terms.contains(*t))
                    .collect();

                if shared.len() >= 4 {
                    let (newer, older) = if other.year > node.year {
                        (other, node)
                    } else {
                        (node, other)
                    };

                    silent.push(SilentCitation {
                        newer_arxiv_id: newer.paper_id.clone(),
                        newer_title: newer.title.clone(),
                        newer_year: newer.year,
                        older_arxiv_id: older.paper_id.clone(),
                        older_title: older.title.clone(),
                        older_year: older.year,
                        shared_methods: shared.iter().take(5).map(|s| s.to_string()).collect(),
                        confidence: (shared.len() as f64 / 10.0).min(0.95),
                    });
                }
            }
        }

        silent.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        silent.truncate(10);
        silent
    }

    pub fn render_text(&self, chain: &CitationChain, max_nodes: usize) -> String {
        if chain.nodes.is_empty() {
            return "No citation chain.".to_string();
        }

        let mut lines = Vec::new();
        lines.push("=".repeat(60));
        lines.push("📚 Citation Chain".to_string());
        lines.push("=".repeat(60));
        lines.push(String::new());

        let mut sorted_nodes: Vec<&CitationNode> = chain.nodes.values().collect();
        sorted_nodes.sort_by_key(|b| std::cmp::Reverse(b.year));

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
            lines.push(String::new());
        }

        lines.push(format!(
            "Total: {} papers, {} connections",
            chain.nodes.len(),
            chain.edges.len()
        ));
        lines.push("=".repeat(60));
        lines.join("\n")
    }

    pub fn render_graphviz(&self, chain: &CitationChain) -> String {
        let mut lines = Vec::new();
        lines.push("digraph citations {".to_string());
        lines.push("  rankdir=LR;".to_string());
        lines.push("  node [shape=box];".to_string());

        for node in chain.nodes.values() {
            let label = if node.year > 0 {
                format!(
                    "{}\\n({})",
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
        let mut lines = Vec::new();
        lines.push("```mermaid".to_string());
        lines.push("flowchart LR".to_string());

        for node in chain.nodes.values() {
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

        let mut lines = Vec::new();
        lines.push("=".repeat(60));
        lines.push("🔬 Research Families".to_string());
        lines.push("=".repeat(60));
        lines.push(String::new());

        for (i, fam) in families.iter().enumerate() {
            lines.push(format!("[{}] Family: {}", i + 1, fam.common_theme));
            lines.push(format!("  Size: {} papers", fam.size));
            for p in fam.papers.iter().take(5) {
                lines.push(format!(
                    "  - [{}] {} ({})",
                    &p.paper_id[..p.paper_id.len().min(8)],
                    &p.title[..p.title.len().min(50)],
                    if p.year > 0 {
                        p.year.to_string()
                    } else {
                        "?".to_string()
                    }
                ));
            }
            lines.push(String::new());
        }

        lines.join("\n")
    }

    pub fn render_silent_citations(&self, silent: &[SilentCitation]) -> String {
        if silent.is_empty() {
            return "No silent citations detected.".to_string();
        }

        let mut lines = Vec::new();
        lines.push("=".repeat(60));
        lines.push("⚠️ Silent Citations (suspected)".to_string());
        lines.push("=".repeat(60));
        lines.push(String::new());

        for s in silent {
            lines.push(format!(
                "[{:.0}% confidence] {}",
                s.confidence * 100.0,
                &s.newer_arxiv_id[..s.newer_arxiv_id.len().min(8)]
            ));
            lines.push(format!(
                "  NEWER: {} ({})",
                &s.newer_title[..s.newer_title.len().min(60)],
                s.newer_year
            ));
            lines.push(format!(
                "  OLDER: {} ({})",
                &s.older_title[..s.older_title.len().min(60)],
                s.older_year
            ));
            lines.push(format!("  SHARED: {}", s.shared_methods.join(", ")));
            lines.push(String::new());
        }

        lines.join("\n")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilentCitation {
    pub newer_arxiv_id: String,
    pub newer_title: String,
    pub newer_year: i32,
    pub older_arxiv_id: String,
    pub older_title: String,
    pub older_year: i32,
    pub shared_methods: Vec<String>,
    pub confidence: f64,
}

fn md5_simple(input: &str) -> u64 {
    let mut hash: u64 = 0;
    for (i, byte) in input.bytes().enumerate() {
        hash = hash.wrapping_add((byte as u64).wrapping_mul((i as u64).wrapping_add(1)));
        hash = hash.rotate_left(5);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_paper() {
        let mut builder = CitationChainBuilder::new();
        builder.add_paper(
            "p1",
            "Paper One",
            2023,
            vec!["Author A".to_string()],
            vec![],
            "Abstract".to_string(),
            5,
        );
        let chain = builder.build_chain();
        assert!(chain.nodes.contains_key("p1"));
    }

    #[test]
    fn test_link_citations() {
        let mut builder = CitationChainBuilder::new();
        builder.add_paper("p1", "Paper One", 2023, vec![], vec![], String::new(), 0);
        builder.add_paper("p2", "Paper Two", 2022, vec![], vec![], String::new(), 0);
        builder.link_citations("p1", "p2");
        let chain = builder.build_chain();
        assert!(chain.edges.contains(&("p1".to_string(), "p2".to_string())));
    }

    #[test]
    fn test_find_path() {
        let mut builder = CitationChainBuilder::new();
        builder.add_paper(
            "p1",
            "Paper One",
            2023,
            vec![],
            vec!["p2".to_string()],
            String::new(),
            0,
        );
        builder.add_paper(
            "p2",
            "Paper Two",
            2022,
            vec![],
            vec!["p3".to_string()],
            String::new(),
            0,
        );
        builder.add_paper("p3", "Paper Three", 2021, vec![], vec![], String::new(), 0);
        let path = builder.find_path("p1", "p3");
        assert!(path.is_some());
        let p = path.unwrap();
        assert_eq!(p[0], "p1");
        assert_eq!(p[p.len() - 1], "p3");
    }

    #[test]
    fn test_find_path_no_path() {
        let mut builder = CitationChainBuilder::new();
        builder.add_paper("p1", "Paper One", 2023, vec![], vec![], String::new(), 0);
        builder.add_paper("p2", "Paper Two", 2022, vec![], vec![], String::new(), 0);
        let path = builder.find_path("p1", "p2");
        assert!(path.is_none());
    }

    #[test]
    fn test_cluster_families_empty() {
        let builder = CitationChainBuilder::new();
        let families = builder.cluster_families();
        assert!(families.is_empty());
    }

    #[test]
    fn test_detect_silent_citations_empty() {
        let builder = CitationChainBuilder::new();
        let silent = builder.detect_silent_citations();
        assert!(silent.is_empty());
    }

    #[test]
    fn test_render_text_empty() {
        let builder = CitationChainBuilder::new();
        let chain = CitationChain::new();
        let text = builder.render_text(&chain, 20);
        assert_eq!(text, "No citation chain.");
    }
}
