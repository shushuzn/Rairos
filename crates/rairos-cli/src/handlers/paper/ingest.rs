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


pub fn handle_ingest(paper_id: Option<&str>, json: bool, no_pdf: bool, source: &str) -> Result<()> {
    let Some(pid) = paper_id else {
        eprintln!("Usage: ingest <paper_id>");
        return Ok(());
    };

    println!("📥 Ingesting: {} (source: {}, no_pdf: {})", pid, source, no_pdf);

    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(async {
        rairos_parser::fetch_paper(pid).await
    });

    match result {
        Ok(paper) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&paper)?);
            } else {
                println!("Title: {}", paper.title);
                println!("ID: {}", paper.id);
                println!("Authors: {}", paper.authors.len());
                println!("Published: {}", paper.published);
                println!("Categories: {:?}", paper.categories);
                println!("Abstract: {}...", &paper.abstract_text[..200.min(paper.abstract_text.len())]);
            }
        }
        Err(e) => eprintln!("Failed to fetch {}: {}", pid, e),
    }

    Ok(())
}
