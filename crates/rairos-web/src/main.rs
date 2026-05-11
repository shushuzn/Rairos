use rairos_core::Database;
use rairos_web::{start, AppState};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let db_path = std::env::var("RAIROS_DB").unwrap_or_else(|_| "rairos.db".to_string());
    let db = Database::open(&db_path).expect("Failed to open database");

    let addr = std::env::var("RAIROS_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());

    let state = Arc::new(AppState::new(db));
    start(&addr, state).await?;

    Ok(())
}