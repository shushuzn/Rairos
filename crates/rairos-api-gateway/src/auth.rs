//! API Authentication

use axum::{
    extract::State,
    http::Request,
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::error::{ApiError, Result};
use crate::models::{ApiKey, Tier};
use crate::state::AppState;

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    let api_key = auth_header
        .strip_prefix("Bearer ")
        .ok_or(ApiError::InvalidApiKey)?;

    let key_hash = hash_api_key(api_key);

    let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let row = conn.query_opt(
        "SELECT id, user_id, key_hash, name, tier, requests_used, requests_limit, created_at, expires_at, rotated_from, rotated_at, grace_period_ends FROM api_keys WHERE key_hash = $1 AND (expires_at IS NULL OR expires_at > NOW() OR grace_period_ends > NOW())",
        &[&key_hash],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let key = match row {
        Some(row) => ApiKey {
            id: row.get("id"),
            user_id: row.get("user_id"),
            key_hash: row.get("key_hash"),
            name: row.get("name"),
            tier: Tier::from_str(row.get::<_, String>("tier")),
            requests_used: row.get("requests_used"),
            requests_limit: row.get("requests_limit"),
            created_at: row.get("created_at"),
            expires_at: row.get("expires_at"),
            rotated_from: row.get("rotated_from"),
            rotated_at: row.get("rotated_at"),
            grace_period_ends: row.get("grace_period_ends"),
        },
        None => return Err(ApiError::InvalidApiKey),
    };

    if key.is_limit_exceeded() {
        return Err(ApiError::RateLimited {
            limit: key.requests_limit as u32,
            reset_at: Utc::now() + chrono::Duration::days(1),
        });
    }

    req.extensions_mut().insert(key);

    Ok(next.run(req).await)
}

pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn generate_api_key() -> String {
    let bytes: [u8; 32] = rand::random();
    hex::encode(bytes)
}

pub async fn create_api_key_for_user(
    state: &AppState,
    user_id: uuid::Uuid,
    tier: Tier,
    name: Option<String>,
) -> Result<(String, uuid::Uuid)> {
    let api_key = generate_api_key();
    let key_hash = hash_api_key(&api_key);
    let key_id = uuid::Uuid::new_v4();
    let requests_limit = tier.requests_limit();

    let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    conn.execute(
        "INSERT INTO api_keys (id, user_id, key_hash, name, tier, requests_used, requests_limit, created_at) VALUES ($1, $2, $3, NULLIF($4, ''), $5, 0, $6, NOW())",
        &[&key_id.to_string(), &user_id.to_string(), &key_hash, &name, &tier.to_string(), &requests_limit],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    Ok((api_key, key_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_api_key() {
        let key = "test_key_123";
        let hash = hash_api_key(key);
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_api_key() {
        let key = generate_api_key();
        assert_eq!(key.len(), 64);
        let hash = hash_api_key(&key);
        assert_eq!(hash.len(), 64);
    }
}
