//! Application State

use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;
use std::sync::Arc;
use std::time::Duration;

use crate::metrics::SharedMetrics;
use crate::ratelimit::RateLimiter;

pub type PostgresPool = Pool<PostgresConnectionManager<NoTls>>;
pub type RedisPool = redis::Client;

const DEFAULT_MIN_POOL_SIZE: u32 = 5;
const DEFAULT_MAX_POOL_SIZE: u32 = 20;
const DEFAULT_CONNECTION_TIMEOUT_SECS: u64 = 30;

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

        let min_pool_size: u32 = std::env::var("DB_MIN_POOL_SIZE")
            .unwrap_or_else(|_| DEFAULT_MIN_POOL_SIZE.to_string())
            .parse()
            .unwrap_or(DEFAULT_MIN_POOL_SIZE);

        let max_pool_size: u32 = std::env::var("DB_MAX_POOL_SIZE")
            .unwrap_or_else(|_| DEFAULT_MAX_POOL_SIZE.to_string())
            .parse()
            .unwrap_or(DEFAULT_MAX_POOL_SIZE);

        let connection_timeout_secs: u64 = std::env::var("DB_CONNECTION_TIMEOUT_SECS")
            .unwrap_or_else(|_| DEFAULT_CONNECTION_TIMEOUT_SECS.to_string())
            .parse()
            .unwrap_or(DEFAULT_CONNECTION_TIMEOUT_SECS);

        let db = Pool::builder()
            .min_idle(Some(min_pool_size))
            .max_size(max_pool_size)
            .connection_timeout(Duration::from_secs(connection_timeout_secs))
            .build(mgr)
            .await?;

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
