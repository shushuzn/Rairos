//! Application State

use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;
use std::sync::Arc;

use crate::ratelimit::RateLimiter;

pub type PostgresPool = Pool<PostgresConnectionManager<NoTls>>;

#[derive(Clone)]
pub struct AppState {
    pub db: PostgresPool,
    pub rate_limiter: Arc<RateLimiter>,
}

impl AppState {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let mgr = PostgresConnectionManager::new_from_stringlike(database_url, NoTls)?;
        let db = Pool::builder().build(mgr).await?;

        let rate_limiter = Arc::new(RateLimiter::new());

        Ok(Self {
            db,
            rate_limiter,
        })
    }
}
