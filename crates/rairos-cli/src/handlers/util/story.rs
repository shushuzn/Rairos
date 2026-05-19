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
use chrono::Datelike;

use rairos_core::Database;

pub fn handle_story(db: &Database, topic: Option<&str>) -> Result<()> {
    let Some(topic) = topic else {
        eprintln!("❌ 请提供 topic");
        std::process::exit(1);
    };
    println!("📖 Weaving story for: {}", topic);

    let papers = db.search_papers_smart(topic, 20)?;
    let inputs: Vec<crate::story::PaperInput> = papers
        .iter()
        .map(|p| crate::story::PaperInput {
            id: p.id.clone(),
            title: p.title.clone(),
            abstract_text: p.abstract_text.clone(),
            year: p.published.year(),
        })
        .collect();

    let weaver = crate::story::StoryWeaver;
    let result = weaver.weave(topic, inputs);
    println!("{}", result.summary);
    Ok(())
}