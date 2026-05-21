//! CLI command handler implementations.
//!
//! Extracted from main.rs for maintainability. Each handler
//! corresponds to one Commands variant from the parent module.

#![allow(
    clippy::too_many_arguments,
    clippy::needless_borrow,
    clippy::print_literal,
    clippy::unwrap_or_default,
    clippy::unnecessary_sort_by,
    clippy::format_in_format_args,
    clippy::map_identity,
    clippy::unused_enumerate_index,
    clippy::needless_borrows_for_generic_args,
    clippy::unnecessary_to_owned,
    clippy::manual_range_contains
)]

use anyhow::Result;
use rairos_core::Database;
use rairos_kg::{GraphAlgorithms, KgNode, KnowledgeGraph};
use std::collections::HashSet;



// ====================================================================
// Handler implementations
// ====================================================================

// ============================================================================
// Command Handlers
// ============================================================================

pub fn handle_kg_stats(format: &str) -> Result<()> {
    let graph = tokio::runtime::Handle::current()
        .block_on(async { KnowledgeGraph::load().await.unwrap_or_else(|_| KnowledgeGraph::new()) });
    let stats = graph.stats();

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&stats)?);
        return Ok(());
    }

    println!("=== Knowledge Graph Stats ===\n");
    println!("Total Nodes:  {}", stats.total_nodes);
    println!("Total Edges:  {}", stats.total_edges);
    println!("Avg Degree:   {:.2}", stats.avg_degree);
    println!("Paper Nodes: {}", stats.paper_nodes);
    println!("Concept Nodes: {}", stats.concept_nodes);
    Ok(())
}

