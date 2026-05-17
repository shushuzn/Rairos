//! Handlers for claim graph commands.

use anyhow::Result;

pub fn handle_claimgraph_stats() -> Result<()> {
    use rairos_claimgraph::ClaimGraph;

    let graph = ClaimGraph::load(None);
    println!("🔗 Claim Graph Statistics");
    println!("   Nodes: {}", graph.node_count());
    println!("   Edges: {}", graph.edge_count());
    Ok(())
}

pub fn handle_claimgraph_contradictions() -> Result<()> {
    use rairos_claimgraph::ClaimGraph;

    let graph = ClaimGraph::load(None);
    let contradictions = graph.find_contradictions();

    println!("🔗 Claim Contradictions");
    if contradictions.is_empty() {
        println!("   No contradictions found");
    } else {
        for c in contradictions.iter().take(10) {
            println!("   - {} vs {} ({})", c.claim_a.claim_id, c.claim_b.claim_id, c.severity);
        }
    }
    Ok(())
}
