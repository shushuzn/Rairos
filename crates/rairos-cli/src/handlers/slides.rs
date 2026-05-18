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
use crate::handlers::*;

pub fn handle_slides(
    db: &rairos_core::Database,
    paper_ids: &[String],
    format: &str,
    template: &str,
    num_slides: usize,
    output: Option<&str>,
    include_notes: bool,
    lang: &str,
) -> Result<()> {
    use rairos_slides::{PaperSlidesGenerator, SlidesConfig, SlideFormat, SlideTemplate, SlideLanguage};

    let config = SlidesConfig {
        template: SlideTemplate::from_string(template),
        num_slides,
        format: SlideFormat::from_string(format),
        output_path: output.map(std::path::PathBuf::from),
        include_notes,
        language: SlideLanguage::from_string(lang),
    };

    println!("📊 Generating slides for {} paper(s)", paper_ids.len());
    println!("   Format: {} | Template: {} | Slides: {}", format, template, num_slides);

    let gen = PaperSlidesGenerator::new(Some(db));
    let result = gen.generate(paper_ids, &config);

    println!();
    println!("✅ Generated {} slides", result.slide_count);
    println!("   Output: {}", result.output_path);

    Ok(())
}
