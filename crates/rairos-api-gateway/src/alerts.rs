//! Alert Configuration and Notification Handling

use chrono::Utc;
use reqwest::Client;
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::{AlertConfig, AlertPayload, AlertStatus};
use crate::state::AppState;

const DEFAULT_THRESHOLD_PERCENT: i32 = 80;
const MIN_ALERT_INTERVAL_HOURS: i64 = 24;

impl AlertConfig {
    pub fn should_send_alert(&self, usage_percent: f64) -> bool {
        if usage_percent < self.threshold_percent as f64 {
            return false;
        }

        if let Some(last_alerted) = self.last_alerted_at {
            let hours_since_last_alert = (Utc::now() - last_alerted).num_hours();
            if hours_since_last_alert < MIN_ALERT_INTERVAL_HOURS {
                return false;
            }
        }

        true
    }

    pub fn usage_percent(requests_used: i64, requests_limit: i64) -> f64 {
        if requests_limit == 0 {
            return 0.0;
        }
        if requests_limit == i64::MAX {
            return 0.0;
        }
        (requests_used as f64 / requests_limit as f64) * 100.0
    }
}

pub struct AlertService;

impl AlertService {
    pub async fn get_alert_config(
        state: &AppState,
        user_id: Uuid,
    ) -> Result<Option<AlertConfig>, ApiError> {
        let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let row = conn
            .query_opt(
                "SELECT id, user_id, threshold_percent, email_alert, webhook_url,
                        last_alerted_at, created_at, updated_at
                 FROM alert_configs WHERE user_id = $1",
                &[&user_id.to_string()],
            )
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(row.map(|r| AlertConfig {
            id: r.get("id"),
            user_id: r.get("user_id"),
            threshold_percent: r.get("threshold_percent"),
            email_alert: r.get("email_alert"),
            webhook_url: r.get("webhook_url"),
            last_alerted_at: r.get("last_alerted_at"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    pub async fn create_alert_config(
        state: &AppState,
        user_id: Uuid,
        threshold_percent: Option<i32>,
        email_alert: Option<bool>,
        webhook_url: Option<String>,
    ) -> Result<AlertConfig, ApiError> {
        let threshold = threshold_percent.unwrap_or(DEFAULT_THRESHOLD_PERCENT);
        let email = email_alert.unwrap_or(true);

        if threshold <= 0 || threshold > 100 {
            return Err(ApiError::ValidationError(
                "threshold_percent must be between 1 and 100".to_string(),
            ));
        }

        if let Some(ref url) = webhook_url {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(ApiError::ValidationError(
                    "webhook_url must be a valid HTTP/HTTPS URL".to_string(),
                ));
            }
        }

        let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let id = Uuid::new_v4();
        let now = Utc::now();

        conn.execute(
            "INSERT INTO alert_configs (id, user_id, threshold_percent, email_alert, webhook_url, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $6)",
            &[&id.to_string(), &user_id.to_string(), &threshold, &email, &webhook_url, &now],
        )
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(AlertConfig {
            id,
            user_id,
            threshold_percent: threshold,
            email_alert: email,
            webhook_url,
            last_alerted_at: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn update_alert_config(
        state: &AppState,
        user_id: Uuid,
        threshold_percent: Option<i32>,
        email_alert: Option<bool>,
        webhook_url: Option<String>,
    ) -> Result<AlertConfig, ApiError> {
        let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let existing = Self::get_alert_config(state, user_id).await?;
        let existing = existing.ok_or_else(|| ApiError::NotFound("Alert configuration not found".to_string()))?;

        let threshold = threshold_percent.unwrap_or(existing.threshold_percent);
        let email = email_alert.unwrap_or(existing.email_alert);
        let webhook = webhook_url.or(existing.webhook_url);

        if threshold <= 0 || threshold > 100 {
            return Err(ApiError::ValidationError(
                "threshold_percent must be between 1 and 100".to_string(),
            ));
        }

        if let Some(ref url) = webhook {
            if !url.is_empty() && !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(ApiError::ValidationError(
                    "webhook_url must be a valid HTTP/HTTPS URL".to_string(),
                ));
            }
        }

        let now = Utc::now();

        conn.execute(
            "UPDATE alert_configs
             SET threshold_percent = $1, email_alert = $2, webhook_url = $3, updated_at = $4
             WHERE user_id = $5",
            &[&threshold, &email, &webhook, &now, &user_id.to_string()],
        )
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(AlertConfig {
            id: existing.id,
            user_id,
            threshold_percent: threshold,
            email_alert: email,
            webhook_url: webhook,
            last_alerted_at: existing.last_alerted_at,
            created_at: existing.created_at,
            updated_at: now,
        })
    }

    pub async fn delete_alert_config(
        state: &AppState,
        user_id: Uuid,
    ) -> Result<(), ApiError> {
        let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        conn.execute(
            "DELETE FROM alert_configs WHERE user_id = $1",
            &[&user_id.to_string()],
        )
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn get_alert_status(
        state: &AppState,
        user_id: Uuid,
        requests_used: i64,
        requests_limit: i64,
    ) -> Result<AlertStatus, ApiError> {
        let config = Self::get_alert_config(state, user_id).await?;

        let usage_percent = AlertConfig::usage_percent(requests_used, requests_limit);

        match config {
            Some(cfg) => {
                let should_alert = cfg.should_send_alert(usage_percent);
                Ok(AlertStatus {
                    current_usage: requests_used,
                    limit: requests_limit,
                    usage_percent,
                    threshold_percent: cfg.threshold_percent,
                    should_alert,
                    last_alerted_at: cfg.last_alerted_at,
                })
            }
            None => {
                let default_threshold = DEFAULT_THRESHOLD_PERCENT;
                let should_alert = usage_percent >= default_threshold as f64;
                Ok(AlertStatus {
                    current_usage: requests_used,
                    limit: requests_limit,
                    usage_percent,
                    threshold_percent: default_threshold,
                    should_alert,
                    last_alerted_at: None,
                })
            }
        }
    }

    pub async fn send_webhook_alert(
        webhook_url: &str,
        user_id: &str,
        tier: &str,
        current_usage: i64,
        limit: i64,
        usage_percent: f64,
        threshold_percent: i32,
    ) -> Result<(), ApiError> {
        let client = Client::new();
        let payload = AlertPayload {
            alert_type: "usage_threshold".to_string(),
            user_id: user_id.to_string(),
            tier: tier.to_string(),
            current_usage,
            limit,
            usage_percent,
            threshold_percent,
            message: format!(
                "Usage alert: You have used {:.1}% of your daily quota ({}/{} requests). \
                 Consider upgrading your plan to avoid rate limiting.",
                usage_percent, current_usage, limit
            ),
            timestamp: Utc::now(),
        };

        let response = client
            .post(webhook_url)
            .json(&payload)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to send webhook: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::Internal(format!(
                "Webhook returned error: {} - {}",
                status, body
            )));
        }

        Ok(())
    }

    pub async fn record_alert_sent(
        state: &AppState,
        user_id: Uuid,
        alert_type: &str,
        threshold_percent: i32,
        usage_percent: f64,
        requests_used: i64,
        requests_limit: i64,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<(), ApiError> {
        let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        conn.execute(
            "INSERT INTO alert_history (id, user_id, alert_type, threshold_percent,
             usage_percent, requests_used, requests_limit, status, error_message)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            &[
                &Uuid::new_v4().to_string(),
                &user_id.to_string(),
                &alert_type.to_string(),
                &threshold_percent,
                &(usage_percent as i32),
                &requests_used,
                &requests_limit,
                &status.to_string(),
                &error_message,
            ],
        )
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn update_last_alerted(
        state: &AppState,
        user_id: Uuid,
    ) -> Result<(), ApiError> {
        let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        conn.execute(
            "UPDATE alert_configs SET last_alerted_at = $1 WHERE user_id = $2",
            &[&Utc::now(), &user_id.to_string()],
        )
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_percent_calculation() {
        assert_eq!(AlertConfig::usage_percent(50, 100), 50.0);
        assert_eq!(AlertConfig::usage_percent(100, 100), 100.0);
        assert_eq!(AlertConfig::usage_percent(0, 100), 0.0);
        assert_eq!(AlertConfig::usage_percent(1000, 10000), 10.0);
    }

    #[test]
    fn test_usage_percent_unlimited() {
        assert_eq!(AlertConfig::usage_percent(1000000, i64::MAX), 0.0);
    }

    #[test]
    fn test_should_send_alert_below_threshold() {
        let config = AlertConfig {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            threshold_percent: 80,
            email_alert: true,
            webhook_url: Some("https://example.com/webhook".to_string()),
            last_alerted_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(!config.should_send_alert(50.0));
        assert!(!config.should_send_alert(79.0));
    }

    #[test]
    fn test_should_send_alert_above_threshold_no_previous() {
        let config = AlertConfig {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            threshold_percent: 80,
            email_alert: true,
            webhook_url: Some("https://example.com/webhook".to_string()),
            last_alerted_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(config.should_send_alert(80.0));
        assert!(config.should_send_alert(90.0));
        assert!(config.should_send_alert(100.0));
    }

    #[test]
    fn test_should_send_alert_recent_previous_alert() {
        let config = AlertConfig {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            threshold_percent: 80,
            email_alert: true,
            webhook_url: Some("https://example.com/webhook".to_string()),
            last_alerted_at: Some(Utc::now() - chrono::Duration::hours(12)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(!config.should_send_alert(90.0));
    }

    #[test]
    fn test_should_send_alert_old_previous_alert() {
        let config = AlertConfig {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            threshold_percent: 80,
            email_alert: true,
            webhook_url: Some("https://example.com/webhook".to_string()),
            last_alerted_at: Some(Utc::now() - chrono::Duration::hours(25)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(config.should_send_alert(90.0));
    }
}
