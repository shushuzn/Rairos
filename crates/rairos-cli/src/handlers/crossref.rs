//! Handlers for cross-reference analysis commands.

use anyhow::Result;

pub fn handle_crossref_analyze(paper_id: &str) -> Result<()> {
    use rairos_cross_referencer::CrossReferencer;

    let referencer = CrossReferencer::new();
    let result = referencer.analyze(
        paper_id,
        "Sample Title",
        "sample abstract",
        "",
        None,
        false,
    );

    println!("🔗 Cross-Reference Analysis: {}", paper_id);
    if result.error.is_empty() {
        println!("   Related papers found: {}", result.related_papers_found);
    } else {
        println!("   Status: {}", result.error);
    }
    Ok(())
}

pub fn handle_crossref_list() -> Result<()> {
    println!("🔗 Cross-References");
    println!("   Use 'rairos crossref <paper_id>' to analyze");
    Ok(())
}
