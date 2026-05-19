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

pub fn handle_code_gene_add(
    crate_name: &str,
    gap_type: &str,
    code_snippet: &str,
    optimization: &str,
    keywords: &str,
) -> Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let capsule_id = format!("{:x}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos());

    let keywords: Vec<String> = if keywords.is_empty() {
        vec![]
    } else {
        keywords.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    };

    let gene = CodeCapsuleGene {
        capsule_id,
        created_at: chrono::Utc::now().to_rfc3339(),
        trigger_topic: String::new(),
        trigger_keywords: keywords.clone(),
        source_paper_id: String::new(),
        source_paper_title: String::new(),
        target_crate: crate_name.to_string(),
        gap_type: gap_type.to_string(),
        gap_location: crate_name.to_string(),
        code_snippet: code_snippet.to_string(),
        optimization: optimization.to_string(),
        outcome_success_score: 0.5,
        feedback_count: 0,
        evolved_generation: 0,
        archetype: std::collections::HashMap::new(),
        status: "active".to_string(),
        low_score_streak: 0,
        credibility_score: 0.5,
        credibility_badge: "medium".to_string(),
    };

    save_code_capsule(&gene)?;
    println!("✅ Code gene added to pool");
    println!("  ID: {}", gene.capsule_id);
    println!("  Crate: {}", gene.target_crate);
    println!("  Gap type: {}", gene.gap_type);
    println!("  Keywords: {:?}", gene.trigger_keywords);
    println!("  Code length: {} chars", gene.code_snippet.len());

    Ok(())
}

pub fn handle_code_gene_feedback(
    gene_id: &str,
    positive: bool,
) -> Result<()> {
    let all_genes = get_top_code_candidates(1000);

    // Support short ID prefix matching (e.g., "695eb954" matches "695eb954-c820-414a-ae13-1de2dca310ee")
    let gene = all_genes.iter().find(|g| g.capsule_id == gene_id || g.capsule_id.starts_with(gene_id));

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

pub fn handle_code_gene_sync_to_pr(
    ids: &str,
    repo: &str,
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

    println!("Syncing {} genes to GitHub PRs...\n", genes.len());

    let mut created = 0;
    let mut errors = 0;

    for gene in &genes {
        let short_id = &gene.capsule_id[..8.min(gene.capsule_id.len())];
        let branch_name = format!("code-gene/{}", short_id);

        let topic = if gene.trigger_topic.is_empty() {
            gene.optimization.chars().take(50).collect::<String>()
        } else {
            gene.trigger_topic.chars().take(50).collect::<String>()
        };
        let crate_name = gene.target_crate.split(':').next().unwrap_or(&gene.target_crate);

        println!("  Processing gene {}...", short_id);

        // Create branch
        let branch_output = std::process::Command::new("git")
            .args(&["checkout", "-b", &branch_name])
            .output()?;

        if !branch_output.status.success() {
            let stderr = String::from_utf8_lossy(&branch_output.stderr);
            // Branch might already exist, try to switch to it
            let switch_output = std::process::Command::new("git")
                .args(&["checkout", &branch_name])
                .output()?;

            if !switch_output.status.success() {
                eprintln!("  ❌ Failed to create/switch to branch {}: {}", branch_name, stderr.trim());
                errors += 1;
                continue;
            }
        }

        // Add code to lib.rs
        let lib_rs_path = format!("crates/{}/src/lib.rs", crate_name);
        let existing_content = match std::fs::read_to_string(&lib_rs_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  ❌ Failed to read {}: {}", lib_rs_path, e);
                errors += 1;
                continue;
            }
        };

        // Unescape code snippet
        let unescaped_snippet = gene.code_snippet
            .replace("\\\"", "\"")
            .replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("\\r", "\r");

        // Handle test module if present
        let code_to_append = if unescaped_snippet.contains("#[cfg(test)]") && unescaped_snippet.contains("mod tests {") {
            let cfg_test_pos = unescaped_snippet.find("#[cfg(test)]").unwrap_or(usize::MAX);
            let non_test_code = unescaped_snippet[..cfg_test_pos].trim();

            let test_module_start = unescaped_snippet[cfg_test_pos..].find("mod tests {").unwrap_or(usize::MAX);
            let after_mod = &unescaped_snippet[cfg_test_pos..][test_module_start + "mod tests {".len()..];
            let last_brace = after_mod.rfind('}').unwrap_or(usize::MAX);
            let test_inner = &after_mod[..last_brace];

            let test_funcs = test_inner.lines()
                .filter(|l| !l.trim().is_empty() && !l.trim().starts_with("use super::*;"))
                .collect::<Vec<_>>()
                .join("\n");

            format!("{}\n\n// ========== Code Gene: {} ==========\n    // {}\n{}",
                non_test_code,
                short_id,
                gene.optimization.chars().take(60).collect::<String>(),
                test_funcs)
        } else {
            format!("{}\n\n// ========== Code Gene: {} ==========\n// {}\n{}",
                existing_content.trim_end(),
                short_id,
                gene.optimization.chars().take(60).collect::<String>(),
                unescaped_snippet)
        };

        // Write updated content (only if we have non-test code)
        if !unescaped_snippet.contains("#[cfg(test)]") {
            let new_content = format!("{}\n\n// ========== Code Gene: {} ==========\n// {}\n{}\n",
                existing_content.trim_end(),
                short_id,
                gene.optimization.chars().take(60).collect::<String>(),
                unescaped_snippet);

            if let Err(e) = std::fs::write(&lib_rs_path, new_content) {
                eprintln!("  ❌ Failed to write to {}: {}", lib_rs_path, e);
                errors += 1;
                continue;
            }
        }

        // Stage and commit
        let add_output = std::process::Command::new("git")
            .args(&["add", &lib_rs_path])
            .output()?;

        if !add_output.status.success() {
            eprintln!("  ❌ Failed to stage changes");
            errors += 1;
            continue;
        }

        let commit_msg = format!("feat({}): implement code gene {} - {}",
            crate_name,
            short_id,
            topic.replace("\"", "\\\""));
        let commit_output = std::process::Command::new("git")
            .args(&["commit", "-m", &commit_msg])
            .output()?;

        if !commit_output.status.success() {
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            eprintln!("  ❌ Failed to commit: {}", stderr.trim());
            errors += 1;
            continue;
        }

        // Push branch
        let push_output = std::process::Command::new("git")
            .args(&["push", "-u", "origin", &branch_name])
            .output()?;

        if !push_output.status.success() {
            let stderr = String::from_utf8_lossy(&push_output.stderr);
            eprintln!("  ❌ Failed to push branch: {}", stderr.trim());
            errors += 1;
            continue;
        }

        // Create PR
        let pr_title = format!("[code-gene] {}: {}", crate_name, topic);
        let pr_body = format!(
            r#"## Code Gene Implementation

**Gene ID:** `{}`
**Crate:** `{}`
**Gap Type:** `{}`

### Optimization

{}

### Code

```rust
{}
```

---

_This PR was auto-generated by the code-gene workflow._"#,
            gene.capsule_id,
            gene.target_crate,
            gene.gap_type,
            gene.optimization,
            gene.code_snippet.trim()
        );

        let pr_output = std::process::Command::new("gh")
            .args(&["pr", "create", "--repo", repo, "--title", &pr_title, "--body", &pr_body])
            .output()?;

        if pr_output.status.success() {
            let stdout = String::from_utf8_lossy(&pr_output.stdout);
            println!("  ✅ Created PR: {}", stdout.trim());
            created += 1;
        } else {
            let stderr = String::from_utf8_lossy(&pr_output.stderr);
            eprintln!("  ❌ Failed to create PR: {}", stderr.trim());
            errors += 1;
        }

        // Switch back to main
        let _ = std::process::Command::new("git")
            .args(&["checkout", "main"])
            .output()?;
    }

    println!("\n{} PRs created, {} errors", created, errors);
    Ok(())
}

