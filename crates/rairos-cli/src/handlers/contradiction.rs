//! Handlers for contradiction detection commands.

use anyhow::Result;

pub fn handle_contradictions_list(limit: usize) -> Result<()> {
    let contrad_map = rairos_contradiction::compute_paper_contradictions();
    
    let mut papers: Vec<_> = contrad_map.into_iter().collect();
    papers.sort_by(|a, b| b.1.count.cmp(&a.1.count));
    
    println!("=== Contradiction Detection ===");
    println!("Top papers with contradictions:\n");
    
    for (i, (paper_id, info)) in papers.iter().take(limit).enumerate() {
        println!("{}. Paper: {}", i + 1, paper_id);
        println!("   Contradictions: {}", info.count);
        if !info.contradictions.is_empty() {
            println!("   Types:");
            for contrad in &info.contradictions[..info.contradictions.len().min(3)] {
                println!("     - [{}] with {} (keywords: {:?})", 
                    contrad.gap_type, contrad.partner_id, contrad.shared_keywords);
            }
        }
        println!();
    }
    
    if papers.is_empty() {
        println!("No contradictions found in gene pool.");
        println!("Add papers and generate gaps to detect contradictions.");
    }
    
    Ok(())
}

pub fn handle_contradictions_render() -> Result<()> {
    let contrad_map = rairos_contradiction::compute_paper_contradictions();
    println!("Contradiction map has {} papers with contradictions", contrad_map.len());
    println!("Run 'rairos contradictions list' to see details.");
    Ok(())
}
