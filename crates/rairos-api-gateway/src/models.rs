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