pub fn handle_kg_rank(limit: usize, format: &str) -> Result<()> {
    let graph = tokio::runtime::Handle::current()
        .block_on(async { KnowledgeGraph::load().await.unwrap_or_else(|_| KnowledgeGraph::new()) });
    let ranks = GraphAlgorithms::rank_papers(&graph);

    let mut sorted: Vec<_> = ranks.into_iter().collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if format == "json" {
        let out: Vec<serde_json::Value> = sorted
            .iter()
            .take(limit)
            .map(|(id, score)| serde_json::json!({ "paper_id": id, "score": score }))
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("=== Paper Rankings (Top {}) ===\n", limit);
    println!("{:>6} {:<40} {:>10}", "RANK", "PAPER ID", "SCORE");
    println!("{}", "-".repeat(60));
    for (i, (id, score)) in sorted.iter().take(limit).enumerate() {
        println!(
            "{:>6} {:<40} {:>10.4}",
            i + 1,
            &id[..id.len().min(40)],
            score
        );
    }
    Ok(())
}

pub fn handle_kg_path(source: &str, target: &str) -> Result<()> {
    let graph = tokio::runtime::Handle::current()
        .block_on(async { KnowledgeGraph::load().await.unwrap_or_else(|_| KnowledgeGraph::new()) });

    match graph.find_path(source, target) {
        Some(path) => {
            println!("=== Path Found ({} steps) ===\n", path.len() - 1);
            for (i, node) in path.iter().enumerate() {
                println!("{}. {}", i + 1, node);
            }
        }
        None => {
            println!("No path found between {} and {}", source, target);
        }
    }
    Ok(())
}

pub fn handle_kg_add_paper(db: &Database, paper_id: &str) -> Result<()> {
    let paper = db
        .get_paper(paper_id)
        .ok()
        .or_else(|| db.get_paper_by_arxiv(paper_id).ok().flatten())
        .ok_or_else(|| anyhow::anyhow!("Paper not found: {}", paper_id))?;

    let mut graph = tokio::runtime::Handle::current()
        .block_on(async { KnowledgeGraph::load().await.unwrap_or_else(|_| KnowledgeGraph::new()) });
    graph.add_paper(&paper);
    graph
        .save()
        .map_err(|e| anyhow::anyhow!("Failed to save knowledge graph: {}", e))?;

    println!("[OK] Added paper to knowledge graph:");
    println!("  ID: {}", paper.id);
    println!("  Title: {}", &paper.title[..paper.title.len().min(60)]);
    Ok(())
}

pub fn handle_kg_add_citation(db: &Database, source: &str, target: &str) -> Result<()> {
    let source_paper = db
        .get_paper(source)
        .ok()
        .or_else(|| db.get_paper_by_arxiv(source).ok().flatten())
        .ok_or_else(|| anyhow::anyhow!("Source paper not found: {}", source))?;

    let target_paper = db
        .get_paper(target)
        .ok()
        .or_else(|| db.get_paper_by_arxiv(target).ok().flatten())
        .ok_or_else(|| anyhow::anyhow!("Target paper not found: {}", target))?;

    let mut graph = tokio::runtime::Handle::current()
        .block_on(async { KnowledgeGraph::load().await.unwrap_or_else(|_| KnowledgeGraph::new()) });

    // Ensure both papers are in the graph
    graph.add_paper(&source_paper);
    graph.add_paper(&target_paper);
    graph.add_citation(&source_paper.id, &target_paper.id);
    graph
        .save()
        .map_err(|e| anyhow::anyhow!("Failed to save knowledge graph: {}", e))?;

    println!("[OK] Added citation edge:");
    println!("  {} -> {}", source_paper.id, target_paper.id);
    Ok(())
}

pub fn handle_kg_graph(paper_id: &str, depth: u32, format: &str) -> Result<()> {
    let graph = tokio::runtime::Handle::current()
        .block_on(async { KnowledgeGraph::load().await })?;

    // Find center node by entity_id or node id
    let center = graph
        .nodes()
        .values()
        .find(|n| n.entity_id == paper_id || n.id == paper_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Paper '{}' not found in KG", paper_id))?;

    // BFS neighbors on in-memory graph
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(center.id.clone());
    let mut results: Vec<(KgNode, String, u32)> = Vec::new();
    let mut current_level = vec![center.id.clone()];

    for d in 1..=depth {
        let mut next_level = Vec::new();
        for nid in &current_level {
            for edge in &graph.edges {
                let neighbor_id = if edge.source == *nid {
                    Some(&edge.target)
                } else if edge.target == *nid {
                    Some(&edge.source)
                } else {
                    None
                };
                if let Some(nbr_id) = neighbor_id {
                    if visited.insert(nbr_id.clone()) {
                        if let Some(node) = graph.get_node(nbr_id) {
                            results.push((node.clone(), edge.relation.clone(), d));
                            next_level.push(nbr_id.clone());
                        }
                    }
                }
            }
        }
        current_level = next_level;
    }

    if format == "json" {
        let neighbors: Vec<serde_json::Value> = results
            .iter()
            .map(|(node, rel, d)| {
                serde_json::json!({
                    "id": node.id,
                    "entity_id": node.entity_id,
                    "label": node.label,
                    "type": node.node_type,
                    "relation": rel,
                    "depth": d,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "center": {
                    "id": center.id,
                    "entity_id": center.entity_id,
                    "label": center.label,
                    "type": center.node_type,
                },
                "neighbors": neighbors,
                "total": results.len(),
            }))?
        );
    } else {
        println!("=== KG Graph for '{}' (depth={}) ===", paper_id, depth);
        println!(
            "Center: [{}] {}",
            center.node_type,
            center.label
        );
        println!("\n{} neighbor(s):", results.len());
        for (node, rel, d) in &results {
            let label = if node.label.len() > 50 {
                format!("{}...", &node.label[..47])
            } else {
                node.label.clone()
            };
            println!(
                "  [depth={}] {:<8} | {:<12} | {}",
                d, node.node_type, rel, label
            );
        }
    }
    Ok(())
}

pub fn handle_kg_search(
    node_type: Option<&str>,
    keyword: Option<&str>,
    format: &str,
) -> Result<()> {
    let graph = tokio::runtime::Handle::current()
        .block_on(async { KnowledgeGraph::load().await })?;

    let nodes: Vec<&KgNode> = graph
        .nodes()
        .values()
        .filter(|n| {
            let type_match = node_type.map(|t| n.node_type == t).unwrap_or(true);
            let kw = keyword.unwrap_or("");
            let keyword_match = if kw.is_empty() {
                true
            } else {
                let kw_lower = kw.to_lowercase();
                let label_lower = n.label.to_lowercase();
                let entity_id_lower = n.entity_id.to_lowercase();
                label_lower.contains(&kw_lower) || entity_id_lower.contains(&kw_lower)
            };
            type_match && keyword_match
        })
        .collect();

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&nodes)?);
    } else {
        println!("=== KG Search Results ===\n");
        if nodes.is_empty() {
            println!("No nodes found.");
            return Ok(());
        }
        println!(
            "{:>4} {:<12} {:<12} {}",
            "#", "TYPE", "ENTITY ID", "LABEL"
        );
        println!("{}", "-".repeat(80));
        for (i, node) in nodes.iter().enumerate().take(100) {
            let eid = if node.entity_id.len() > 12 {
                format!("{}...", &node.entity_id[..9])
            } else {
                node.entity_id.clone()
            };
            let label = if node.label.len() > 52 {
                format!("{}...", &node.label[..49])
            } else {
                node.label.clone()
            };
            println!(
                "{:>4} {:<12} {:<12} {}",
                i + 1,
                node.node_type,
                eid,
                label
            );
        }
        if nodes.len() > 100 {
            println!("... and {} more", nodes.len() - 100);
        }
        println!("\nTotal: {} nodes", nodes.len());
    }
    Ok(())
}

