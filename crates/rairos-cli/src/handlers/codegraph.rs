//! Handlers for codegraph commands.

use anyhow::Result;
use rairos_codegraph::CodeGraph;
use std::path::PathBuf;

fn get_codegraph_db_path() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find local data directory"))?;
    Ok(base.join("rairos").join("codegraph.db"))
}

fn with_codegraph<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&CodeGraph) -> Result<T>,
{
    let db_path = get_codegraph_db_path()?;
    let graph = CodeGraph::open(&db_path)?;
    f(&graph)
}

pub fn handle_codegraph_stats() -> Result<()> {
    let stats = with_codegraph(|g| Ok(g.stats()?))?;
    
    println!("📊 CodeGraph Statistics");
    println!("   Nodes: {}", stats.nodes);
    println!("   Edges: {}", stats.edges);
    println!("   Files: {}", stats.files);
    
    Ok(())
}

pub fn handle_codegraph_files() -> Result<()> {
    let files = with_codegraph(|g| Ok(g.files()?))?;
    
    println!("📁 Indexed Files ({} total)", files.len());
    for f in files.iter().take(50) {
        println!("   {}", f);
    }
    if files.len() > 50 {
        println!("   ... and {} more", files.len() - 50);
    }
    
    Ok(())
}

pub fn handle_codegraph_search(query: &str) -> Result<()> {
    let results = with_codegraph(|g| Ok(g.search(query, 20)?))?;
    
    println!("🔍 Search results for '{}' ({} total)", query, results.len());
    for r in results {
        println!("   [{}] {} ({}) - {}:{}", r.node.kind, r.node.name, r.node.id, r.node.file, r.node.line);
        if !r.snippet.is_empty() {
            println!("       {}", r.snippet);
        }
    }
    
    Ok(())
}

pub fn handle_codegraph_node(node_id: i64) -> Result<()> {
    let node = with_codegraph(|g| Ok(g.get_node(node_id)?))?;
    
    if let Some(node) = node {
        println!("📍 Node #{}", node.id);
        println!("   Name: {}", node.name);
        println!("   Kind: {}", node.kind);
        println!("   Location: {}:{}-{}:{}", node.file, node.line, node.end_line, node.end_col);
        if let Some(doc) = &node.docstring {
            println!("   Doc: {}", doc);
        }
    } else {
        println!("Node #{} not found", node_id);
    }
    
    Ok(())
}

pub fn handle_codegraph_callers(node_id: i64, depth: usize) -> Result<()> {
    let callers = with_codegraph(|g| Ok(g.get_callers(node_id, depth)?))?;
    
    println!("⬆️  Callers of node #{} (depth={}, {} found)", node_id, depth, callers.len());
    for c in callers {
        println!("   [d={}] {} ({}) - {}:{}", c.depth, c.node.name, c.node.kind, c.node.file, c.node.line);
    }
    
    Ok(())
}

pub fn handle_codegraph_callees(node_id: i64, depth: usize) -> Result<()> {
    let callees = with_codegraph(|g| Ok(g.get_callees(node_id, depth)?))?;
    
    println!("⬇️  Callees of node #{} (depth={}, {} found)", node_id, depth, callees.len());
    for c in callees {
        println!("   [d={}] {} ({}) - {}:{}", c.depth, c.node.name, c.node.kind, c.node.file, c.node.line);
    }
    
    Ok(())
}
