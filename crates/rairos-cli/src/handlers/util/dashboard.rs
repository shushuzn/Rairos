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
use std::sync::Arc;

pub fn handle_dashboard(port: u16, host: &str, _no_browser: bool) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let addr = format!("{}:{}", host, port);
        let db = Database::open("rairos.db")?;
        let state = Arc::new(rairos_web::AppState::new(db));
        println!("🚀 Rairos Web UI starting on http://{}", addr);
        rairos_web::start(&addr, state).await.map_err(|e| anyhow::anyhow!("Web UI failed: {}", e))
    })?;
    Ok(())
}