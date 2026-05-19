//! API Data Models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub tier: Tier,
    pub stripe_customer_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    #[default]
    Free,
    Pro,
    Team,
    Enterprise,
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tier::Free => write!(f, "free"),
            Tier::Pro => write!(f, "pro"),
            Tier::Team => write!(f, "team"),
            Tier::Enterprise => write!(f, "enterprise"),
        }
    }
}

impl Tier {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str<S: AsRef<str>>(s: S) -> Self {
        match s.as_ref() {
            "pro" => Tier::Pro,
            "team" => Tier::Team,
            "enterprise" => Tier::Enterprise,
            _ => Tier::Free,
        }
    }

    pub fn requests_limit(&self) -> i64 {
        match self {
            Tier::Free => 100,
            Tier::Pro => 10_000,
            Tier::Team => 100_000,
            Tier::Enterprise => i64::MAX,
        }
    }

    pub fn rate_limit_per_minute(&self) -> u32 {
        match self {
            Tier::Free => 10,
            Tier::Pro => 1_000,
            Tier::Team => 10_000,
            Tier::Enterprise => u32::MAX,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub key_hash: String,
    pub name: Option<String>,
    pub tier: Tier,
    pub requests_used: i64,
    pub requests_limit: i64,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub rotated_from: Option<Uuid>,
    pub rotated_at: Option<DateTime<Utc>>,
    pub grace_period_ends: Option<DateTime<Utc>>,
}

impl ApiKey {
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|e| e < Utc::now())
            .unwrap_or(false)
    }

    pub fn is_in_grace_period(&self) -> bool {
        self.grace_period_ends
            .map(|g| g > Utc::now())
            .unwrap_or(false)
    }

