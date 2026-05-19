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
use std::time::{Duration, Instant};

pub fn handle_benchmark(kind: &str, iterations: usize) -> Result<()> {
    println!("=== Rairos Benchmark ===");
    println!("Type: {} | Iterations: {}\n", kind, iterations);

    match kind {
        "all" | "db" => {
            println!("[DB] Running database benchmark...");
            let db_path = std::path::PathBuf::from("rairos.db");
            if !db_path.exists() {
                println!("No database at rairos.db, skipping DB benchmark");
            } else if let Ok(db) = Database::open(&db_path) {
                let start = Instant::now();
                for _ in 0..iterations {
                    let _ = db.stats();
                }
                let elapsed = start.elapsed();
                println!(
                    "[DB] {} stats() calls in {:.3}s ({:.0} ops/s)",
                    iterations,
                    elapsed.as_secs_f64(),
                    iterations as f64 / elapsed.as_secs_f64()
                );

                let start = Instant::now();
                for _ in 0..iterations.min(100) {
                    let _ = db.search_papers_smart("machine learning", 10);
                }
                let elapsed = start.elapsed();
                let ops = iterations.min(100);
                println!(
                    "[DB] {} search() calls in {:.3}s ({:.0} ops/s)",
                    ops,
                    elapsed.as_secs_f64(),
                    ops as f64 / elapsed.as_secs_f64()
                );
            } else {
                println!("Could not open database");
            }
        }
        "api" => {
            println!("[API] Running API benchmark (measuring TCP connection latency)...");
            let port = 8080u16;
            let start = Instant::now();
            let mut ok = 0;
            for _ in 0..iterations {
                if let Ok(stream) = std::net::TcpStream::connect_timeout(
                    &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
                    Duration::from_millis(500),
                ) {
                    stream
                        .set_read_timeout(Some(Duration::from_millis(500)))
                        .ok();
                    ok += 1;
                }
            }
            let elapsed = start.elapsed();
            if ok > 0 {
                println!(
                    "[API] {} TCP connect attempts in {:.3}s ({:.0} conn/s), {} OK",
                    iterations,
                    elapsed.as_secs_f64(),
                    iterations as f64 / elapsed.as_secs_f64(),
                    ok
                );
            } else {
                println!(
                    "[API] Could not reach localhost:{} (no server running?)",
                    port
                );
                println!(
                    "[API] {} attempts in {:.3}s (no server to test)",
                    iterations,
                    elapsed.as_secs_f64()
                );
            }
        }
        "parse" => {
            println!("[Parse] Running parse benchmark...");
            let sample_text =
                "This is a sample abstract about machine learning and neural networks. ".repeat(50);
            let start = Instant::now();
            for _ in 0..iterations {
                let words: Vec<&str> = sample_text.split_whitespace().collect();
                let mut count = 0;
                for w in &words {
                    if w.len() > 3 {
                        count += 1;
                    }
                }
                let _ = count;
            }
            println!(
                "[Parse] {} text processing iterations in {:.3}s",
                iterations,
                start.elapsed().as_secs_f64()
            );
        }
        _ => {
            println!("Unknown benchmark type: {}. Use: all, db, api, parse", kind);
        }
    }

    println!("\n[OK] Benchmark complete");
    Ok(())
}