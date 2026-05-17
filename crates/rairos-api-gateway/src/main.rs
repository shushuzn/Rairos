//! Rairos API Gateway Server

use rairos_api_gateway::{create_app, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/rairos_api".to_string());

    let state = AppState::new(&database_url).await?;

    let app = create_app(state);

    let addr = std::env::var("ADDR").unwrap_or_else(|_| "0.0.0.0:8081".to_string());

    tracing::info!("Starting Rairos API Gateway on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