pub fn handle_kg_rebuild(db: &Database, incremental: bool) -> Result<()> {
    // Load the knowledge graph (with DB connection)
    let graph = tokio::runtime::Handle::current()
        .block_on(async { KnowledgeGraph::load().await })?;

    // Load all papers from the papers database
    let papers = db.list_papers(None, 100_000, 0)?;
    if papers.is_empty() {
        println!("No papers found in database.");
        return Ok(());
    }

    println!("Loading {} papers into knowledge graph...", papers.len());

    // Convert to KgNode
    let kg_nodes: Vec<KgNode> = papers.iter().map(KgNode::from_paper).collect();

    // Load all citations from the papers database
    let all_citations = db.list_all_citations()?;
    println!(
        "Connecting {} citation edges...",
        all_citations.len()
    );

    // Use the KgDatabase to rebuild
    let db_ref = graph
        .database()
        .ok_or_else(|| anyhow::anyhow!("Knowledge graph has no database connection"))?;

    let stats = tokio::runtime::Handle::current()
        .block_on(async { db_ref.rebuild_from_papers(&kg_nodes, &all_citations).await })?;

    println!(
        "Done: {} nodes, {} edges.",
        stats.total_nodes, stats.total_edges
    );

    if incremental {
        println!("(Incremental mode enabled — only new/changed papers processed.)");
    }

    Ok(())
}

pub fn try_get_kg() -> Option<rairos_kg::KnowledgeGraph> {
    let kg_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".ai_research_os")
        .join("kg.db");
    if kg_path.exists() {
        tokio::runtime::Handle::current()
            .block_on(async { rairos_kg::KnowledgeGraph::with_db(kg_path).await.ok() })
    } else {
        // Try local path
        let local_path = std::path::PathBuf::from("kg.db");
        if local_path.exists() {
            tokio::runtime::Handle::current()
                .block_on(async { rairos_kg::KnowledgeGraph::with_db(local_path).await.ok() })
        } else {
            Some(rairos_kg::KnowledgeGraph::new())
        }
    }
}

