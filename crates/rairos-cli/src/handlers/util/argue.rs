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

pub fn handle_argue(db: &Database, thesis: &[String]) -> Result<()> {
    let topic_text = if thesis.is_empty() {
        let papers = db.list_papers(None, 1, 0)?;
        if let Some(p) = papers.first() {
            p.title.clone()
        } else {
            "research".to_string()
        }
    } else {
        thesis.join(" ")
    };

    println!("🧠 Building argument for: {}", topic_text);
    println!("{}", rairos_argument_builder::render_argument(
        &rairos_argument_builder::ArgumentResult {
            topic: topic_text.clone(),
            argument: rairos_argument_builder::Argument {
                thesis: topic_text.clone(),
                claims: vec![],
                supporting_evidence: vec![],
                contradicting_evidence: vec![],
                related_gaps: vec![],
                paper_suggestions: vec![],
            },
            summary: String::new(),
            section_guidance: std::collections::HashMap::new(),
        }
    ));
    Ok(())
}