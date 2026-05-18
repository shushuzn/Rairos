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

pub fn handle_validate(
    db: &rairos_core::Database,
    question: Option<&str>,
    no_llm: bool,
    json: bool,
    depth: &str,
    model: Option<&str>,
    interactive: bool,
) -> Result<()> {
    // Phase 1 & 2: rule-based validation (no LLM yet)
    // Phase 3 will add LLM integration when model is provided and no_llm is false

    // Interactive mode
    if interactive || question.is_none() {
        return handle_validate_interactive(db, no_llm, json, depth, model);
    }

    let question = question.unwrap();
    println!("🔬 Validating: {}", question);

    let related = find_related_works(db, question, if depth == "full" { 10 } else { 5 });
    let result = crate::validator::validate_rules(question, related);

    // Record NARRATED event (same as Python)
    if let Ok(tracker) = rairos_narratives::ResearchThreadTracker::new() {
        // Non-critical: just record the event
        let _ = tracker.save();
    }

    if json {
        println!("{}", render_validation_json(&result));
    } else {
        println!();
        println!("{}", crate::validator::render_result(&result));
    }

    Ok(())
}

fn find_related_works(
    db: &rairos_core::Database,
    question: &str,
    limit: usize,
) -> Vec<crate::validator::RelatedWork> {
    let keywords = crate::validator::expand_question(question, &crate::validator::default_ai_keywords());

    let mut related: Vec<crate::validator::RelatedWork> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for kw in keywords.iter().take(3) {
        if let Ok(papers) = db.search_papers(kw, limit) {
            for paper in &papers {
                if seen.contains(&paper.id) {
                    continue;
                }
                let text = format!("{} {}", paper.title, paper.abstract_text).to_lowercase();
                let matches = keywords
                    .iter()
                    .filter(|k| text.contains(&k.to_lowercase()))
                    .count();
                let relevance = if keywords.is_empty() {
                    0.0
                } else {
                    matches as f64 / keywords.len() as f64
                };
                if relevance > 0.1 {
                    seen.insert(paper.id.clone());
                    related.push(crate::validator::RelatedWork {
                        paper_id: paper.id.clone(),
                        title: paper.title.chars().take(80).collect(),
                        year: paper.published.year(),
                        relevance_score: relevance,
                    });
                }
            }
        }
    }

    related.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));
    related.truncate(limit);
    related
}

fn render_validation_json(result: &crate::validator::ValidationResult) -> String {
    let dim_strs: Vec<&str> = result
        .innovation_score
        .dimensions
        .iter()
        .map(|d| match d {
            crate::validator::InnovationDimension::Method => "method",
            crate::validator::InnovationDimension::Task => "task",
            crate::validator::InnovationDimension::Evaluation => "evaluation",
            crate::validator::InnovationDimension::Theory => "theory",
            crate::validator::InnovationDimension::Application => "application",
        })
        .collect();

    let data = serde_json::json!({
        "question": result.question,
        "is_novel": result.is_novel,
        "novelty_level": result.novelty_level.as_str(),
        "innovation_score": {
            "overall": result.innovation_score.overall,
            "method": result.innovation_score.method,
            "task": result.innovation_score.task,
            "evaluation": result.innovation_score.evaluation,
            "dimensions": dim_strs,
            "reasoning": result.innovation_score.reasoning,
        },
        "related_works": result.related_works.iter().map(|w| {
            serde_json::json!({
                "paper_id": w.paper_id,
                "title": w.title,
                "year": w.year,
                "relevance_score": w.relevance_score,
            })
        }).collect::<Vec<_>>(),
        "suggestions": result.suggestions,
        "confidence": result.confidence,
    });
    serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".into())
}

fn handle_validate_interactive(
    db: &rairos_core::Database,
    mut no_llm: bool,
    mut json: bool,
    depth: &str,
    _model: Option<&str>,
) -> Result<()> {
    println!("🔬 Research Question Validator");
    println!("  输入研究问题开始验证");
    println!("  输入 no-llm 切换 LLM 分析");
    println!("  输入 depth quick/full 切换分析深度");
    println!("  输入 json 切换 JSON 输出");
    println!("  输入 q/quit 退出");
    println!();

    let mut depth_owned = depth.to_string();

    loop {
        let user_input = match std::io::stdin().lines().next() {
            Some(Ok(line)) => line.trim().to_string(),
            _ => break,
        };

        if user_input.is_empty() {
            continue;
        }

        match user_input.to_lowercase().as_str() {
            "q" | "quit" | "exit" => break,
            "no-llm" => {
                no_llm = !no_llm;
                let status = if no_llm { "禁用" } else { "启用" };
                println!("  ✓ LLM 分析已{}", status);
                continue;
            }
            "json" => {
                json = !json;
                let status = if json { "启用" } else { "禁用" };
                println!("  ✓ JSON 输出已{}", status);
                continue;
            }
            "depth quick" | "quick" => {
                depth_owned = "quick".into();
                println!("  ✓ 分析深度: quick");
                continue;
            }
            "depth full" | "full" => {
                depth_owned = "full".into();
                println!("  ✓ 分析深度: full");
                continue;
            }
            _ => {}
        }

        // Treat as question
        println!();
        println!("🔬 Validating: {}...", &user_input[..user_input.len().min(60)]);
        println!("   LLM: {} | 深度: {}",
            if no_llm { "禁用" } else { "启用" },
            depth_owned
        );

        let limit = if depth_owned == "full" { 10 } else { 5 };
        let related = find_related_works(db, &user_input, limit);
        let result = crate::validator::validate_rules(&user_input, related);

        if json {
            println!("{}", render_validation_json(&result));
        } else {
            println!("{}", crate::validator::render_result(&result));
        }
        println!();
    }

    Ok(())
}
