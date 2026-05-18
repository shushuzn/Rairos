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

pub fn handle_discover(force: bool) -> Result<()> {
    let result = crate::discover::discover(force);
    println!("{}", serde_json::to_string_pretty(&result)?);
    if result.patterns_discovered > 0 {
        println!("{} new patterns discovered", result.patterns_discovered);
    }
    Ok(())
}

pub fn handle_scout(topic: Option<&str>, sources: &str, max_results: usize) -> Result<()> {
    let topic_str = topic.unwrap_or("machine learning");
    println!("🔍 Scouting topic: {} (sources: {})", topic_str, sources);
    let results = crate::scout::scout(topic_str, sources, 5, max_results, 0.3, &[]);
    println!("{}", crate::scout::render_scout_results(&results));
    Ok(())
}
