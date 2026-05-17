//! Stripe Subscription Integration
//!
//! Price IDs are loaded from environment variables at runtime.
//! Set these in your environment or .env file:
//! - STRIPE_PRICE_PRO_MONTHLY
//! - STRIPE_PRICE_PRO_ANNUAL
//! - STRIPE_PRICE_TEAM_MONTHLY
//! - STRIPE_PRICE_TEAM_ANNUAL
//! - STRIPE_PRICE_ENTERPRISE_MONTHLY
//! - STRIPE_PRICE_ENTERPRISE_ANNUAL

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::models::Tier;

static PRICE_IDS: OnceLock<PriceIds> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct PriceIds {
    pub pro_monthly: String,
    pub pro_annual: String,
    pub team_monthly: String,
    pub team_annual: String,
    pub enterprise_monthly: String,
    pub enterprise_annual: String,
}

impl PriceIds {
    pub fn from_env() -> Self {
        Self {
            pro_monthly: std::env::var("STRIPE_PRICE_PRO_MONTHLY")
                .unwrap_or_else(|_| "price_pro_monthly".to_string()),
            pro_annual: std::env::var("STRIPE_PRICE_PRO_ANNUAL")
                .unwrap_or_else(|_| "price_pro_annual".to_string()),
            team_monthly: std::env::var("STRIPE_PRICE_TEAM_MONTHLY")
                .unwrap_or_else(|_| "price_team_monthly".to_string()),
            team_annual: std::env::var("STRIPE_PRICE_TEAM_ANNUAL")
                .unwrap_or_else(|_| "price_team_annual".to_string()),
            enterprise_monthly: std::env::var("STRIPE_PRICE_ENTERPRISE_MONTHLY")
                .unwrap_or_else(|_| "price_enterprise_monthly".to_string()),
            enterprise_annual: std::env::var("STRIPE_PRICE_ENTERPRISE_ANNUAL")
                .unwrap_or_else(|_| "price_enterprise_annual".to_string()),
        }
    }

    pub fn get() -> &'static PriceIds {
        PRICE_IDS.get_or_init(|| PriceIds::from_env())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionTier {
    pub name: &'static str,
    pub price_id: String,
    pub price_monthly: i64,
    pub requests_limit: i64,
}

pub fn get_subscription_tiers() -> Vec<SubscriptionTier> {
    let ids = PriceIds::get();
    vec![
        SubscriptionTier {
            name: "free",
            price_id: String::new(),
            price_monthly: 0,
            requests_limit: Tier::Free.requests_limit(),
        },
        SubscriptionTier {
            name: "pro",
            price_id: ids.pro_monthly.clone(),
            price_monthly: 2900,
            requests_limit: Tier::Pro.requests_limit(),
        },
        SubscriptionTier {
            name: "team",
            price_id: ids.team_monthly.clone(),
            price_monthly: 9900,
            requests_limit: Tier::Team.requests_limit(),
        },
        SubscriptionTier {
            name: "enterprise",
            price_id: ids.enterprise_monthly.clone(),
            price_monthly: 49900,
            requests_limit: Tier::Enterprise.requests_limit(),
        },
    ]
}

pub fn get_tier_by_name(name: &str) -> Option<SubscriptionTier> {
    get_subscription_tiers().into_iter().find(|t| t.name == name)
}

pub fn get_tier_by_price_id(price_id: &str) -> Option<SubscriptionTier> {
    get_subscription_tiers().into_iter().find(|t| t.price_id == price_id)
}

pub fn get_tier_by_checkout_session(data: &serde_json::Value) -> Option<SubscriptionTier> {
    let price_id = data
        .get("line_items")
        .and_then(|li| li.get("data"))
        .and_then(|items| items.as_array())
        .and_then(|items| items.first())
        .and_then(|item| item.get("price"))
        .and_then(|price| price.get("id"))
        .and_then(|id| id.as_str())?;

    get_tier_by_price_id(price_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeWebhookEvent {
    pub id: String,
    pub event_type: String,
    pub created: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_tier_by_name() {
        let pro = get_tier_by_name("pro").unwrap();
        assert_eq!(pro.price_monthly, 2900);
        assert_eq!(pro.requests_limit, 10_000);
    }

    #[test]
    fn test_get_tier_by_price_id() {
        let tier = get_tier_by_price_id("price_team_monthly").unwrap();
        assert_eq!(tier.name, "team");
    }
}
