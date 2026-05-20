//! API Metrics

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use rustc_hash::FxHashMap;

#[derive(Default)]
pub struct ApiMetrics {
    pub requests_total: AtomicU64,
    pub requests_by_endpoint: RwLock<FxHashMap<String, AtomicU64>>,
    pub requests_by_tier: RwLock<FxHashMap<String, AtomicU64>>,
    pub errors_total: AtomicU64,
    pub subscriptions_by_tier: RwLock<FxHashMap<String, AtomicU64>>,
    pub mrr_cents: AtomicU64,
    pub active_users: AtomicU64,
    pub dau: AtomicU64,
    pub mau: AtomicU64,
    pub api_costs_cents: AtomicU64,
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

    pub fn record_subscription(&self, tier: &str) {
        if let Ok(mut tiers) = self.subscriptions_by_tier.try_write() {
            let counter = tiers.entry(tier.to_string()).or_insert_with(|| AtomicU64::new(0));
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn update_mrr(&self, cents: u64) {
        self.mrr_cents.store(cents, Ordering::Relaxed);
    }

    pub fn update_active_users(&self, count: u64) {
        self.active_users.store(count, Ordering::Relaxed);
    }

    pub fn update_dau(&self, count: u64) {
        self.dau.store(count, Ordering::Relaxed);
    }

    pub fn update_mau(&self, count: u64) {
        self.mau.store(count, Ordering::Relaxed);
    }

    pub fn record_api_cost(&self, cents: u64) {
        self.api_costs_cents.fetch_add(cents, Ordering::Relaxed);
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

        output.push_str("# TYPE rairos_subscriptions_by_tier gauge\n");
        if let Ok(tiers) = self.subscriptions_by_tier.try_read() {
            for (tier, count) in tiers.iter() {
                output.push_str(&format!("rairos_subscriptions_by_tier{{tier=\"{}\"}} {}\n", tier, count.load(Ordering::Relaxed)));
            }
        }

        output.push_str("# TYPE rairos_mrr_cents gauge\n");
        output.push_str(&format!("rairos_mrr_cents {}\n", self.mrr_cents.load(Ordering::Relaxed)));

        output.push_str("# TYPE rairos_active_users gauge\n");
        output.push_str(&format!("rairos_active_users {}\n", self.active_users.load(Ordering::Relaxed)));

        output.push_str("# TYPE rairos_dau gauge\n");
        output.push_str(&format!("rairos_dau {}\n", self.dau.load(Ordering::Relaxed)));

        output.push_str("# TYPE rairos_mau gauge\n");
        output.push_str(&format!("rairos_mau {}\n", self.mau.load(Ordering::Relaxed)));

        output.push_str("# TYPE rairos_api_costs_cents counter\n");
        output.push_str(&format!("rairos_api_costs_cents {}\n", self.api_costs_cents.load(Ordering::Relaxed)));

        output
    }
}

pub type SharedMetrics = Arc<ApiMetrics>;

pub fn create_metrics() -> SharedMetrics {
    Arc::new(ApiMetrics::new())
}
