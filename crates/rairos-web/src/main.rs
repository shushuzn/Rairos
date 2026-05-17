use rairos_core::Database;
use rairos_web::{start, AppState};
use rairos_observability::layer::{init_logging, cleanup_old_logs, log_dir};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = init_logging();

    let log_dir = log_dir();
    if let Ok(removed) = cleanup_old_logs(&log_dir, 30) {
        if removed > 0 {
            eprintln!("Cleaned up {} old log files", removed);
        }
    }

    let db_path = std::env::var("RAIROS_DB").unwrap_or_else(|_| "rairos.db".to_string());
    let db = Database::open(&db_path).expect("Failed to open database");

    let addr = std::env::var("RAIROS_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());

    let state = Arc::new(AppState::new(db));
    start(&addr, state).await?;

    Ok(())
}