pub fn handle_code_gene_plan(
    ids: &str,
    repo: &str,
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

    println!("Creating implementation plans (draft PRs) for {} genes...\n", genes.len());

    let mut created = 0;
    let mut errors = 0;

    for gene in &genes {
        let short_id = &gene.capsule_id[..8.min(gene.capsule_id.len())];
        let branch_name = format!("plan/code-gene/{}", short_id);

        let topic = if gene.trigger_topic.is_empty() {
            gene.optimization.chars().take(50).collect::<String>()
        } else {
            gene.trigger_topic.chars().take(50).collect::<String>()
        };
        let crate_name = gene.target_crate.split(':').next().unwrap_or(&gene.target_crate);

        println!("  Processing gene {}...", short_id);

        // Create branch for plan
        let branch_output = std::process::Command::new("git")
            .args(&["checkout", "-b", &branch_name])
            .output()?;

        if !branch_output.status.success() {
            let switch_output = std::process::Command::new("git")
                .args(&["checkout", &branch_name])
                .output()?;

            if !switch_output.status.success() {
                let stderr = String::from_utf8_lossy(&branch_output.stderr);
                eprintln!("  ❌ Failed to create/switch to branch {}: {}", branch_name, stderr.trim());
                errors += 1;
                continue;
            }
        }

        // Create plan document
        let plan_content = format!(
            r#"# Code Gene Implementation Plan

## Gene Information

| Field | Value |
|-------|-------|
| **ID** | `{}` |
| **Crate** | `{}` |
| **Gap Type** | `{}` |
| **Score** | {:.2} |
| **Feedback Count** | {} |

## Optimization

{}

## Implementation Plan

### 1. Code Analysis
- [ ] Analyze code snippet for dependencies
- [ ] Identify required imports
- [ ] Check for trait bounds

### 2. Target Selection
- [ ] Confirm target crate: `{}`
- [ ] Identify insertion point in lib.rs

### 3. Implementation
- [ ] Implement core logic
- [ ] Add necessary tests
- [ ] Handle edge cases

### 4. Verification
- [ ] Compile successfully
- [ ] Run tests
- [ ] Verify output

## Code Snippet

```rust
{}
```

## Notes

_Plan created by code-gene workflow. This is a DRAFT - do not merge until approved._
"#,
            gene.capsule_id,
            gene.target_crate,
            gene.gap_type,
            gene.outcome_success_score,
            gene.feedback_count,
            gene.optimization,
            gene.target_crate,
            gene.code_snippet.trim()
        );

        // Write plan to a file
        let plan_path = format!("plans/{}.md", short_id);
        if let Err(e) = std::fs::create_dir_all("plans") {
            eprintln!("  ⚠️  Failed to create plans dir: {}", e);
        }
        if let Err(e) = std::fs::write(&plan_path, plan_content.as_bytes()) {
            eprintln!("  ❌ Failed to write plan: {}", e);
            errors += 1;
            continue;
        }

        // Stage and commit
        let add_output = std::process::Command::new("git")
            .args(&["add", &plan_path])
            .output()?;

        if !add_output.status.success() {
            eprintln!("  ❌ Failed to stage plan");
            errors += 1;
            continue;
        }

        let commit_msg = format!("plan: code gene {} - {}",
            short_id,
            topic.replace("\"", "\\\""));
        let commit_output = std::process::Command::new("git")
            .args(&["commit", "-m", &commit_msg])
            .output()?;

        if !commit_output.status.success() {
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            eprintln!("  ❌ Failed to commit: {}", stderr.trim());
            errors += 1;
            continue;
        }

        // Push branch
        let push_output = std::process::Command::new("git")
            .args(&["push", "-u", "origin", &branch_name])
            .output()?;

        if !push_output.status.success() {
            let stderr = String::from_utf8_lossy(&push_output.stderr);
            eprintln!("  ❌ Failed to push branch: {}", stderr.trim());
            errors += 1;
            continue;
        }

        // Create draft PR
        let pr_title = format!("[plan] code-gene {}: {}", short_id, topic);
        let pr_body = format!(
            r#"## Code Gene Implementation Plan

**Gene ID:** `{}`
**Status:** DRAFT - Pending Review

### Review Checklist

- [ ] Code is correct and efficient
- [ ] No duplicate implementation exists
- [ ] Tests are appropriate
- [ ] No breaking changes

### Actions Required

1. Review the plan document
2. Approve or request changes
3. Once approved, run: `rairos code-gene-approve --ids {} --repo {}`

---

_This is an automated plan PR for code review._"#,
            gene.capsule_id,
            gene.capsule_id,
            repo
        );

        let pr_output = std::process::Command::new("gh")
            .args(&["pr", "create", "--repo", repo, "--title", &pr_title, "--body", &pr_body, "--draft"])
            .output()?;

        if pr_output.status.success() {
            let stdout = String::from_utf8_lossy(&pr_output.stdout);
            println!("  ✅ Created draft PR: {}", stdout.trim());
            created += 1;
        } else {
            let stderr = String::from_utf8_lossy(&pr_output.stderr);
            eprintln!("  ❌ Failed to create PR: {}", stderr.trim());
            errors += 1;
        }

        // Switch back to main
        let _ = std::process::Command::new("git")
            .args(&["checkout", "main"])
            .output()?;
    }

    println!("\n{} draft PRs created, {} errors", created, errors);
    println!("\n📋 Next steps:");
    println!("  1. Review the draft PRs");
    println!("  2. Request changes or approve");
    println!("  3. Run: raios code-gene-approve --ids <id> --repo {}", repo);
    Ok(())
}

