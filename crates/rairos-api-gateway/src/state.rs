//! Application State

use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;
use std::sync::Arc;
use redis::aio::MultiplexedConnection;

use crate::metrics::SharedMetrics;
use crate::ratelimit::RateLimiter;

pub type PostgresPool = Pool<PostgresConnectionManager<NoTls>>;
pub type RedisPool = redis::Client;

#[derive(Clone)]
pub struct AppState {
    pub db: PostgresPool,
    pub redis: Option<RedisPool>,
    pub rate_limiter: Arc<RateLimiter>,
    pub metrics: SharedMetrics,
    pub stripe_webhook_secret: Option<String>,
}

impl AppState {
    pub async fn new(
        database_url: &str,
        redis_url: Option<&str>,
        stripe_webhook_secret: Option<String>,
    ) -> anyhow::Result<Self> {
        let mgr = PostgresConnectionManager::new_from_stringlike(database_url, NoTls)?;
        let db = Pool::builder().build(mgr).await?;

        let redis = redis_url.and_then(|url| {
            redis::Client::open(url).ok()
        });

        let rate_limiter = Arc::new(RateLimiter::new(redis.clone()));
        let metrics = crate::metrics::create_metrics();

        Ok(Self {
            db,
            redis,
            rate_limiter,
            metrics,
            stripe_webhook_secret,
        })
    }
}
