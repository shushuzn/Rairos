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

pub fn handle_citations(
    db: &Database,
    from: Option<&str>,
    to: Option<&str>,
    format: &str,
) -> Result<()> {
    if from.is_none() && to.is_none() {
        eprintln!("Error: must specify --from or --to");
        std::process::exit(1);
    }

    match (from, to) {
        (Some(f), Some(t)) => {
            let from_title = db.get_paper(f)?.title;
            let to_title = db.get_paper(t)?.title;

            let citations_from = db.get_citations(f)?;
            let citations_to = db.get_citations(t)?;

            let direct = citations_from.references.contains(&t.to_string());
            let citing_to_sources: std::collections::HashSet<String> =
                citations_to.citing.into_iter().collect();
            let via_papers: Vec<&String> = citations_from
                .references
                .iter()
                .filter(|id| citing_to_sources.contains(*id))
                .collect();

            if format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "from": f,
                        "from_title": from_title,
                        "to": t,
                        "to_title": to_title,
                        "direct": direct,
                        "via_papers": via_papers,
                    }))?
                );
                return Ok(());
            }

            println!("Citation Bridge — {} ↔ {}", f, t);
            println!("  From: {}", from_title.chars().take(60).collect::<String>());
            println!("  To:   {}", to_title.chars().take(60).collect::<String>());
            if direct {
                println!("  ✅ DIRECT: {} cites {}", f, t);
            }
            if !via_papers.is_empty() {
                println!("  ⚡ INDIRECT ({} connections):", via_papers.len());
                for v in &via_papers {
                    println!("    {} → {} → {}", f, v, t);
                }
            }
            if !direct && via_papers.is_empty() {
                println!("  No citation path found between these papers");
            }
        }
        (Some(pid), None) | (None, Some(pid)) => {
            let direction = if from.is_some() { "from" } else { "to" };
            let paper_result = db.get_paper(pid);
            let title = paper_result
                .as_ref()
                .map(|p| p.title.clone())
                .unwrap_or_else(|_| "?".to_string());

            let citations = db.get_citations(pid)?;

            let ids: Vec<String> = if direction == "from" {
                citations.references
            } else {
                citations.citing
            };

            if format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "paper_id": pid,
                        "direction": direction,
                        "count": ids.len(),
                        "citations": ids,
                    }))?
                );
                return Ok(());
            }

            let label = if direction == "from" {
                "References"
            } else {
                "Cited by"
            };

            println!("{} — {}", label, pid);
            println!("  {}", title.chars().take(60).collect::<String>());
            if ids.is_empty() {
                println!("  No citations found");
            } else {
                println!("  Found {} citation(s):", ids.len());
                for cid in &ids {
                    println!("    {}", cid);
                }
            }
        }
        (None, None) => unreachable!(),
    }
    Ok(())
}

pub fn severity_icon(severity: &str) -> &'static str {
    match severity.to_lowercase().as_str() {
        "high" => "🔴",
        "medium" => "🟡",
        "low" => "🟢",
        _ => "⚪",
    }
}