pub fn handle_code_gene_approve(
    ids: &str,
    repo: &str,
) -> Result<()> {
    let all_genes = get_top_code_candidates(1000);

    let id_list: Vec<&str> = ids.split(',').map(|s| s.trim()).collect();
    let genes: Vec<CodeCapsuleGene> = all_genes
        .into_iter()
        .filter(|g| id_list.iter().any(|id| g.capsule_id.starts_with(id)))
        .collect();

    if genes.is_empty() {
        println!("No code genes match the IDs: {}", ids);
        return Ok(());
    }

    println!("Approving and implementing {} genes...\n", genes.len());

    let mut implemented = 0;
    let mut errors = 0;

    for gene in &genes {
        let short_id = &gene.capsule_id[..8.min(gene.capsule_id.len())];
        let branch_name = format!("code-gene/{}", short_id);

        let topic = if gene.trigger_topic.is_empty() {
            gene.optimization.chars().take(50).collect::<String>()
        } else {
            gene.trigger_topic.chars().take(50).collect::<String>()
        };
        let crate_name = gene.target_crate.split(':').next().unwrap_or(&gene.target_crate);

        println!("  Implementing gene {}...", short_id);

        // Create implementation branch
        let branch_output = std::process::Command::new("git")
            .args(&["checkout", "-b", &branch_name])
            .output()?;

        if !branch_output.status.success() {
            let switch_output = std::process::Command::new("git")
                .args(&["checkout", &branch_name])
                .output()?;

            if !switch_output.status.success() {
                let stderr = String::from_utf8_lossy(&branch_output.stderr);
                eprintln!("  ❌ Failed to create/switch to branch: {}", stderr.trim());
                errors += 1;
                continue;
            }
        }

        // Add code to lib.rs
        let lib_rs_path = format!("crates/{}/src/lib.rs", crate_name);
        let existing_content = match std::fs::read_to_string(&lib_rs_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  ❌ Failed to read {}: {}", lib_rs_path, e);
                errors += 1;
                continue;
            }
        };

        // Unescape code snippet
        let unescaped_snippet = gene.code_snippet
            .replace("\\\"", "\"")
            .replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("\\r", "\r");

        // Handle test module if present
        let new_content = if unescaped_snippet.contains("#[cfg(test)]") && unescaped_snippet.contains("mod tests {") {
            let cfg_test_pos = unescaped_snippet.find("#[cfg(test)]").unwrap_or(usize::MAX);
            let non_test_code = unescaped_snippet[..cfg_test_pos].trim();

            let test_module_start = unescaped_snippet[cfg_test_pos..].find("mod tests {").unwrap_or(usize::MAX);
            let after_mod = &unescaped_snippet[cfg_test_pos..][test_module_start + "mod tests {".len()..];
            let last_brace = after_mod.rfind('}').unwrap_or(usize::MAX);
            let test_inner = &after_mod[..last_brace];

            let test_funcs = test_inner.lines()
                .filter(|l| !l.trim().is_empty() && !l.trim().starts_with("use super::*;"))
                .collect::<Vec<_>>()
                .join("\n");

            format!("{}\n\n// ========== Code Gene: {} ==========\n// {}\n{}\n\n// ========== Test: {} ==========\n    // {}\n{}",
                existing_content.trim_end(),
                short_id,
                gene.optimization.chars().take(60).collect::<String>(),
                non_test_code,
                short_id,
                gene.optimization.chars().take(60).collect::<String>(),
                test_funcs)
        } else {
            format!("{}\n\n// ========== Code Gene: {} ==========\n// {}\n{}\n",
                existing_content.trim_end(),
                short_id,
                gene.optimization.chars().take(60).collect::<String>(),
                unescaped_snippet)
        };

        // Write updated content
        if let Err(e) = std::fs::write(&lib_rs_path, new_content.as_bytes()) {
            eprintln!("  ❌ Failed to write to {}: {}", lib_rs_path, e);
            errors += 1;
            continue;
        }

        // Stage and commit
        let add_output = std::process::Command::new("git")
            .args(&["add", &lib_rs_path])
            .output()?;

        if !add_output.status.success() {
            eprintln!("  ❌ Failed to stage changes");
            errors += 1;
            continue;
        }

        let commit_msg = format!("feat({}): implement code gene {} - {}",
            crate_name,
            short_id,
            topic.replace("\"", "\\\""));
        let commit_output = std::process::Command::new("git")
            .args(&["commit", "-m", &commit_msg])
            .output()?;

        if !commit_output.status.success() {
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            eprintln!("  ❌ Failed to commit: {}", stderr.trim());
            errors += 1;
            continue;
        }

        // Push branch
        let push_output = std::process::Command::new("git")
            .args(&["push", "-u", "origin", &branch_name])
            .output()?;

        if !push_output.status.success() {
            let stderr = String::from_utf8_lossy(&push_output.stderr);
            eprintln!("  ❌ Failed to push branch: {}", stderr.trim());
            errors += 1;
            continue;
        }

        // Create PR
        let pr_title = format!("[code-gene] {}: {}", crate_name, topic);
        let pr_body = format!(
            r#"## Code Gene Implementation

**Gene ID:** `{}`
**Status:** Approved and Implemented

### Changes

- Added implementation to `{}`
- Added tests

### Code

```rust
{}
```

---

_This PR was auto-generated after plan approval._"#,
            gene.capsule_id,
            lib_rs_path,
            gene.code_snippet.trim()
        );

        let pr_output = std::process::Command::new("gh")
            .args(&["pr", "create", "--repo", repo, "--title", &pr_title, "--body", &pr_body])
            .output()?;

        if pr_output.status.success() {
            let stdout = String::from_utf8_lossy(&pr_output.stdout);
            println!("  ✅ Created PR: {}", stdout.trim());
            implemented += 1;

            // Update gene status to "implemented"
            let mut updated_gene = gene.clone();
            updated_gene.status = "implemented".to_string();
            if let Err(e) = save_code_capsule(&updated_gene) {
                eprintln!("  ⚠️  Failed to update gene status: {}", e);
            } else {
                println!("  ✅ Gene status updated to 'implemented'");
            }
        } else {
            let stderr = String::from_utf8_lossy(&pr_output.stderr);
            eprintln!("  ❌ Failed to create PR: {}", stderr.trim());
            errors += 1;
        }

        // Switch back to main
        let _ = std::process::Command::new("git")
            .args(&["checkout", "main"])
            .output()?;
    }

    println!("\n{} genes implemented, {} errors", implemented, errors);
    Ok(())
}