    pub fn is_limit_exceeded(&self) -> bool {
        self.requests_used >= self.requests_limit
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub stripe_subscription_id: Option<String>,
    pub tier: Tier,
    pub status: SubscriptionStatus,
    pub current_period_end: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    Cancelled,
    PastDue,
    Trialing,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub name: Option<String>,
    pub tier: Tier,
    pub requests_used: i64,
    pub requests_limit: i64,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<ApiKey> for ApiKeyResponse {
    fn from(key: ApiKey) -> Self {
        Self {
            id: key.id,
            name: key.name,
            tier: key.tier,
            requests_used: key.requests_used,
            requests_limit: key.requests_limit,
            created_at: key.created_at,
            expires_at: key.expires_at,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub user_id: Uuid,
    pub email: String,
    pub api_key: String,
    pub tier: Tier,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateKeyRequest {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RotateKeyRequest {
    pub key_id: Uuid,
    #[serde(default = "default_grace_period_hours")]
    pub grace_period_hours: i64,
}

fn default_grace_period_hours() -> i64 {
    24
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RotateKeyResponse {
    pub new_key: String,
    pub new_key_id: Uuid,
    pub old_key_id: Uuid,
    pub grace_period_ends: DateTime<Utc>,
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UsageResponse {
    pub tier: Tier,
    pub requests_used: i64,
    pub requests_limit: i64,
    pub requests_remaining: i64,
    pub reset_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationParams {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    20
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UsageDashboard {
    pub total_requests: i64,
    pub requests_today: i64,
    pub requests_this_week: i64,
    pub requests_this_month: i64,
    pub limit: i64,
    pub remaining: i64,
    pub usage_percent: f64,
    pub tier: Tier,
    pub reset_at: DateTime<Utc>,
    pub endpoint_breakdown: Vec<EndpointUsage>,
    pub daily_trend: Vec<DailyUsage>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EndpointUsage {
    pub endpoint: String,
    pub count: i64,
    pub avg_latency_ms: f64,
    pub last_called: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DailyUsage {
    pub date: String,
    pub count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_from_str() {
        assert_eq!(Tier::from_str("free"), Tier::Free);
        assert_eq!(Tier::from_str("pro"), Tier::Pro);
        assert_eq!(Tier::from_str("team"), Tier::Team);
        assert_eq!(Tier::from_str("enterprise"), Tier::Enterprise);
        assert_eq!(Tier::from_str("unknown"), Tier::Free);
        assert_eq!(Tier::from_str(""), Tier::Free);
        assert_eq!(Tier::from_str("FREE"), Tier::Free);
        assert_eq!(Tier::from_str("Pro"), Tier::Free);
    }

    #[test]
    fn test_tier_from_str_string() {
        let s = String::from("pro");
        assert_eq!(Tier::from_str(s), Tier::Pro);
    }

    #[test]
    fn test_tier_requests_limit() {
        assert_eq!(Tier::Free.requests_limit(), 100);
        assert_eq!(Tier::Pro.requests_limit(), 10_000);
        assert_eq!(Tier::Team.requests_limit(), 100_000);
        assert_eq!(Tier::Enterprise.requests_limit(), i64::MAX);
    }

    #[test]
    fn test_tier_rate_limit_per_minute() {
        assert_eq!(Tier::Free.rate_limit_per_minute(), 10);
        assert_eq!(Tier::Pro.rate_limit_per_minute(), 1_000);
        assert_eq!(Tier::Team.rate_limit_per_minute(), 10_000);
        assert_eq!(Tier::Enterprise.rate_limit_per_minute(), u32::MAX);
    }

    #[test]
    fn test_tier_display() {
        assert_eq!(Tier::Free.to_string(), "free");
        assert_eq!(Tier::Pro.to_string(), "pro");
        assert_eq!(Tier::Team.to_string(), "team");
        assert_eq!(Tier::Enterprise.to_string(), "enterprise");
    }

    #[test]
    fn test_api_key_is_expired_no_expiry() {
        let key = ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            key_hash: "hash".to_string(),
            name: None,
            tier: Tier::Free,
            requests_used: 0,
            requests_limit: 100,
            created_at: Utc::now(),
            expires_at: None,
            rotated_from: None,
            rotated_at: None,
            grace_period_ends: None,
        };
        assert!(!key.is_expired());
    }

    #[test]
    fn test_api_key_is_expired_future() {
        let key = ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            key_hash: "hash".to_string(),
            name: None,
            tier: Tier::Free,
            requests_used: 0,
            requests_limit: 100,
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::days(1)),
            rotated_from: None,
            rotated_at: None,
            grace_period_ends: None,
        };
        assert!(!key.is_expired());
    }

    #[test]
    fn test_api_key_is_expired_past() {
        let key = ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            key_hash: "hash".to_string(),
            name: None,
            tier: Tier::Free,
            requests_used: 0,
            requests_limit: 100,
            created_at: Utc::now(),
            expires_at: Some(Utc::now() - chrono::Duration::days(1)),
            rotated_from: None,
            rotated_at: None,
            grace_period_ends: None,
        };
        assert!(key.is_expired());
    }

    #[test]
    fn test_api_key_is_in_grace_period_no_grace() {
        let key = ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            key_hash: "hash".to_string(),
            name: None,
            tier: Tier::Free,
            requests_used: 0,
            requests_limit: 100,
            created_at: Utc::now(),
            expires_at: None,
            rotated_from: None,
            rotated_at: None,
            grace_period_ends: None,
        };
        assert!(!key.is_in_grace_period());
    }

    #[test]
    fn test_api_key_is_in_grace_period_active() {
        let key = ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            key_hash: "hash".to_string(),
            name: None,
            tier: Tier::Free,
            requests_used: 0,
            requests_limit: 100,
            created_at: Utc::now(),
            expires_at: None,
            rotated_from: None,
            rotated_at: None,
            grace_period_ends: Some(Utc::now() + chrono::Duration::hours(12)),
        };
        assert!(key.is_in_grace_period());
    }

    #[test]
    fn test_api_key_is_in_grace_period_expired() {
        let key = ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            key_hash: "hash".to_string(),
            name: None,
            tier: Tier::Free,
            requests_used: 0,
            requests_limit: 100,
            created_at: Utc::now(),
            expires_at: None,
            rotated_from: None,
            rotated_at: None,
            grace_period_ends: Some(Utc::now() - chrono::Duration::hours(1)),
        };
        assert!(!key.is_in_grace_period());
    }

    #[test]
    fn test_api_key_is_limit_exceeded_false() {
        let key = ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            key_hash: "hash".to_string(),
            name: None,
            tier: Tier::Free,
            requests_used: 50,
            requests_limit: 100,
            created_at: Utc::now(),
            expires_at: None,
            rotated_from: None,
            rotated_at: None,
            grace_period_ends: None,
        };
        assert!(!key.is_limit_exceeded());
    }

    #[test]
    fn test_api_key_is_limit_exceeded_true() {
        let key = ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            key_hash: "hash".to_string(),
            name: None,
            tier: Tier::Free,
            requests_used: 100,
            requests_limit: 100,
            created_at: Utc::now(),
            expires_at: None,
            rotated_from: None,
            rotated_at: None,
            grace_period_ends: None,
        };
        assert!(key.is_limit_exceeded());
    }

    #[test]
    fn test_api_key_is_limit_exceeded_unlimited() {
        let key = ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            key_hash: "hash".to_string(),
            name: None,
            tier: Tier::Enterprise,
            requests_used: 1_000_000,
            requests_limit: i64::MAX,
            created_at: Utc::now(),
            expires_at: None,
            rotated_from: None,
            rotated_at: None,
            grace_period_ends: None,
        };
        assert!(!key.is_limit_exceeded());
    }

    #[test]
    fn test_api_key_response_from_api_key() {
        let now = Utc::now();
        let key = ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            key_hash: "hash".to_string(),
            name: Some("Test Key".to_string()),
            tier: Tier::Pro,
            requests_used: 500,
            requests_limit: 10_000,
            created_at: now,
            expires_at: None,
            rotated_from: None,
            rotated_at: None,
            grace_period_ends: None,
        };
        let response: ApiKeyResponse = key.into();
        assert_eq!(response.tier, Tier::Pro);
        assert_eq!(response.requests_used, 500);
        assert_eq!(response.requests_limit, 10_000);
    }

    #[test]
    fn test_pagination_params_defaults() {
        let json = r#"{}"#;
        let params: PaginationParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.page, 1);
        assert_eq!(params.per_page, 20);
        assert!(params.q.is_none());
    }

