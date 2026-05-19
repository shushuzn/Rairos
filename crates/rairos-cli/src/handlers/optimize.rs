use anyhow::{Context, Result};
use rairos_core::constants::{LLM_BASE_URL, LLM_MODEL};
use rairos_crossover::{CodeCapsuleGene, get_top_code_candidates, save_code_capsule, get_all_code_capsules};
use rairos_core::Database;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Write;

pub fn handle_code_gene_list(
    _db: &Database,
    crate_name: Option<String>,
    limit: usize,
    format: &str,
) -> Result<()> {
    let capsules = get_top_code_candidates(limit);

    let filtered: Vec<CodeCapsuleGene> = if let Some(ref cn) = crate_name {
        let cn_clean = cn.replace(|c: char| !c.is_alphanumeric(), "");
        capsules.into_iter().filter(|c| {
            let target_clean = c.target_crate.replace(|c: char| !c.is_alphanumeric(), "");
            target_clean.contains(&cn_clean)
        }).collect()
    } else {
        capsules
    };

    if filtered.is_empty() {
        println!("No code genes found. Run 'rairos optimize --topic <tech> --crate <name>' first.");
        return Ok(());
    }

    if format == "json" {
        let out: Vec<serde_json::Value> = filtered
            .iter()
            .map(|c| {
                serde_json::json!({
                    "capsule_id": c.capsule_id,
                    "trigger_topic": c.trigger_topic,
                    "target_crate": c.target_crate,
                    "gap_type": c.gap_type,
                    "gap_location": c.gap_location,
                    "optimization": c.optimization,
                    "feedback_count": c.feedback_count,
                    "outcome_success_score": c.outcome_success_score,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("\n=== Code Optimization Genes ({}) ===\n", filtered.len());
        println!(
            "{:<36} {:<20} {:<15} {}",
            "ID", "TOPIC", "CRATE", "GAP TYPE"
        );
        println!("{}", "-".repeat(90));
        for gene in &filtered {
            let id_short = if gene.capsule_id.len() > 8 {
                &gene.capsule_id[..8]
            } else {
                &gene.capsule_id
            };
            let topic_short = if gene.trigger_topic.len() > 18 {
                format!("{}...", &gene.trigger_topic[..18])
            } else {
                gene.trigger_topic.clone()
            };
            println!(
                "{:<36} {:<20} {:<15} {}",
                id_short, topic_short, gene.target_crate, gene.gap_type
            );
        }
        println!();
    }
    Ok(())
}

pub fn handle_code_evolve(
    crate_name: Option<String>,
    max_crossovers: usize,
    format: &str,
) -> Result<()> {
    let all_capsules = get_top_code_candidates(100);

    let capsules: Vec<CodeCapsuleGene> = if let Some(ref cn) = crate_name {
        let cn_clean = cn.replace(|c: char| !c.is_alphanumeric(), "");
        all_capsules.into_iter().filter(|c| {
            let target_clean = c.target_crate.replace(|c: char| !c.is_alphanumeric(), "");
            target_clean.contains(&cn_clean)
        }).collect()
    } else {
        all_capsules
    };

    if capsules.len() < 2 {
        println!("Need at least 2 code genes for evolution. Current: {}", capsules.len());
        println!("Run 'rairos optimize --topic <tech>' to generate more.");
        return Ok(());
    }

    println!("=== Code Gene Evolution ===");
    println!("Available parents: {}", capsules.len());
    println!("Max crossovers: {}", max_crossovers);
    println!();

    let mut results = Vec::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(1);

    for i in 0..max_crossovers.min(capsules.len().saturating_sub(1)) {
        let idx_a = ((now as usize).wrapping_add(i * 17)) % capsules.len();
        let idx_b = ((now as usize).wrapping_mul(31).wrapping_add(i * 13)) % capsules.len();

        if idx_a == idx_b {
            continue;
        }

        let parent_a = &capsules[idx_a];
        let parent_b = &capsules[idx_b];

        let offspring_topic = format!(
            "{} + {}",
            parent_a.trigger_topic.chars().take(15).collect::<String>(),
            parent_b.trigger_topic.chars().take(15).collect::<String>()
        );

        let offspring_gap_type = if parent_a.gap_type == parent_b.gap_type {
            parent_a.gap_type.clone()
        } else {
            format!("{} / {}", parent_a.gap_type, parent_b.gap_type)
        };

        let offspring_optimization = format!(
            "Hybrid: {} + {}. {}",
            parent_a.optimization.chars().take(50).collect::<String>(),
            parent_b.optimization.chars().take(50).collect::<String>(),
            "Combined approach leveraging strengths from both parents."
        );

        let offspring = CodeCapsuleGene {
            capsule_id: uuid::Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            trigger_topic: offspring_topic,
            trigger_keywords: [
                parent_a.trigger_keywords.clone(),
                parent_b.trigger_keywords.clone()
            ].concat(),
            source_paper_id: format!("{} + {}", parent_a.capsule_id, parent_b.capsule_id),
            source_paper_title: format!("Evolved from {} + {}", parent_a.trigger_topic, parent_b.trigger_topic),
            target_crate: if parent_a.target_crate == parent_b.target_crate {
                parent_a.target_crate.clone()
            } else {
                format!("{} / {}", parent_a.target_crate, parent_b.target_crate)
            },
            gap_type: offspring_gap_type,
            gap_location: parent_a.gap_location.clone(),
            code_snippet: format!("// Combined from:\n// A: {}\n// B: {}",
                parent_a.code_snippet.chars().take(100).collect::<String>(),
                parent_b.code_snippet.chars().take(100).collect::<String>()
            ),
            optimization: offspring_optimization,
            outcome_success_score: (parent_a.outcome_success_score + parent_b.outcome_success_score) / 2.0,
            feedback_count: 0,
            evolved_generation: parent_a.evolved_generation.max(parent_b.evolved_generation) + 1,
            archetype: HashMap::new(),
            status: "active".to_string(),
            low_score_streak: 0,
            credibility_score: 0.5,
            credibility_badge: "medium".to_string(),
        };

        if save_code_capsule(&offspring).is_ok() {
            results.push(offspring);
        }
    }

    println!("Generated {} offspring\n", results.len());

    if format == "json" {
        let out: Vec<serde_json::Value> = results.iter().map(|c| {
            serde_json::json!({
                "capsule_id": c.capsule_id,
                "trigger_topic": c.trigger_topic,
                "target_crate": c.target_crate,
                "gap_type": c.gap_type,
                "evolved_generation": c.evolved_generation,
                "parents": c.source_paper_title,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("=== Crossover Results ===\n");
        println!(
            "{:<36} {:<20} {:<15} GEN",
            "ID", "TOPIC", "CRATE"
        );
        println!("{}", "-".repeat(80));
        for gene in &results {
            let id_short = if gene.capsule_id.len() > 8 {
                &gene.capsule_id[..8]
            } else {
                &gene.capsule_id
            };
            let topic_short = if gene.trigger_topic.len() > 18 {
                format!("{}...", &gene.trigger_topic[..18])
            } else {
                gene.trigger_topic.clone()
            };
            println!(
                "{:<36} {:<20} {:<15} {}",
                id_short, topic_short, gene.target_crate, gene.evolved_generation
            );
        }
        println!();
    }

    println!("{} new code genes saved to code gene pool", results.len());
    Ok(())
}

pub fn handle_optimize(
    db: &Database,
    topic: &str,
    crate_name: Option<String>,
    limit: usize,
    format: &str,
) -> Result<()> {
    println!("=== Code Optimization Analysis ===");
    println!("Topic: {}", topic);
    if let Some(ref cn) = crate_name {
        println!("Target crate: {}", cn);
    }
    println!();

    let mut papers = db.search_papers_smart(topic, 10)?;

    if papers.len() < 3 {
        println!("Not enough local papers ({}), fetching from arXiv...", papers.len());
        let rt = tokio::runtime::Runtime::new().ok();
        if let Some(ref rt) = rt {
            if let Ok(arxiv_papers) = rt.block_on(rairos_parser::search_arxiv_recent(topic, 15)) {
                for arxiv_paper in arxiv_papers {
                    let paper = rairos_core::Paper::with_metadata(
                        arxiv_paper.arxiv_id,
                        arxiv_paper.title,
                        arxiv_paper.abstract_text,
                        arxiv_paper.authors,
                        arxiv_paper.categories,
                        rairos_core::PaperMetadata::default(),
                    );
                    if db.insert_paper(&paper).is_ok() {
                        papers.push(paper);
                    }
                }
            }
        }
    }

    if papers.is_empty() {
        println!("No papers found for topic '{}'.", topic);
        return Ok(());
    }

    println!("Analyzing {} papers for optimization opportunities...\n", papers.len());

    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("LLM_API_KEY"))
        .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;
    let base_url = std::env::var("LLM_BASE_URL")
        .or_else(|_| std::env::var("OPENAI_BASE_URL"))
        .unwrap_or_else(|_| LLM_BASE_URL.to_string());
    let chat_model = std::env::var("LLM_MODEL")
        .unwrap_or_else(|_| LLM_MODEL.to_string());

    let context_parts: Vec<String> = papers.iter().enumerate().map(|(i, p)| {
        let abstract_text = if p.abstract_text.len() > 400 {
            format!("{}...", &p.abstract_text[..400])
        } else {
            p.abstract_text.clone()
        };
        format!(
            "[Paper {}] Title: {}\nAbstract: {}",
            i + 1,
            p.title,
            abstract_text
        )
    }).collect();
    let context_str = context_parts.join("\n\n");

    let target_crate_str = crate_name.clone().unwrap_or_else(|| "all crates".to_string());

    let system_prompt = "You are a code optimization analysis AI. Analyze research papers and identify concrete optimization opportunities for Rust codebases.

For each optimization opportunity, provide:
1. The specific technique/method from the paper
2. What code pattern it would replace
3. Where it would apply (crate/module)
4. Expected improvement

Format your response exactly as:

## Optimization Opportunities

### 1. [Technique Name]
- **Paper**: [paper title]
- **Gap Type**: [memory_gap/performance_gap/concurrency_gap/architecture_gap]
- **Location**: [crate::module::function or file:line]
- **Current Code**: [what inefficient code looks like]
- **Optimization**: [specific improvement]
- **Impact**: [high/medium/low]

## Gap Summary
List the gap types detected, e.g.: memory_gap, performance_gap, concurrency_gap";

    let user_prompt = format!(
        "Topic: {}\nTarget crate(s): {}\n\nPapers:\n{}\n\nAnalyze these papers and identify concrete code optimization opportunities for Rust projects. Focus on techniques that can be applied to improve performance, memory usage, or architecture.",
        topic, target_crate_str, context_str
    );

    println!("Running LLM analysis...\n");

    let rt = tokio::runtime::Runtime::new()?;

    let result = rt.block_on(async {
        let client = rairos_llm::client_async::AsyncClient::new(
            api_key,
            base_url,
            chat_model,
        );
        let messages = vec![
            std::collections::HashMap::from([
                ("role".to_string(), "user".to_string()),
                ("content".to_string(), user_prompt.clone()),
            ]),
        ];
        client.chat_completions(messages, None, Some(system_prompt), false).await
    });

    match result {
        Ok(analysis) => {
            println!("{}", "═".repeat(60));
            println!("Code Optimization Analysis for: {}", topic);
            println!("{}", "═".repeat(60));
            println!("{}", analysis);
            println!("{}", "═".repeat(60));

            let optimizations = parse_optimizations_from_analysis(&analysis);
            let mut saved_count = 0;

            for opt in optimizations.iter().take(limit) {
                let clean_location = opt.location.replace('`', "").trim().to_string();
                let gene = CodeCapsuleGene {
                    capsule_id: uuid::Uuid::new_v4().to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    trigger_topic: opt.technique.clone(),
                    trigger_keywords: vec![topic.to_string()],
                    source_paper_id: opt.paper.clone(),
                    source_paper_title: opt.paper.clone(),
                    target_crate: clean_location.clone(),
                    gap_type: opt.gap_type.replace('`', "").trim().to_string(),
                    gap_location: clean_location,
                    code_snippet: opt.current_code.clone(),
                    optimization: opt.improvement.clone(),
                    outcome_success_score: 0.5,
                    feedback_count: 0,
                    evolved_generation: 0,
                    archetype: HashMap::new(),
                    status: "active".to_string(),
                    low_score_streak: 0,
                    credibility_score: 0.5,
                    credibility_badge: "medium".to_string(),
                };

                if save_code_capsule(&gene).is_ok() {
                    saved_count += 1;
                }
            }

            println!("\n📚 {} optimizations saved to code gene pool", saved_count);
            if format == "json" {
                let out = serde_json::json!({
                    "topic": topic,
                    "crate": crate_name,
                    "papers_analyzed": papers.len(),
                    "optimizations_found": optimizations.len(),
                    "analysis": analysis,
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            }
        }
        Err(e) => {
            eprintln!("Analysis failed: {}", e);
        }
    }

    Ok(())
}

struct ParsedOptimization {
    technique: String,
    paper: String,
    gap_type: String,
    location: String,
    current_code: String,
    improvement: String,
}

pub fn handle_gap_suggest_code(
    db: &Database,
    gap_id: &str,
    crate_name: Option<String>,
    format: &str,
) -> Result<()> {
    let gap = db.get_gap(gap_id)?;

    let gap = match gap {
        Some(g) => g,
        None => {
            println!("Gap not found: {}", gap_id);
            return Ok(());
        }
    };

    println!("=== Gap to Code Optimization ===");
    println!("Gap ID: {}", gap.id);
    println!("Topic: {}", gap.topic);
    println!("Gap Type: {}", gap.gap_type);
    println!("Description: {}", gap.description);
    println!();

    let target_crate = crate_name.unwrap_or_else(|| "rairos-llm".to_string());

    let system_prompt = r#"You are a Rust code optimization AI. Given a research gap, suggest CONCRETE code optimizations for a Rust crate.

IMPORTANT: For each suggestion, you MUST provide ACTUAL RUST CODE in a ```rust code block.

Format your response exactly as:

## Code Optimization Suggestions

### 1. [Descriptive Title]
- **Replaces**: [what existing code pattern or problem this fixes]
- **Expected Impact**: [high/medium/low with brief justification]
- **Code Pattern**:
```rust
// BRIEF COMMENT: what this does
pub fn example_function() {
    // actual working Rust code with proper types and logic
}
```
- **Why it works**: [1-2 sentences on the performance/quality mechanism]

Focus on practical, implementable patterns. Prefer:
- Parallelization with rayon, tokio
- Caching and memoization
- Data structure optimizations
- Metric instrumentation with tracing
- Property-based testing with proptest
- Benchmarking with criterion

Generate COMPLETE, COMPILABLE code patterns that can be directly inserted into a Rust project.

CRITICAL: Before suggesting any optimization, you MUST check the EXISTING CODE in the target crate and EXISTING GENES to avoid duplication."#;

    let gap_keywords: Vec<String> = gap.topic
        .split_whitespace()
        .map(|s| s.to_lowercase())
        .chain(gap.gap_type.split_whitespace().map(|s| s.to_lowercase()))
        .collect();

    let existing_code_context = search_existing_code(&target_crate, &gap.topic, &gap_keywords);
    let existing_gene_context = get_existing_gene_context(&target_crate, &gap_keywords);

    if !existing_code_context.is_empty() {
        println!("\n🔍 Searched existing code in crate...");
    }
    if !existing_gene_context.is_empty() {
        println!("\n🔍 Found {} similar existing genes...", existing_gene_context.lines().count() / 3);
    }

    let user_prompt = format!(
        "Research Gap:\n  Topic: {}\n  Type: {}\n  Description: {}\n\nTarget crate: {}{}{}\n\nIMPORTANT: DO NOT duplicate existing code patterns or previously suggested genes. Build upon or extend them instead.",
        gap.topic,
        gap.gap_type,
        gap.description,
        target_crate,
        existing_code_context,
        existing_gene_context
    );

    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("LLM_API_KEY"))
        .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;
    let base_url = std::env::var("LLM_BASE_URL")
        .or_else(|_| std::env::var("OPENAI_BASE_URL"))
        .unwrap_or_else(|_| LLM_BASE_URL.to_string());
    let chat_model = std::env::var("LLM_MODEL")
        .unwrap_or_else(|_| LLM_MODEL.to_string());

    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(async {
        let client = rairos_llm::client_async::AsyncClient::new(
            api_key,
            base_url,
            chat_model,
        );
        let messages = vec![
            std::collections::HashMap::from([
                ("role".to_string(), "user".to_string()),
                ("content".to_string(), user_prompt.clone()),
            ]),
        ];
        client.chat_completions(messages, None, Some(system_prompt), false).await
    });

    match result {
        Ok(suggestions) => {
            println!("{}", "═".repeat(60));
            println!("Code Optimization Suggestions for Gap: {}", gap_id);
            println!("{}", "═".repeat(60));
            println!("{}", suggestions);
            println!("{}", "═".repeat(60));

            let parsed: Vec<(String, String, String)> = parse_suggestions_from_text(&suggestions);
            let mut saved_count = 0;

            for (title, _impact, pattern) in parsed.iter().take(5) {
                let gene = CodeCapsuleGene {
                    capsule_id: uuid::Uuid::new_v4().to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    trigger_topic: gap.topic.clone(),
                    trigger_keywords: vec![gap.gap_type.clone()],
                    source_paper_id: gap.id.clone(),
                    source_paper_title: format!("Research Gap: {}", gap.topic),
                    target_crate: target_crate.clone(),
                    gap_type: gap.gap_type.clone(),
                    gap_location: target_crate.clone(),
                    code_snippet: pattern.clone(),
                    optimization: title.clone(),
                    outcome_success_score: 0.5,
                    feedback_count: 0,
                    evolved_generation: 0,
                    archetype: HashMap::new(),
                    status: "active".to_string(),
                    low_score_streak: 0,
                    credibility_score: 0.5,
                    credibility_badge: "medium".to_string(),
                };

                if save_code_capsule(&gene).is_ok() {
                    saved_count += 1;
                }
            }

            println!("\n📚 {} optimization genes saved to code gene pool", saved_count);

            if format == "json" {
                let out = serde_json::json!({
                    "gap_id": gap_id,
                    "topic": gap.topic,
                    "gap_type": gap.gap_type,
                    "suggestions": suggestions,
                    "saved_count": saved_count,
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            }
        }
        Err(e) => {
            eprintln!("Failed to generate suggestions: {}", e);
        }
    }

    Ok(())
}

fn parse_suggestions_from_text(text: &str) -> Vec<(String, String, String)> {
    let mut suggestions = Vec::new();

    let sections: Vec<&str> = text.split("### ").collect();

    for section in sections.iter().skip(1) {
        let lines: Vec<&str> = section.lines().collect();
        if lines.is_empty() {
            continue;
        }

        let title = lines[0].trim().to_string();
        if title.is_empty() {
            continue;
        }

        let section_text = lines[1..].join("\n");

        let impact = extract_field(&section_text, "Expected Impact")
            .unwrap_or_else(|| "medium".to_string());

        let pattern = extract_code_block(&section_text)
            .or_else(|| extract_field(&section_text, "Code Pattern"))
            .unwrap_or_default();

        if !pattern.is_empty() {
            suggestions.push((title, impact, pattern));
        }
    }

    suggestions
}

fn extract_field(text: &str, field: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with(&format!("- **{}**:", field)) {
            let value = line.trim_start_matches(&format!("- **{}**:", field)).trim();
            return Some(value.to_string());
        }
    }
    None
}

fn extract_code_block(text: &str) -> Option<String> {
    let marker = "```rust";
    if let Some(start) = text.find(marker) {
        let after_marker = &text[start + marker.len()..];
        if let Some(end) = after_marker.find("```") {
            let code = &after_marker[..end];
            let trimmed = code.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn search_existing_code(crate_name: &str, gap_topic: &str, gap_keywords: &[String]) -> String {
    let mut context = String::new();

    let search_terms: Vec<&str> = if !gap_topic.is_empty() {
        gap_topic.split_whitespace().take(5).collect()
    } else {
        gap_keywords.iter().take(5).map(|s| s.as_str()).collect()
    };

    if search_terms.is_empty() {
        return context;
    }

    for term in &search_terms {
        if let Ok(output) = std::process::Command::new("grep")
            .args(&["-r", "-l", "-i", term, &format!("crates/{}/src", crate_name.replace('-', "_"))])
            .output()
        {
            if output.status.success() {
                let files = String::from_utf8_lossy(&output.stdout);
                for file in files.lines().take(3) {
                    if let Ok(content) = std::fs::read_to_string(file.trim()) {
                        let snippet: String = content
                            .lines()
                            .filter(|l| l.to_lowercase().contains(&term.to_lowercase()))
                            .take(5)
                            .map(|l| format!("  {}\n", l.trim()))
                            .collect();
                        if !snippet.is_empty() {
                            context.push_str(&format!("\n=== Found in {} (for '{}') ===\n{}\n", file, term, snippet));
                        }
                    }
                }
            }
        }
    }

    context
}

fn get_existing_gene_context(target_crate: &str, gap_keywords: &[String]) -> String {
    let capsules = get_all_code_capsules();
    let relevant: Vec<_> = capsules
        .iter()
        .filter(|c| {
            if !target_crate.is_empty() && !c.target_crate.contains(target_crate) {
                return false;
            }
            c.trigger_keywords.iter().any(|k| {
                gap_keywords.iter().any(|gk| gk.to_lowercase().contains(&k.to_lowercase()))
            })
        })
        .take(3)
        .collect();

    if relevant.is_empty() {
        return String::new();
    }

    let mut ctx = String::from("\n\n=== Existing code genes for similar gaps (DO NOT duplicate) ===\n");
    for gene in &relevant {
        ctx.push_str(&format!(
            "- {} (gap: {}, crate: {})\n  Code: {:.50}...\n",
            gene.optimization.trim(),
            gene.gap_type,
            gene.target_crate,
            gene.code_snippet.trim()
        ));
    }
    ctx
}

fn parse_optimizations_from_analysis(analysis: &str) -> Vec<ParsedOptimization> {
    let mut optimizations = Vec::new();
    let lines: Vec<&str> = analysis.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("### ") || (line.starts_with("### ") && line.contains("Optimization")) {
            let technique = line.trim_start_matches("### ").trim();
            let mut paper = String::new();
            let mut gap_type = String::new();
            let mut location = String::new();
            let mut current_code = String::new();
            let mut improvement = String::new();

            i += 1;
            while i < lines.len() {
                let curr = lines[i].trim();
                if curr.starts_with("### ") || (curr.starts_with("## ") && !curr.contains("Optimization")) {
                    break;
                }
                if curr.starts_with("- **Paper**:") {
                    paper = curr.trim_start_matches("- **Paper**:").trim().to_string();
                } else if curr.starts_with("- **Gap Type**:") {
                    gap_type = curr.trim_start_matches("- **Gap Type**:").trim().to_string();
                } else if curr.starts_with("- **Location**:") {
                    location = curr.trim_start_matches("- **Location**:").trim().to_string();
                } else if curr.starts_with("- **Current Code**:") {
                    current_code = curr.trim_start_matches("- **Current Code**:").trim().to_string();
                } else if curr.starts_with("- **Optimization**:") {
                    improvement = curr.trim_start_matches("- **Optimization**:").trim().to_string();
                }
                i += 1;
            }

            if !technique.is_empty() && !gap_type.is_empty() {
                optimizations.push(ParsedOptimization {
                    technique: technique.to_string(),
                    paper,
                    gap_type,
                    location,
                    current_code,
                    improvement,
                });
            }
        } else {
            i += 1;
        }
    }

    optimizations
}

pub fn handle_gap_code_link(db: &Database, gap_id: Option<String>) -> Result<()> {
    let gaps = db.list_gaps(100, 0)?;
    let code_genes = get_top_code_candidates(100);

    if let Some(gid) = gap_id {
        let gap = gaps.iter().find(|g| g.id == gid);
        match gap {
            Some(g) => {
                let display = if g.topic.is_empty() { &g.description } else { &g.topic };
                println!("Gap: {}", display);
                println!("Type: {}", g.gap_type);
                println!();

                let linked: Vec<_> = code_genes.iter().filter(|c| {
                    c.trigger_topic.to_lowercase().contains(&display.to_lowercase()) ||
                    c.trigger_keywords.iter().any(|k| display.to_lowercase().contains(&k.to_lowercase()))
                }).collect();

                if linked.is_empty() {
                    println!("No linked code genes found.");
                    println!("Run 'rairos gap-suggest-code --gap-id {}' to generate suggestions.", gid);
                } else {
                    println!("Linked Code Genes ({}):", linked.len());
                    for gene in linked {
                        println!("  - {} [{}] @ {}", gene.trigger_topic.chars().take(50).collect::<String>(), gene.gap_type, gene.target_crate);
                    }
                }
            }
            None => {
                println!("Gap not found: {}", gid);
            }
        }
    } else {
        println!("{}", "═".repeat(60));
        println!("🔗 Gap → Code Gene Linkage");
        println!("{}", "═".repeat(60));
        println!();

        let mut link_count = 0;
        for gap in &gaps {
            let display = if gap.topic.is_empty() {
                gap.description.chars().take(40).collect::<String>()
            } else {
                gap.topic.chars().take(40).collect::<String>()
            };

            let linked: Vec<_> = code_genes.iter().filter(|c| {
                let gap_search = if gap.topic.is_empty() { &gap.description } else { &gap.topic };
                c.trigger_topic.to_lowercase().contains(&gap_search.to_lowercase()) ||
                c.trigger_keywords.iter().any(|k| gap_search.to_lowercase().contains(&k.to_lowercase()))
            }).collect();

            if !linked.is_empty() {
                link_count += 1;
                println!("[{}] {} → {} code genes", gap.gap_type, display, linked.len());
            }
        }

        println!();
        println!("Total: {} gaps with linked code genes", link_count);
        println!();
        println!("Use 'rairos gap-code-link --gap-id <id>' for details.");
    }

    Ok(())
}

pub fn handle_workflow_stats(db: &Database) -> Result<()> {
    let gaps = db.list_gaps(100, 0)?;
    let code_genes = get_top_code_candidates(100);

    let research_gap_types: HashSet<String> = gaps.iter().map(|g| {
        if g.gap_type.is_empty() { g.category.clone() } else { g.gap_type.clone() }
    }).collect();
    let code_gap_types: HashSet<String> = code_genes.iter().map(|g| g.gap_type.clone()).collect();

    let linked_gaps: Vec<_> = gaps.iter().filter(|g| {
        code_genes.iter().any(|c| {
            let gap_search = if g.topic.is_empty() { &g.description } else { &g.topic };
            c.trigger_topic.to_lowercase().contains(&gap_search.to_lowercase()) ||
            c.trigger_keywords.iter().any(|k| gap_search.to_lowercase().contains(&k.to_lowercase()))
        })
    }).collect();

    println!("{}", "═".repeat(60));
    println!("📊 Rairos Workflow Statistics");
    println!("{}", "═".repeat(60));
    println!();
    println!("📚 Papers: {}", db.stats().map(|s| s.total).unwrap_or(0));
    println!("🔍 Research Gaps: {}", gaps.len());
    println!("💻 Code Genes: {}", code_genes.len());
    println!("🔗 Linked (Gap→Code): {}", linked_gaps.len());
    println!();

    println!("Research Gap Types:");
    for gt in &research_gap_types {
        if !gt.is_empty() {
            println!("  - {}", gt);
        }
    }
    println!();

    println!("Code Gap Types:");
    for gt in &code_gap_types {
        if !gt.is_empty() {
            println!("  - {}", gt);
        }
    }
    println!();

    if !linked_gaps.is_empty() {
        println!("🔗 Linked Gaps (have code optimization suggestions):");
        for gap in linked_gaps.iter().take(5) {
            let display = if gap.topic.is_empty() {
                gap.description.chars().take(40).collect::<String>()
            } else {
                gap.topic.chars().take(40).collect::<String>()
            };
            println!("  - {}: {}", display, gap.gap_type);
        }
    }

    println!();
    println!("{}", "═".repeat(60));

    Ok(())
}

pub fn handle_optimize_pipeline(
    db: &Database,
    topic: &str,
    crate_name: Option<String>,
    optimizations: usize,
    evolutions: usize,
) -> Result<()> {
    println!("{}", "═".repeat(60));
    println!("🚀 Optimize Pipeline: {} → Code Optimization → Evolution", topic);
    println!("{}", "═".repeat(60));
    println!();

    println!("Step 1/3: Detecting research gaps...");
    let gaps = db.list_gaps(10, 0)?;
    let existing_gaps: Vec<_> = gaps.iter().filter(|g| {
        g.topic.to_lowercase().contains(&topic.to_lowercase()) ||
        g.description.to_lowercase().contains(&topic.to_lowercase())
    }).collect();

    if !existing_gaps.is_empty() {
        println!("  Found {} existing gaps for '{}'", existing_gaps.len(), topic);
    } else {
        println!("  No existing gaps found, will generate from papers");
    }
    println!();

    println!("Step 2/3: Generating code optimizations...");
    handle_optimize(db, topic, crate_name.clone(), optimizations, "table")?;
    println!();

    println!("Step 3/3: Evolving code genes...");
    if evolutions > 0 {
        handle_code_evolve(crate_name.clone(), evolutions, "table")?;
    }
    println!();

    let code_genes = get_top_code_candidates(100);
    println!("{}", "═".repeat(60));
    println!("✅ Pipeline Complete!");
    println!("{}", "═".repeat(60));
    println!("  Topic: {}", topic);
    println!("  Code genes available: {}", code_genes.len());
    println!();
    println!("  Next steps:");
    println!("    - View genes: rairos code-gene-list");
    println!("    - Give feedback: rairos code-gene-feedback --id <uuid> --positive");
    println!("    - Export: rairos code-gene-export --output <file>");
    println!("{}", "═".repeat(60));

    Ok(())
}

pub fn handle_code_gene_feedback(
    gene_id: &str,
    positive: bool,
) -> Result<()> {
    let all_genes = get_top_code_candidates(1000);

    let gene = all_genes.iter().find(|g| g.capsule_id == gene_id);

    match gene {
        Some(g) => {
            let new_score = if positive {
                (g.outcome_success_score + 0.1).min(1.0)
            } else {
                (g.outcome_success_score - 0.1).max(0.0)
            };
            let new_count = g.feedback_count + 1;

            let updated_gene = CodeCapsuleGene {
                capsule_id: g.capsule_id.clone(),
                created_at: g.created_at.clone(),
                trigger_topic: g.trigger_topic.clone(),
                trigger_keywords: g.trigger_keywords.clone(),
                source_paper_id: g.source_paper_id.clone(),
                source_paper_title: g.source_paper_title.clone(),
                target_crate: g.target_crate.clone(),
                gap_type: g.gap_type.clone(),
                gap_location: g.gap_location.clone(),
                code_snippet: g.code_snippet.clone(),
                optimization: g.optimization.clone(),
                outcome_success_score: new_score,
                feedback_count: new_count,
                evolved_generation: g.evolved_generation,
                archetype: g.archetype.clone(),
                status: g.status.clone(),
                low_score_streak: if new_score < 0.3 { g.low_score_streak + 1 } else { 0 },
                credibility_score: g.credibility_score,
                credibility_badge: if new_score > 0.7 { "high".to_string() } else if new_score > 0.4 { "medium".to_string() } else { "low".to_string() },
            };

            println!("  Updating feedback for gene: {}", g.trigger_topic.chars().take(40).collect::<String>());
            println!("  Score: {:.2} → {:.2}", g.outcome_success_score, new_score);
            println!("  Feedback count: {} → {}", g.feedback_count, new_count);
            println!("  Badge: {} → {}", g.credibility_badge, updated_gene.credibility_badge);

            save_code_capsule(&updated_gene)?;
            println!("\n✅ Feedback recorded!");
        }
        None => {
            println!("Code gene not found: {}", gene_id);
            println!("Use 'rairos code-gene-list' to see available genes.");
        }
    }

    Ok(())
}

pub fn handle_code_gene_export(
    output: &str,
    crate_name: Option<String>,
) -> Result<()> {
    let all_genes = get_top_code_candidates(1000);

    let genes: Vec<CodeCapsuleGene> = if let Some(ref cn) = crate_name {
        let cn_clean = cn.replace(|c: char| !c.is_alphanumeric(), "");
        all_genes.into_iter().filter(|c| {
            let target_clean = c.target_crate.replace(|c: char| !c.is_alphanumeric(), "");
            target_clean.contains(&cn_clean)
        }).collect()
    } else {
        all_genes
    };

    if genes.is_empty() {
        println!("No code genes to export.");
        return Ok(());
    }

    let json = serde_json::to_string_pretty(&genes)?;
    std::fs::write(output, json)?;

    println!("Exported {} code genes to {}", genes.len(), output);
    println!("  File: {}", output);
    println!("  Genes: {}", genes.len());
    if let Some(ref cn) = crate_name {
        println!("  Filter: crate={}", cn);
    }

    Ok(())
}

pub fn handle_code_gene_clean(
    min_score: f64,
    min_feedback: i32,
    min_code_length: usize,
    dry_run: bool,
) -> Result<()> {
    let all_genes = get_all_code_capsules();

    let low_quality: Vec<CodeCapsuleGene> = all_genes.iter()
        .filter(|g| {
            g.outcome_success_score < min_score ||
            g.feedback_count < min_feedback ||
            g.code_snippet.len() < min_code_length
        })
        .cloned()
        .collect();

    let high_quality: Vec<CodeCapsuleGene> = all_genes.iter()
        .filter(|g| {
            g.outcome_success_score >= min_score &&
            g.feedback_count >= min_feedback &&
            g.code_snippet.len() >= min_code_length
        })
        .cloned()
        .collect();

    println!("{}", "═".repeat(60));
    println!("🧹 Code Gene Cleanup Report");
    println!("{}", "═".repeat(60));
    println!();
    println!("  Threshold: score >= {:.2}, feedback >= {}, code_len >= {}",
        min_score, min_feedback, min_code_length);
    println!("  Total genes: {}", all_genes.len());
    println!("  Low quality: {} (will remove)", low_quality.len());
    println!("  High quality: {} (will keep)", high_quality.len());
    println!();

    if low_quality.is_empty() {
        println!("No low-quality genes found. Nothing to clean.");
        return Ok(());
    }

    println!("Low quality genes to remove:");
    for gene in &low_quality {
        println!("  - {}: score={:.2}, feedback={}",
            gene.trigger_topic.chars().take(40).collect::<String>(),
            gene.outcome_success_score,
            gene.feedback_count
        );
    }
    println!();

    if dry_run {
        println!("🔍 Dry-run mode: no genes were deleted.");
        println!("  Run without --dry-run to actually delete.");
    } else {
        let path = rairos_crossover::code_gene_pool_path();
        let mut file = std::fs::File::create(&path)?;
        for gene in &high_quality {
            let json = serde_json::to_string(gene)?;
            file.write_all(json.as_bytes())?;
            file.write_all(b"\n")?;
        }
        println!("✅ Deleted {} low-quality genes.", low_quality.len());
        println!("  Kept {} high-quality genes.", high_quality.len());
    }

    println!();
    println!("{}", "═".repeat(60));

    Ok(())
}

pub fn handle_code_gene_sync_to_issue(
    ids: &str,
    crate_name: Option<String>,
    min_score: f64,
) -> Result<()> {
    let all_genes = get_top_code_candidates(1000);

    let genes: Vec<CodeCapsuleGene> = {
        let filtered: Vec<CodeCapsuleGene> = if ids == "all" {
            all_genes
        } else {
            let id_list: Vec<&str> = ids.split(',').map(|s| s.trim()).collect();
            all_genes.into_iter().filter(|g| id_list.iter().any(|id| g.capsule_id.starts_with(id))).collect()
        };

        let filtered = if let Some(ref cn) = crate_name {
            let cn_clean = cn.replace(|c: char| !c.is_alphanumeric(), "");
            filtered.into_iter().filter(|c| {
                let target_clean = c.target_crate.replace(|c: char| !c.is_alphanumeric(), "");
                target_clean.contains(&cn_clean)
            }).collect()
        } else {
            filtered
        };

        filtered.into_iter().filter(|g| g.outcome_success_score >= min_score).collect()
    };

    if genes.is_empty() {
        println!("No code genes match the criteria.");
        return Ok(());
    }

    println!("Syncing {} genes to GitHub Issues...\n", genes.len());

    let repo = "shushuzn/Rairos";
    let mut created = 0;
    let mut errors = 0;

    for gene in &genes {
        let topic = if gene.trigger_topic.is_empty() {
            gene.optimization.chars().take(60).collect::<String>()
        } else {
            gene.trigger_topic.chars().take(60).collect::<String>()
        };
        let crate_name = gene.target_crate.split(':').next().unwrap_or(&gene.target_crate);
        let title = format!(
            "[code-gene] {}: {}",
            crate_name,
            topic
        );

        let code_snippet = if gene.code_snippet.contains("fn ") || gene.code_snippet.contains("struct ") || gene.code_snippet.contains("pub ") {
            format!("```rust\n{}\n```", gene.code_snippet.trim())
        } else {
            format!("```\n{}\n```", gene.code_snippet.trim())
        };

        let body = format!(
            r#"## Gene Information

**ID:** `{}`
**Crate:** `{}`
**Gap Type:** `{}`

## Optimization

{}

## Code Snippet

{}

## Metrics

| Metric | Value |
|--------|-------|
| Score | {:.2} |
| Feedback Count | {} |
| Generation | {} |

## Status

- [ ] Not started
- [ ] In progress
- [ ] Implemented
- [ ] Verified
"#,
            gene.capsule_id,
            gene.target_crate,
            gene.gap_type,
            gene.optimization,
            code_snippet,
            gene.outcome_success_score,
            gene.feedback_count,
            gene.evolved_generation
        );

        let gap_type_labels: Vec<String> = gene.gap_type
            .split(|c| c == ',' || c == '/')
            .flat_map(|t| t.split_whitespace())
            .map(|t| t.trim_matches(|c| c == ',' || c == '/' || c == ' '))
            .filter(|t| !t.is_empty())
            .filter(|t| *t != "gap" && *t != "gaps")
            .map(|t| t.replace("_gap", "").replace("_gaps", "").to_lowercase())
            .filter(|t| !t.is_empty())
            .map(|t| format!("gap-type-{}", t))
            .collect();
        let gap_labels_str = if gap_type_labels.is_empty() {
            String::new()
        } else {
            format!(",{}", gap_type_labels.join(","))
        };
        let labels_arg = format!("code-gene,crate-{}{}", crate_name, gap_labels_str);
        let output = std::process::Command::new("gh")
            .args(&["issue", "create", "--repo", repo, "--title", &title, "--body", &body, "--label", &labels_arg])
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("  ✅ Created: {}", stdout.trim());
            created += 1;
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("  ❌ Error for {}: {}", gene.capsule_id.chars().take(8).collect::<String>(), stderr.trim());
            errors += 1;
        }
    }

    println!("\n{} created, {} errors", created, errors);
    Ok(())
}

pub fn handle_code_gene_sync_from_issue(
    _issues: &str,
    repo: &str,
) -> Result<()> {
    println!("Syncing code genes from GitHub Issues in {}\n", repo);

    let output = std::process::Command::new("gh")
        .args(&["issue", "list", "--repo", repo, "--label", "code-gene", "--limit", "100", "--json", "number,title,body,labels"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to list issues: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let issues_data: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .context("Failed to parse JSON response")?;

    println!("Found {} code-gene issues\n", issues_data.len());

    let mut imported = 0;
    let mut skipped = 0;
    let mut errors = 0;

    for issue in &issues_data {
        let number = issue["number"].as_i64().unwrap_or(0);
        let body = issue["body"].as_str().unwrap_or("");
        let labels: Vec<String> = issue["labels"].as_array()
            .map(|arr| arr.iter().filter_map(|l| l["name"].as_str().map(String::from)).collect())
            .unwrap_or_default();

        if let Some(gene) = parse_gene_from_issue_body(number, body, &labels) {
            match save_code_capsule(&gene) {
                Ok(_) => {
                    println!("  ✅ Imported #{}: {}", number, gene.trigger_topic.chars().take(50).collect::<String>());
                    imported += 1;
                }
                Err(e) => {
                    eprintln!("  ❌ Error saving #{}: {}", number, e);
                    errors += 1;
                }
            }
        } else {
            println!("  ⏭️  Skipped #{}: could not parse", number);
            skipped += 1;
        }
    }

    println!("\n{} imported, {} skipped, {} errors", imported, skipped, errors);
    Ok(())
}

fn parse_gene_from_issue_body(number: i64, body: &str, labels: &[String]) -> Option<CodeCapsuleGene> {
    let capsule_id = format!("issue-{}", number);

    let title = body.lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches("# ").to_string())
        .unwrap_or_else(|| format!("From issue #{}", number));

    let trigger_topic = labels.iter()
        .find(|l| l.starts_with("crate:"))
        .map(|l| l.replace("crate:", ""))
        .unwrap_or_else(|| title.clone());

    let gap_type = labels.iter()
        .find(|l| l.starts_with("gap-type:"))
        .map(|l| l.replace("gap-type:", ""))
        .unwrap_or_else(|| "evaluation".to_string());

    let code_snippet = extract_github_code_block(body);

    let optimization = extract_section(body, "## Optimization", "## Code")
        .or_else(|| extract_section(body, "## Description", "## Code"))
        .unwrap_or_default();

    let target_crate = labels.iter()
        .find(|l| l.starts_with("crate:"))
        .cloned()
        .map(|l| l.replace("crate:", ""))
        .unwrap_or_default();

    Some(CodeCapsuleGene {
        capsule_id,
        created_at: chrono::Utc::now().to_rfc3339(),
        trigger_topic,
        trigger_keywords: vec![],
        source_paper_id: String::new(),
        source_paper_title: String::new(),
        target_crate,
        gap_type,
        gap_location: format!("GitHub Issue #{}", number),
        code_snippet,
        optimization,
        outcome_success_score: 0.5,
        feedback_count: 0,
        evolved_generation: 0,
        archetype: Default::default(),
        status: Default::default(),
        low_score_streak: 0,
        credibility_score: 0.5,
        credibility_badge: "medium".to_string(),
    })
}

fn extract_github_code_block(text: &str) -> String {
    let markers = ["```rust", "```", "```toml", "```python", "```cpp"];
    for marker in &markers {
        if let Some(start) = text.find(marker) {
            let after_marker = &text[start + marker.len()..];
            if let Some(end) = after_marker.find("```") {
                let code = after_marker[..end].trim();
                if !code.is_empty() {
                    return code.to_string();
                }
            }
        }
    }
    String::new()
}

fn extract_section(text: &str, start_marker: &str, end_marker: &str) -> Option<String> {
    let start_idx = text.find(start_marker)? + start_marker.len();
    let remaining = &text[start_idx..];
    let end_idx = remaining.find(end_marker).unwrap_or(remaining.len());
    let section = remaining[..end_idx].trim();
    if section.is_empty() {
        None
    } else {
        Some(section.to_string())
    }
}
