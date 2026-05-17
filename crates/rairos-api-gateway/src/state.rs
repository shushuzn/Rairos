//! Application State

use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;
use std::sync::Arc;

use crate::metrics::SharedMetrics;
use crate::ratelimit::RateLimiter;

pub type PostgresPool = Pool<PostgresConnectionManager<NoTls>>;

#[derive(Clone)]
pub struct AppState {
    pub db: PostgresPool,
    pub rate_limiter: Arc<RateLimiter>,
    pub metrics: SharedMetrics,
    pub stripe_webhook_secret: Option<String>,
}

impl AppState {
    pub async fn new(database_url: &str, stripe_webhook_secret: Option<String>) -> anyhow::Result<Self> {
        let mgr = PostgresConnectionManager::new_from_stringlike(database_url, NoTls)?;
        let db = Pool::builder().build(mgr).await?;

        let rate_limiter = Arc::new(RateLimiter::new());
        let metrics = crate::metrics::create_metrics();

        Ok(Self {
            db,
            rate_limiter,
            metrics,
            stripe_webhook_secret,
        })
    }
}