pub fn handle_code_gene_reject(
    ids: &str,
    repo: &str,
    reason: &str,
) -> Result<()> {
    let all_genes = get_top_code_candidates(1000);

    let id_list: Vec<&str> = ids.split(',').map(|s| s.trim()).collect();
    let genes: Vec<CodeCapsuleGene> = all_genes
        .into_iter()
        .filter(|g| id_list.iter().any(|id| g.capsule_id.starts_with(id)))
        .collect();

    if genes.is_empty() {
        println!("No code genes match the IDs: {}", ids);
        return Ok(());
    }

    println!("Rejecting {} genes...\n", genes.len());

    let mut rejected = 0;
    let mut errors = 0;

    for gene in &genes {
        let short_id = &gene.capsule_id[..8.min(gene.capsule_id.len())];
        let topic = gene.optimization.chars().take(50).collect::<String>();

        println!("  Rejecting gene {}...", short_id);

        // Find the draft PR for this gene
        let pr_list_output = std::process::Command::new("gh")
            .args(&["pr", "list", "--repo", repo, "--author", "kilo-code-bot", "--state", "open", "--json", "number,title,body"])
            .output()?;

        if !pr_list_output.status.success() {
            let stderr = String::from_utf8_lossy(&pr_list_output.stderr);
            eprintln!("  ❌ Failed to list PRs: {}", stderr.trim());
            errors += 1;
            continue;
        }

        let stdout = String::from_utf8_lossy(&pr_list_output.stdout);
        let prs: Vec<serde_json::Value> = match serde_json::from_str(&stdout) {
            Ok(v) => v,
            Err(_) => vec![],
        };

        // Find plan PR for this gene
        let plan_pr = prs.iter().find(|pr| {
            pr["title"].as_str().unwrap_or("").contains(&format!("plan] code-gene {}", short_id))
        });

        if let Some(pr) = plan_pr {
            let pr_number = pr["number"].as_i64().unwrap_or(0);

            // Add rejection comment
            let comment_body = format!(
                r#"## ❌ Plan Rejected

**Reason:** {}

**Gene ID:** `{}`

This plan requires changes before implementation. Please address the issues above and resubmit.

---
_Rejected by code-gene workflow_"#,
                reason,
                gene.capsule_id
            );

            let comment_output = std::process::Command::new("gh")
                .args(&["issue", "comment", &pr_number.to_string(), "--repo", repo, "--body", &comment_body])
                .output()?;

            // Close the PR
            let close_output = std::process::Command::new("gh")
                .args(&["pr", "close", &pr_number.to_string(), "--repo", repo])
                .output()?;

            if close_output.status.success() {
                println!("  ✅ Rejected and closed PR #{}", pr_number);

                // Update gene status back to 'rejected'
                let mut updated_gene = gene.clone();
                updated_gene.status = "rejected".to_string();
                if let Err(e) = save_code_capsule(&updated_gene) {
                    eprintln!("  ⚠️  Failed to update gene status: {}", e);
                }

                rejected += 1;
            } else {
                let stderr = String::from_utf8_lossy(&close_output.stderr);
                eprintln!("  ❌ Failed to close PR: {}", stderr.trim());
                errors += 1;
            }
        } else {
            println!("  ⚠️  No draft PR found for gene {}", short_id);

            // Just update the gene status
            let mut updated_gene = gene.clone();
            updated_gene.status = "rejected".to_string();
            if let Err(e) = save_code_capsule(&updated_gene) {
                eprintln!("  ⚠️  Failed to update gene status: {}", e);
            }
            rejected += 1;
        }
    }

    println!("\n{} genes rejected, {} errors", rejected, errors);
    Ok(())
}