    #[test]
    fn test_pagination_params_with_values() {
        let json = r#"{"q": "test", "page": 5, "per_page": 50}"#;
        let params: PaginationParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.q, Some("test".to_string()));
        assert_eq!(params.page, 5);
        assert_eq!(params.per_page, 50);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlertConfig {
    pub id: Uuid,
    pub user_id: Uuid,
    pub threshold_percent: i32,
    pub email_alert: bool,
    pub webhook_url: Option<String>,
    pub last_alerted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AlertConfigResponse {
    pub threshold_percent: i32,
    pub email_alert: bool,
    pub webhook_url: Option<String>,
    pub last_alerted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<AlertConfig> for AlertConfigResponse {
    fn from(config: AlertConfig) -> Self {
        Self {
            threshold_percent: config.threshold_percent,
            email_alert: config.email_alert,
            webhook_url: config.webhook_url,
            last_alerted_at: config.last_alerted_at,
            created_at: config.created_at,
            updated_at: config.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAlertConfigRequest {
    pub threshold_percent: Option<i32>,
    pub email_alert: Option<bool>,
    pub webhook_url: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAlertConfigRequest {
    pub threshold_percent: Option<i32>,
    pub email_alert: Option<bool>,
    pub webhook_url: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AlertStatus {
    pub current_usage: i64,
    pub limit: i64,
    pub usage_percent: f64,
    pub threshold_percent: i32,
    pub should_alert: bool,
    pub last_alerted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AlertPayload {
    pub alert_type: String,
    pub user_id: String,
    pub tier: String,
    pub current_usage: i64,
    pub limit: i64,
    pub usage_percent: f64,
    pub threshold_percent: i32,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}
