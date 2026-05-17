//! API Metrics

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[derive(Default)]
pub struct ApiMetrics {
    pub requests_total: AtomicU64,
    pub requests_by_endpoint: RwLock<HashMap<String, AtomicU64>>,
    pub requests_by_tier: RwLock<HashMap<String, AtomicU64>>,
    pub errors_total: AtomicU64,
}

impl ApiMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_request(&self, endpoint: &str, tier: &str) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);

        if let Ok(mut endpoints) = self.requests_by_endpoint.try_write() {
            let counter = endpoints.entry(endpoint.to_string()).or_insert_with(|| AtomicU64::new(0));
            counter.fetch_add(1, Ordering::Relaxed);
        }

        if let Ok(mut tiers) = self.requests_by_tier.try_write() {
            let counter = tiers.entry(tier.to_string()).or_insert_with(|| AtomicU64::new(0));
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_error(&self) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();

        output.push_str("# TYPE rairos_api_requests_total counter\n");
        output.push_str(&format!("rairos_api_requests_total {}\n", self.requests_total.load(Ordering::Relaxed)));

        output.push_str("# TYPE rairos_api_errors_total counter\n");
        output.push_str(&format!("rairos_api_errors_total {}\n", self.errors_total.load(Ordering::Relaxed)));

        if let Ok(endpoints) = self.requests_by_endpoint.try_read() {
            output.push_str("# TYPE rairos_api_requests_by_endpoint counter\n");
            for (endpoint, count) in endpoints.iter() {
                output.push_str(&format!("rairos_api_requests_by_endpoint{{endpoint=\"{}\"}} {}\n", endpoint, count.load(Ordering::Relaxed)));
            }
        }

        if let Ok(tiers) = self.requests_by_tier.try_read() {
            output.push_str("# TYPE rairos_api_requests_by_tier counter\n");
            for (tier, count) in tiers.iter() {
                output.push_str(&format!("rairos_api_requests_by_tier{{tier=\"{}\"}} {}\n", tier, count.load(Ordering::Relaxed)));
            }
        }

        output
    }
}

pub type SharedMetrics = Arc<ApiMetrics>;

pub fn create_metrics() -> SharedMetrics {
    Arc::new(ApiMetrics::new())
}