pub fn handle_code_gene_auto_review(
    ids: &str,
    repo: &str,
    auto_approve: bool,
) -> Result<()> {
    let all_genes = get_all_code_capsules();

    // Filter for genes that have a plan (status is not "implemented", "rejected", etc.)
    let genes: Vec<CodeCapsuleGene> = {
        let filtered: Vec<CodeCapsuleGene> = all_genes
            .into_iter()
            .filter(|g| g.status != "implemented" && g.status != "rejected" && g.status != "low")
            .collect();

        if ids == "all" {
            filtered
        } else {
            let id_list: Vec<&str> = ids.split(',').map(|s| s.trim()).collect();
            filtered.into_iter().filter(|g| id_list.iter().any(|id| g.capsule_id.starts_with(id))).collect()
        }
    };

    if genes.is_empty() {
        println!("No code genes match the criteria: {}", ids);
        return Ok(());
    }

    println!("Auto-reviewing {} genes...\n", genes.len());

    // Get LLM credentials
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("LLM_API_KEY"))
        .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;
    let base_url = std::env::var("LLM_BASE_URL")
        .or_else(|_| std::env::var("OPENAI_BASE_URL"))
        .unwrap_or_else(|_| LLM_BASE_URL.to_string());
    let chat_model = std::env::var("LLM_MODEL")
        .unwrap_or_else(|_| LLM_MODEL.to_string());

    let mut reviewed = 0;
    let mut approved = 0;
    let mut rejected = 0;
    let mut errors = 0;

    for gene in &genes {
        let short_id = &gene.capsule_id[..8.min(gene.capsule_id.len())];

        // Find the plan PR
        let pr_list_output = std::process::Command::new("gh")
            .args(&["pr", "list", "--repo", repo, "--state", "open", "--json", "number,title,body"])
            .output()?;

        if !pr_list_output.status.success() {
            errors += 1;
            continue;
        }

        let stdout = String::from_utf8_lossy(&pr_list_output.stdout);
        let prs: Vec<serde_json::Value> = match serde_json::from_str(&stdout) {
            Ok(v) => v,
            Err(_) => vec![],
        };

        let plan_pr = prs.iter().find(|pr| {
            pr["title"].as_str().unwrap_or("").contains(&format!("plan] code-gene {}", short_id))
        });

        if plan_pr.is_none() {
            continue;
        }

        let pr = plan_pr.unwrap();
        let pr_number = pr["number"].as_i64().unwrap_or(0);
        let pr_body = pr["body"].as_str().unwrap_or("");

        println!("  Reviewing gene {} (PR #{})...", short_id, pr_number);

        // Build review prompt
        let review_prompt = format!(
            r#"You are reviewing a code implementation plan for a Rust crate.

## Gene Information
- ID: {}
- Crate: {}
- Gap Type: {}
- Optimization: {}

## Code Snippet
```rust
{}
```

## Review Checklist

Evaluate the code plan and respond with EXACTLY one of these formats:

**APPROVE:**
```
VERDICT: APPROVE
REASON: <brief reason>
```

**REJECT:**
```
VERDICT: REJECT
REASON: <specific issues and fixes needed>
```

Consider:
1. Is the code correct and idiomatic Rust?
2. Are trait bounds appropriate?
3. Is there duplicate code already in the codebase?
4. Are tests adequate?
5. Is the optimization likely to work?
"#,
            gene.capsule_id,
            gene.target_crate,
            gene.gap_type,
            gene.optimization,
            gene.code_snippet.trim()
        );

        // Call LLM for review
        let rt = tokio::runtime::Runtime::new()?;
        let review_result = rt.block_on(async {
            let client = rairos_llm::client_async::AsyncClient::new(
                api_key.clone(),
                base_url.clone(),
                chat_model.clone(),
            );
            let messages = vec![
                std::collections::HashMap::from([
                    ("role".to_string(), "user".to_string()),
                    ("content".to_string(), review_prompt.clone()),
                ]),
            ];
            client.chat_completions(messages, None, None, false).await
        });

        match review_result {
            Ok(review) => {
                println!("    Review result: {}", review.chars().take(100).collect::<String>());

                let verdict = if review.contains("VERDICT: APPROVE") {
                    "approve"
                } else if review.contains("VERDICT: REJECT") {
                    "reject"
                } else {
                    "unknown"
                };

                // Add review comment
                let comment_body = format!(
                    r#"## 🔍 AI Review

{}

---
_Auto-reviewed by code-gene workflow_"#,
                    review
                );

                let _ = std::process::Command::new("gh")
                    .args(&["pr", "comment", &pr_number.to_string(), "--repo", repo, "--body", &comment_body])
                    .output()?;

                if verdict == "approve" && auto_approve {
                    println!("    ✅ Auto-approving...");
                    // Update gene status to approved
                    let mut updated_gene = gene.clone();
                    updated_gene.status = "approved".to_string();
                    let _ = save_code_capsule(&updated_gene);
                    approved += 1;
                } else if verdict == "reject" {
                    println!("    ❌ Rejecting...");
                    // Update gene status to needs_revision
                    let mut updated_gene = gene.clone();
                    updated_gene.status = "needs_revision".to_string();
                    let _ = save_code_capsule(&updated_gene);

                    // Close the PR with comment
                    let reject_comment = format!(
                        r#"## ❌ Plan Needs Revision

The AI review found issues with this plan:

{}

Please address the issues and resubmit.

---
_Rejected by code-gene auto-review_"#,
                        review
                    );
                    let _ = std::process::Command::new("gh")
                        .args(&["pr", "comment", &pr_number.to_string(), "--repo", repo, "--body", &reject_comment])
                        .output()?;
                    let _ = std::process::Command::new("gh")
                        .args(&["pr", "close", &pr_number.to_string(), "--repo", repo])
                        .output()?;
                    rejected += 1;
                }

                reviewed += 1;
            }
            Err(e) => {
                eprintln!("    ❌ LLM review failed: {}", e);
                errors += 1;
            }
        }
    }

    println!("\n📊 Auto-review complete:");
    println!("  Reviewed: {}", reviewed);
    println!("  Approved: {}", approved);
    println!("  Rejected: {}", rejected);
    println!("  Errors: {}", errors);

    if auto_approve && approved > 0 {
        println!("\n📋 Next step: Run `rairos code-gene-approve --ids <ids> --repo {}`", repo);
    }

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