pub fn handle_influence(
    db: &Database,
    top: usize,
    paper: Option<&str>,
    min_cites: usize,
    format: &str,
) -> Result<()> {
    if let Some(paper_id) = paper {
        let pap = db.get_paper(paper_id)?;
        let citations = db.get_citations(paper_id)?;
        let forward = citations.citing.len() as f64;
        let backward = citations.references.len() as f64;

        let year = pap.published.year();
        let age = if year > 2000 && year <= 2026 {
            (2026 - year + 1) as f64
        } else {
            0.0
        };
        let velocity = if age > 0.0 { forward / age } else { 0.0 };

        let impact = if velocity >= 10.0 {
            "\u{1f525} Extremely high velocity (\u{2265}10/y) — field-defining"
        } else if velocity >= 5.0 {
            "\u{1f4c8} High velocity (5-10/y) — very active research"
        } else if velocity >= 1.0 {
            "\u{1f4ca} Moderate velocity (1-5/y) — steady influence"
        } else {
            "\u{1f4c9} Low velocity — emerging or niche"
        };

        println!("=== Paper Influence Profile ===");
        println!("  Paper ID  : {}", paper_id);
        println!("  Title     : {}", pap.title);
        println!("  Published : {}", year);
        if age > 0.0 {
            println!("  Age       : {:.0} years (as of 2026)", age);
        }
        println!();
        println!("  Citations");
        if age > 0.0 {
            println!(
                "    Cited by (forward) : {:.0}  → velocity = {:.0}/{:.0} = {:.2}/y",
                forward, forward, age, velocity
            );
        } else {
            println!("    Cited by (forward) : {:.0}", forward);
        }
        println!("    References (backward): {:.0}", backward);
        println!();
        if age > 0.0 {
            println!("  Impact Assessment");
            println!("    {}", impact);
        }
        return Ok(());
    }

    let all_papers = db.list_papers(None, 100000, 0)?;
    let mut results: Vec<(String, String, i32, f64, f64)> = Vec::new();

    for p in &all_papers {
        if p.metadata.cited_by == 0 && min_cites > 0 {
            continue;
        }
        let forward = p.metadata.cited_by as f64;
        if forward < min_cites as f64 {
            continue;
        }
        let year = p.published.year();
        if year < 2000 || year > 2026 {
            continue;
        }
        let age = (2026 - year + 1) as f64;
        let velocity = forward / age;
        results.push((p.id.clone(), p.title.clone(), year, forward, velocity));
    }

    results.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));

    if results.is_empty() {
        println!("No papers with sufficient citation data found.");
        return Ok(());
    }

    let top_n: Vec<_> = results.iter().take(top).collect();

    match format {
        "json" => {
            let data: Vec<serde_json::Value> = top_n
                .iter()
                .enumerate()
                .map(|(i, (id, title, year, forward, vel))| {
                    serde_json::json!({
                        "rank": i + 1,
                        "paper_id": id,
                        "title": title,
                        "year": year,
                        "forward_cites": forward,
                        "age_years": (2026 - year + 1) as f64,
                        "velocity": (vel * 100.0).round() / 100.0,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
        "csv" => {
            println!("rank,paper_id,title,year,forward_cites,age_years,velocity");
            for (i, (id, title, year, forward, vel)) in top_n.iter().enumerate() {
                let title_esc = title.replace('"', "\"\"");
                println!(
                    "{},\"{}\",\"{}\",{},{},{:.1},{:.2}",
                    i + 1,
                    id,
                    title_esc,
                    year,
                    forward,
                    2026 - year + 1,
                    vel
                );
            }
        }
        _ => {
            println!(
                "{:>4}  {:>8}  {:>5}  {:>3}y  Year  Paper",
                "Rank", "Velocity", "Cites", "Age"
            );
            println!("{}", "-".repeat(50));
            for (i, (_id, title, year, forward, vel)) in top_n.iter().enumerate() {
                let title_short = if title.len() > 50 {
                    format!("{}…", &title[..50])
                } else {
                    title.clone()
                };
                println!(
                    "{:>4}  {:>7.1}/y  {:>5.0}  {:>3.0}   {}  {}",
                    i + 1,
                    vel,
                    forward,
                    2026 - year + 1,
                    year,
                    title_short
                );
            }
            println!();
            println!(
                "Showing {} of {} papers with >= {} citation(s)",
                top_n.len(),
                results.len(),
                min_cites
            );
            println!("Formula: velocity = forward_citations / age_years  (age = 2026 - published + 1)");
        }
    }
    Ok(())
}

pub fn handle_citation_chain(
    db: &Database,
    paper_id: Option<&str>,
    depth: i32,
    graphviz: bool,
    mermaid: bool,
    influencers: bool,
    impact: bool,
    path: Option<&str>,
) -> Result<()> {
    let mut builder = rairos_citation_chain::CitationChainBuilder::new();

    if influencers || impact {
        let Some(pid) = paper_id else {
            eprintln!("Usage: citation-chain <paper_id> --influencers|--impact");
            return Ok(());
        };

        if influencers {
            println!("Finding influences for: {}", pid);
            if let Ok(papers) = db.search_papers_smart(pid, 1) {
                if let Some(p) = papers.first() {
                builder.add_paper(pid.to_string(), p.title.clone(), p.published.year(), Vec::new(), Vec::new(), String::new(), 0);
            }
        }
        println!("Influencers: (requires citations data in DB)");
    }

    if impact {
        println!("Finding impact for: {}", pid);
        println!("Impact: (requires citations data in DB)");
        }

        return Ok(());
    }

    let Some(pid) = paper_id else {
        eprintln!("Usage: citation-chain <paper_id> [options]");
        return Ok(());
    };

    if let Ok(papers) = db.search_papers_smart(pid, 5) {
        for p in &papers {
            builder.add_paper(p.id.clone(), p.title.clone(), p.published.year(), Vec::new(), Vec::new(), String::new(), 0);
        }
    }

    let chain = builder.build_from_db(pid, depth);

    if graphviz {
        println!("{}", builder.render_graphviz(&chain));
    } else if mermaid {
        println!("{}", builder.render_mermaid(&chain));
    } else {
        println!("{}", builder.render_text(&chain, 20));
    }

    if let Some(_target) = path {
        println!("Path finding requires citation graph data in DB.");
    }

    Ok(())
}
