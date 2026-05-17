//! Stripe Webhook Handler
//!
//! Handles Stripe webhook events to sync subscription status.

use serde::{Deserialize, Serialize};
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;

const TOLERANCE_SECONDS: i64 = 300;

type HmacSha256 = Hmac<Sha256>;

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

pub fn verify_stripe_signature(
    payload: &str,
    signature_header: &str,
    secret: &str,
) -> Result<(), StripeSignatureError> {
    if signature_header.is_empty() {
        return Err(StripeSignatureError::MissingSignature);
    }

    let mut timestamp: Option<i64> = None;
    let mut signatures: Vec<String> = Vec::new();

    for part in signature_header.split(',') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix("t=") {
            timestamp = Some(value.parse().map_err(|_| StripeSignatureError::InvalidFormat)?);
        } else if let Some(value) = part.strip_prefix("v1=") {
            signatures.push(value.to_string());
        }
    }

    let timestamp = timestamp.ok_or(StripeSignatureError::InvalidFormat)?;

    let now = Utc::now().timestamp();
    if (now - timestamp).abs() > TOLERANCE_SECONDS {
        return Err(StripeSignatureError::TimestampExpired);
    }

    let signed_payload = format!("{}.{}", timestamp, payload);

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| StripeSignatureError::MacError)?;
    mac.update(signed_payload.as_bytes());

    let expected_signature = hex::encode(mac.finalize().into_bytes());

    let valid = signatures.iter().any(|sig| {
        constant_time_eq(sig.as_bytes(), expected_signature.as_bytes())
    });

    if valid {
        Ok(())
    } else {
        Err(StripeSignatureError::SignatureMismatch)
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

#[derive(Debug)]
pub enum StripeSignatureError {
    MissingSignature,
    InvalidFormat,
    TimestampExpired,
    SignatureMismatch,
    MacError,
}

impl std::fmt::Display for StripeSignatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSignature => write!(f, "Missing Stripe signature header"),
            Self::InvalidFormat => write!(f, "Invalid signature header format"),
            Self::TimestampExpired => write!(f, "Webhook timestamp expired (replay attack prevention)"),
            Self::SignatureMismatch => write!(f, "Signature verification failed"),
            Self::MacError => write!(f, "MAC computation error"),
        }
    }
}

impl std::error::Error for StripeSignatureError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_tier_from_status() {
        assert_eq!(get_tier_from_status("active"), "active");
        assert_eq!(get_tier_from_status("past_due"), "past_due");
        assert_eq!(get_tier_from_status("canceled"), "canceled");
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }
}