pub fn handle_code_gene_implement(
    issue_number: usize,
    repo: &str,
    execute: bool,
) -> Result<()> {
    println!("\n{}", "═".repeat(60));
    println!("📋 Code Gene Implementation Workflow");
    println!("{}", "═".repeat(60));
    println!("  Issue: #{}", issue_number);
    println!("  Repo: {}", repo);
    println!("  Mode: {}", if execute { "EXECUTE" } else { "DRY-RUN (preview only)" });
    println!();

    // Step 1: Fetch issue from GitHub
    println!("Step 1/6: Fetching issue from GitHub...");
    let output = std::process::Command::new("gh")
        .args(&["issue", "view", &format!("{}", issue_number), "--repo", repo, "--json", "number,title,body,labels,state"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to fetch issue #{}", issue_number);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let issue: serde_json::Value = serde_json::from_str(&stdout)
        .context("Failed to parse issue JSON")?;

    let title = issue["title"].as_str().unwrap_or("");
    let body = issue["body"].as_str().unwrap_or("");
    let labels: Vec<String> = issue["labels"].as_array()
        .map(|arr| arr.iter().filter_map(|l| l["name"].as_str().map(String::from)).collect())
        .unwrap_or_default();

    println!("  ✅ Fetched: {}", title);

    // Step 2: Parse gene ID and optimization from issue body
    println!("\nStep 2/6: Parsing gene information...");
    let gene_id = body.lines()
        .find(|l| l.starts_with("**ID:**"))
        .and_then(|l| l.split("`").nth(1))
        .map(|s| s.to_string());

    let optimization = extract_section(body, "## Optimization", "## Code")
        .unwrap_or_else(|| "Unknown optimization".to_string());

    let code_snippet = extract_github_code_block(body);

    if let Some(ref id) = gene_id {
        println!("  ✅ Gene ID: {}", id);
    }
    println!("  ✅ Optimization: {}...", optimization.chars().take(50).collect::<String>());
    println!("  ✅ Code snippet: {} chars", code_snippet.len());

    // Step 3: Search existing code to prevent duplicates
    println!("\nStep 3/6: Searching for existing code (duplicate check)...");
    let crate_label = labels.iter().find(|l| l.starts_with("crate-")).cloned();

    // Extract function names and identifiers from code snippet for search
    // Filter out common function names that cause false positives
    let common_fn_names = ["new", "get", "insert", "set", "remove", "clear", "push", "pop", "len", "is_empty", "iter", "into_iter", "map", "filter", "fold"];
    let search_terms: Vec<String> = code_snippet.lines()
        .filter(|l| !l.trim().starts_with("//") && !l.trim().is_empty())
        .filter_map(|l| {
            // Look for function names, struct names
            if l.contains("fn ") {
                l.split("fn ").nth(1)?.split('(').next().map(|s| s.trim().to_string())
            } else if l.contains("pub struct") {
                l.split("pub struct ").nth(1).map(|s| s.split_whitespace().next().unwrap_or("").to_string())
            } else if l.contains("struct ") {
                l.split("struct ").nth(1).map(|s| s.split_whitespace().next().unwrap_or("").to_string())
            } else {
                None
            }
        })
        .filter_map(|s| {
            // Strip generic type parameters for matching (e.g., "LruRankerCache<K," -> "LruRankerCache")
            let stripped = s.split('<').next().unwrap_or(&s).trim().to_string();
            if stripped.len() > 3 && !stripped.contains('{') && !common_fn_names.contains(&stripped.as_str()) {
                Some(stripped)
            } else {
                None
            }
        })
        .take(5)
        .collect();

    println!("  🔍 Searching for: {:?}", search_terms);

    let existing_code = if let Some(ref crate_name) = crate_label {
        let crate_path = format!("crates/{}/src", crate_name.replace("crate-", ""));
        let mut results = Vec::new();
        for term in &search_terms {
            let r = search_code_in_path(term, &crate_path);
            results.extend(r);
        }
        results
    } else {
        let mut results = Vec::new();
        for term in &search_terms {
            let r = search_code_in_path(term, "crates");
            results.extend(r);
        }
        results
    };

    if !existing_code.is_empty() {
        println!("  ⚠️  Found existing code - NO DUPLICATE IMPLEMENTATION:");
        for (path, line) in existing_code.iter().take(3) {
            println!("    - {}: {}", path, line.chars().take(60).collect::<String>());
        }
    } else {
        println!("  ✅ No existing code found - safe to implement");
    }

    // Step 4: Post implementation plan as comment
    println!("\nStep 4/6: Posting implementation plan to issue...");

    let plan = if existing_code.is_empty() {
        format!("## Implementation Plan\n\n### Status: Ready to Implement ✅\n\n**Gene ID:** `{}`\n**Search Results:** No existing code found - safe to implement.\n\n### Implementation Steps\n\n1. [ ] Create implementation in target crate\n2. [ ] Add tests\n3. [ ] Run tests to verify\n4. [ ] Mark issue as `Implemented`\n5. [ ] Update gene feedback\n\n### Code to Implement\n\n```rust\n{}\n```\n\n---\n*Workflow: Search → Plan → Confirm → Implement*", gene_id.as_deref().unwrap_or("N/A"), code_snippet)
    } else {
        format!("## Implementation Plan\n\n### Status: Already Implemented ✅\n\n**Gene ID:** `{}`\n**Search Results:** Found existing code - no duplicate needed.\n\n**Existing Code Locations:**\n{}\n\n### Verification Steps\n\n1. [ ] Verify existing implementation is complete\n2. [ ] Run tests to confirm\n3. [ ] Mark issue as `Implemented`\n4. [ ] Update gene feedback\n\n---\n*Workflow: Search → Plan → Confirm → Implement*",
            gene_id.as_deref().unwrap_or("N/A"),
            existing_code.iter().take(3).map(|(p, l)| format!("- {}: {}", p, l.chars().take(60).collect::<String>())).collect::<Vec<_>>().join("\n"))
    };

    if execute {
        let output = std::process::Command::new("gh")
            .args(&["issue", "comment", &format!("{}", issue_number), "--repo", repo, "--body", &plan])
            .output()?;

        if output.status.success() {
            println!("  ✅ Posted plan to issue #{}", issue_number);
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("  ❌ Failed to post comment: {}", stderr.trim());
        }
    } else {
        println!("  📝 DRY-RUN: Would post this plan:");
        println!("{}", "─".repeat(60));
        for line in plan.lines().take(20) {
            println!("  {}", line);
        }
        if plan.lines().count() > 20 {
            println!("  ... ({} more lines)", plan.lines().count() - 20);
        }
        println!("{}", "─".repeat(60));
    }

    // Step 5: Execute if --execute is set
    println!("\nStep 5/6: {}", if execute { "Implementing..." } else { "Skipping implementation (dry-run)" });

    let mut implementation_succeeded = false;

    if execute && existing_code.is_empty() {
        if let Some(ref crate_name) = crate_label {
            let crate_src_path = format!("crates/{}/src", crate_name.replace("crate-", ""));
            let lib_rs_path = format!("{}/lib.rs", crate_src_path);

            println!("  📝 Implementing code in {}...", lib_rs_path);

            // Read existing file
            let existing_content = std::fs::read_to_string(&lib_rs_path)
                .context(format!("Failed to read {}", lib_rs_path))?;

            // Append code snippet with separator
            let separator = "\n\n// ========== Code Gene Implementation ==========\n";
            // Unescape common escape sequences from GitHub markdown
            let unescaped_snippet = code_snippet
                .replace("\\\"", "\"")
                .replace("\\n", "\n")
                .replace("\\t", "\t")
                .replace("\\r", "\r");

            // Handle test module: if code has both regular code and a #[cfg(test)] mod tests { ... },
            // we need to extract properly
            let code_to_append = if unescaped_snippet.contains("#[cfg(test)]") && unescaped_snippet.contains("mod tests {") {
                // Split into non-test code (before #[cfg(test)]) and test code
                let cfg_test_pos = unescaped_snippet.find("#[cfg(test)]").unwrap_or(usize::MAX);

                // Non-test code (before #[cfg(test)])
                let non_test_code = unescaped_snippet[..cfg_test_pos].trim();

                // Test module content - extract inner functions
                let test_module_start = unescaped_snippet[cfg_test_pos..].find("mod tests {").unwrap_or(usize::MAX);
                let after_mod = &unescaped_snippet[cfg_test_pos..][test_module_start + "mod tests {".len()..];
                let last_brace = after_mod.rfind('}').unwrap_or(usize::MAX);
                let test_inner = &after_mod[..last_brace];

                // Remove use super::*; and format for existing tests module
                let test_funcs = test_inner.lines()
                    .filter(|l| !l.trim().is_empty() && !l.trim().starts_with("use super::*;"))
                    .collect::<Vec<_>>()
                    .join("\n");

                format!("{}\n\n// ========== Test Code ==========\n    // From code gene test: {}\n{}",
                    non_test_code,
                    gene_id.as_deref().unwrap_or("unknown"),
                    test_funcs)
            } else {
                unescaped_snippet
            };

            let new_content = format!("{}{}{}\n", existing_content.trim_end(), separator, code_to_append);

            // Write updated content
            std::fs::write(&lib_rs_path, new_content)
                .context(format!("Failed to write to {}", lib_rs_path))?;

            println!("  ✅ Written code to {}", lib_rs_path);

            // Run tests
            println!("  🧪 Running tests...");
            let test_output = std::process::Command::new("cargo")
                .args(&["test", "-p", &crate_name.replace("crate-", "")])
                .current_dir(std::env::current_dir()?)
                .output()?;

            implementation_succeeded = test_output.status.success();
            if implementation_succeeded {
                println!("  ✅ Tests passed!");

                // Update gene status to "implemented"
                if let Some(ref gid) = gene_id {
                    if let Some(gene) = get_all_code_capsules().iter().find(|g| g.capsule_id == *gid) {
                        let mut updated_gene = gene.clone();
                        updated_gene.status = "implemented".to_string();
                        if let Err(e) = save_code_capsule(&updated_gene) {
                            eprintln!("  ⚠️  Failed to update gene status: {}", e);
                        } else {
                            println!("  ✅ Gene status updated to 'implemented'");
                        }
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&test_output.stderr);
                println!("  ⚠️  Tests failed:");
                for line in stderr.lines().take(10) {
                    println!("    {}", line);
                }
            }
        } else {
            println!("  ⚠️  No crate label found - cannot implement");
        }
    } else if existing_code.is_empty() {
        println!("  ℹ️  Run with --execute to actually implement");
    }

    // Step 6: Close issue if --execute, implementation succeeded, and no duplicate
    if execute {
        if existing_code.is_empty() {
            // New implementation - only close if tests passed
            if implementation_succeeded {
                println!("\nStep 6/6: Closing issue...");
                let close_output = std::process::Command::new("gh")
                    .args(&["issue", "close", &format!("{}", issue_number), "--repo", repo, "--reason", "completed"])
                    .output()?;

                if close_output.status.success() {
                    println!("  ✅ Closed issue #{}", issue_number);
                } else {
                    let stderr = String::from_utf8_lossy(&close_output.stderr);
                    eprintln!("  ⚠️  Could not close issue: {}", stderr.trim());
                }
            } else {
                println!("\nStep 6/6: Keeping issue open (tests failed)");
                println!("  ⚠️  Issue #{} remains open - fix compilation/test errors", issue_number);
            }
        } else {
            // Existing code found - close as completed
            println!("\nStep 6/6: Closing issue (duplicate detection)...");
            let close_output = std::process::Command::new("gh")
                .args(&["issue", "close", &format!("{}", issue_number), "--repo", repo, "--reason", "completed"])
                .output()?;

            if close_output.status.success() {
                println!("  ✅ Closed issue #{}", issue_number);
            } else {
                let stderr = String::from_utf8_lossy(&close_output.stderr);
                eprintln!("  ⚠️  Could not close issue: {}", stderr.trim());
            }
        }
    }

    println!("\n{}", "═".repeat(60));
    if execute {
        println!("✅ Workflow complete!");
    } else {
        println!("📋 Dry-run complete! Use --execute to implement.");
    }
    println!("{}", "═".repeat(60));

    Ok(())
}

fn search_code_in_path(term: &str, path: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();

    if let Ok(output) = std::process::Command::new("grep")
        .args(&["-r", "-n", "-i", term, path, "--include=*.rs"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().take(10) {
                if let Some((path_part, code_part)) = line.split_once(':') {
                    results.push((path_part.to_string(), code_part.to_string()));
                }
            }
        }
    }
    results
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
