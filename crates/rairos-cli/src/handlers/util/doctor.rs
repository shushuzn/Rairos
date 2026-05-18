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

pub fn handle_doctor(format: &str) -> Result<()> {
    use std::env;
    use std::path::Path;

    let mut ok: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let issues: Vec<String> = Vec::new();

    let ver = env!("CARGO_PKG_VERSION");
    ok.push(format!("rairos-cli {}", ver));

    #[cfg(windows)]
    ok.push("Platform: Windows (MSVC)".to_string());
    #[cfg(not(windows))]
    ok.push("Platform: non-Windows".to_string());

    let db_paths = [
        Path::new("rairos.db"),
        Path::new("research.db"),
        Path::new(".ai_research_os/research.db"),
    ];
    let found_db = db_paths.iter().find(|p| p.exists());
    if let Some(db_path) = found_db {
        ok.push(format!("Database: {} exists", db_path.display()));
        if let Ok(db) = Database::open(db_path) {
            if let Ok(stats) = db.stats() {
                ok.push(format!("  {} papers, {} gaps", stats.total, stats.gaps));
            }
        }
    } else {
        warnings.push("No database found (run 'rairos init')".to_string());
    }

    for name in &[".env", ".env.example"] {
        if Path::new(name).exists() {
            ok.push(format!("{}: exists", name));
        } else {
            warnings.push(format!("{}: not found", name));
        }
    }

    if Path::new(".git").exists() {
        ok.push("Git repository: yes".to_string());
    } else {
        warnings.push("Not a git repository".to_string());
    }

    match reqwest::blocking::get("http://127.0.0.1:8080/health") {
        Ok(resp) if resp.status().is_success() => {
            ok.push("API daemon: reachable".to_string());
        }
        Ok(resp) => {
            warnings.push(format!("API daemon: returned {}", resp.status()));
        }
        Err(_e) => {
            warnings.push(
                "API daemon: not reachable (run 'rairos daemon --foreground' to start)".to_string(),
            );
        }
    }

    ok.push(format!("Rust: {}", env::consts::ARCH));

    if format == "json" {
        let out = serde_json::json!({
            "ok": ok,
            "warnings": warnings,
            "issues": issues,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("=== Rairos Health Check ===");
        println!();

        println!("[OK] Checks passed ({}):", ok.len());
        for item in &ok {
            println!("  ✓ {}", item);
        }

        if !warnings.is_empty() {
            println!();
            println!("[WARN] Warnings ({}):", warnings.len());
            for w in &warnings {
                println!("  ⚠ {}", w);
            }
        }

        if !issues.is_empty() {
            println!();
            println!("[FAIL] Issues ({}):", issues.len());
            for i in &issues {
                println!("  ✗ {}", i);
            }
        }

        println!();
        if issues.is_empty() {
            println!("All checks passed.");
        } else {
            println!("{} issue(s) found.", issues.len());
        }
    }

    Ok(())
}