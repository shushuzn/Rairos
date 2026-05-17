//! Stripe Webhook Handler
//!
//! Handles Stripe webhook events to sync subscription status.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Deserialize)]
pub struct StripeWebhookPayload {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: StripeEventData,
    pub id: String,
    pub created: i64,
}

#[derive(Debug, Deserialize)]
pub struct StripeEventData {
    pub object: StripeObject,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StripeObject {
    pub id: Option<String>,
    pub customer: Option<String>,
    pub subscription: Option<String>,
    pub status: Option<String>,
    pub price: Option<StripePrice>,
    pub customer_email: Option<String>,
    pub metadata: Option<StripeMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct StripePrice {
    pub id: Option<String>,
    pub product: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StripeMetadata {
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionEvent {
    pub customer_id: String,
    pub subscription_id: String,
    pub status: String,
    pub tier: String,
    pub price_id: Option<String>,
    pub current_period_end: Option<i64>,
}

impl StripeWebhookPayload {
    pub fn parse_event(self) -> Option<SubscriptionEvent> {
        let obj = &self.data.object;

        let customer_id = obj.customer.clone()?;
        let subscription_id = obj.subscription.clone()?;

        let tier = if let Some(price) = &obj.price {
            crate::stripe::get_tier_by_price_id(&price.id.as_deref().unwrap_or(""))
                .map(|t| t.name.to_string())
                .unwrap_or_else(|| "free".to_string())
        } else {
            "free".to_string()
        };

        Some(SubscriptionEvent {
            customer_id,
            subscription_id,
            status: obj.status.clone().unwrap_or_else(|| "unknown".to_string()),
            tier,
            price_id: obj.price.as_ref().and_then(|p| p.id.clone()),
            current_period_end: None,
        })
    }
}

pub fn get_tier_from_status(status: &str) -> &'static str {
    match status {
        "active" | "trialing" => "active",
        "past_due" => "past_due",
        "canceled" | "unpaid" => "canceled",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_tier_from_status() {
        assert_eq!(get_tier_from_status("active"), "active");
        assert_eq!(get_tier_from_status("past_due"), "past_due");
        assert_eq!(get_tier_from_status("canceled"), "canceled");
    }
}
