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
use std::sync::Arc;

use rairos_core::Database;
use rairos_web::{start, AppState};

pub fn handle_daemon(db: &Database, port: u16, _log_level: &str, _foreground: bool) -> Result<()> {
    println!("Starting Rairos web server on port {}...", port);
    println!();
    println!("API endpoints:");
    println!("  GET  /              - Web UI");
    println!("  GET  /health        - Health check");
    println!("  GET  /stats         - Database stats");
    println!("  GET  /papers        - List papers");
    println!("  GET  /papers/:id    - Get paper details");
    println!("  GET  /papers/search - Search papers");
    println!("  GET  /gaps          - List research gaps");
    println!("  GET  /genes         - List gene pool");
    println!("  GET  /genes/diversity - Gene diversity metrics");
    println!("  GET  /kg/stats      - Knowledge graph stats");
    println!("  GET  /kg/rank       - Paper rankings");
    println!();
    println!("Press Ctrl+C to stop.");
    println!();

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let state = Arc::new(AppState::new(db.clone()));
        start(&format!("127.0.0.1:{}", port), state).await
    })?;
    Ok(())
}