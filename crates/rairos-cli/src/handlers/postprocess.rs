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

pub fn handle_postprocess(
    db: &rairos_core::Database,
    paper_id: &str,
    root: &str,
    stages: &[String],
    skip_llm: bool,
    tags: Option<&str>,
) -> Result<()> {
    let root_path = std::path::PathBuf::from(root);
    let tags_vec: Vec<String> = tags
        .map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();

    // Parse stage filter
    let stage_filter: Option<Vec<rairos_postprocess::PostStage>> = if stages.is_empty() {
        None
    } else {
        let parsed: Vec<rairos_postprocess::PostStage> = stages
            .iter()
            .filter_map(|s| rairos_postprocess::PostStage::from_string(s))
            .collect();
        if parsed.is_empty() { None } else { Some(parsed) }
    };

    // LLM config
    let llm_config = if skip_llm {
        None
    } else {
        rairos_postprocess::LlmConfig::from_env()
    };
    if llm_config.is_some() {
        println!("  LLM mode: enabled");
    } else {
        println!("  LLM mode: disabled (keyword-only fallback)");
    }

    // Try to get paper data from DB
    let paper = db.get_paper(paper_id).ok();

    // Guess P-note path
    let pnote_path = find_pnote_path(&root_path, paper.as_ref());

    // Run pipeline
    let mut pipeline = rairos_postprocess::ResearchDeepDivePipeline::new(
        Some(db.clone()),
        root_path,
    );
    let result = pipeline.run(
        paper_id,
        "", // extracted_text from DB
        paper.as_ref(),
        &tags_vec,
        pnote_path.as_deref(),
        stage_filter.as_deref(),
        llm_config.as_ref(),
    );

    // Report
    println!();
    println!("Pipeline complete — {}", result.summary());
    if !result.stages_completed.is_empty() {
        println!("  + {}", result.stages_completed.join(", "));
    }
    for failed in &result.stages_failed {
        if let Some(sr) = result.stage_results.get(failed) {
            if !sr.error.is_empty() {
                let truncated = if sr.error.len() > 80 {
                    &sr.error[..80]
                } else {
                    &sr.error
                };
                println!("  x {failed}: {truncated}");
            }
        }
    }
    if result.pnote_updated {
        if let Some(ref path) = pnote_path {
            if let Some(name) = path.file_name() {
                println!("  -> P-note: {}", name.to_string_lossy());
            }
        }
    }

    Ok(())
}

fn find_pnote_path(root: &std::path::Path, paper: Option<&rairos_core::Paper>) -> Option<std::path::PathBuf> {
    let paper = paper?;
    let category_dir = "02-Models";
    let title_slug = slugify(&paper.title);
    if title_slug.is_empty() {
        return None;
    }
    let year = paper.published.format("%Y").to_string();
    let guessed = root
        .join(category_dir)
        .join(format!("P - {year} - {title_slug}.md"));
    if guessed.exists() {
        Some(guessed)
    } else {
        None
    }
}

fn slugify(title: &str) -> String {
    let mut slug = String::new();
    for c in title.chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            slug.push(c);
        } else if (c.is_whitespace() || c == ':' || c == '/' || c == '\\')
            && !slug.ends_with('-') {
                slug.push('-');
            }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.len() > 80 {
        slug[..80].to_string()
    } else {
        slug
    }
}
