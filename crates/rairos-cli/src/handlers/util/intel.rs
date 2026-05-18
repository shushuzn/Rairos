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

pub fn handle_intel(topic: &str, verbose: bool) -> Result<()> {
    let report = rairos_intelligence::IntelligenceGenerator::generate(topic, verbose);
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}