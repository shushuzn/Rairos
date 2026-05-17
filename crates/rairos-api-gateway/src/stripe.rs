//! Stripe Subscription Integration

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionTier {
    pub name: &'static str,
    pub price_id: &'static str,
    pub price_monthly: i64,
    pub requests_limit: i64,
}

pub const SUBSCRIPTION_TIERS: &[SubscriptionTier] = &[
    SubscriptionTier {
        name: "free",
        price_id: "",
        price_monthly: 0,
        requests_limit: 100,
    },
    SubscriptionTier {
        name: "pro",
        price_id: "price_pro_monthly",
        price_monthly: 2900,
        requests_limit: 10_000,
    },
    SubscriptionTier {
        name: "team",
        price_id: "price_team_monthly",
        price_monthly: 9900,
        requests_limit: 100_000,
    },
    SubscriptionTier {
        name: "enterprise",
        price_id: "price_enterprise_monthly",
        price_monthly: 49900,
        requests_limit: i64::MAX,
    },
];

pub fn get_tier_by_name(name: &str) -> Option<&'static SubscriptionTier> {
    SUBSCRIPTION_TIERS.iter().find(|t| t.name == name)
}

pub fn get_tier_by_price_id(price_id: &str) -> Option<&'static SubscriptionTier> {
    SUBSCRIPTION_TIERS.iter().find(|t| t.price_id == price_id)
}

pub fn get_tier_by_checkout_session(data: &serde_json::Value) -> Option<&'static SubscriptionTier> {
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
