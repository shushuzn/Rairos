//! API Routes

use axum::{
    extract::{Path, State, Query},
    middleware::from_fn_with_state,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use uuid::Uuid;

use crate::auth::{authMiddleware, generate_api_key, hash_api_key};
use crate::error::{ApiError, Result};
use crate::models::{
    ApiKey, ApiKeyResponse, AuthResponse, CreateKeyRequest, LoginRequest,
    PaginationParams, RegisterRequest, Tier, UsageResponse,
};
use crate::state::AppState;

pub fn create_api_router(state: AppState) -> Router {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/keys", post(create_api_key).layer(from_fn_with_state(state.clone(), authMiddleware)))
        .route("/keys", get(list_api_keys).layer(from_fn_with_state(state.clone(), authMiddleware)))
        .route("/usage", get(get_usage).layer(from_fn_with_state(state.clone(), authMiddleware)))
        .route("/papers/search", get(search_papers).layer(from_fn_with_state(state.clone(), authMiddleware)))
        .route("/papers/:id", get(get_paper).layer(from_fn_with_state(state.clone(), authMiddleware)))
        .route("/gap/detect", post(detect_gap).layer(from_fn_with_state(state.clone(), authMiddleware)))
        .route("/research/run", post(run_research).layer(from_fn_with_state(state.clone(), authMiddleware)))
        .with_state(state)
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl axum::response::IntoResponse> {
    if req.email.is_empty() || req.password.len() < 8 {
        return Err(ApiError::ValidationError(
            "Email must be valid and password must be at least 8 characters".to_string(),
        ));
    }

    let password_hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let user_id = Uuid::new_v4();

    let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    conn.execute(
        "INSERT INTO users (id, email, password_hash, tier, created_at) VALUES ($1, $2, $3, 'free', NOW())",
        &[&user_id.to_string(), &req.email, &password_hash],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let api_key = generate_api_key();
    let key_hash = hash_api_key(&api_key);

    let key_id = Uuid::new_v4();
    conn.execute(
        "INSERT INTO api_keys (id, user_id, key_hash, tier, requests_used, requests_limit, created_at) VALUES ($1, $2, $3, 'free', 0, 100, NOW())",
        &[&key_id.to_string(), &user_id.to_string(), &key_hash],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    Ok(Json(AuthResponse {
        user_id,
        email: req.email,
        api_key,
        tier: Tier::Free,
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<impl axum::response::IntoResponse> {
    let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let row = conn.query_opt(
        "SELECT id, email, password_hash, tier FROM users WHERE email = $1",
        &[&req.email],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let row = row.ok_or(ApiError::Unauthorized)?;

    let password_hash: String = row.get("password_hash");
    bcrypt::verify(&req.password, &password_hash)
        .map_err(|_| ApiError::Unauthorized)?;

    let user_id: Uuid = row.get("id");
    let tier_str: String = row.get("tier");
    let tier = parse_tier(tier_str);

    let api_key = generate_api_key();
    let key_hash = hash_api_key(&api_key);

    let key_id = Uuid::new_v4();
    let requests_limit = get_tier_limit(tier);

    conn.execute(
        "INSERT INTO api_keys (id, user_id, key_hash, tier, requests_used, requests_limit, created_at) VALUES ($1, $2, $3, $4, 0, $5, NOW())",
        &[&key_id.to_string(), &user_id.to_string(), &key_hash, &tier.to_string(), &requests_limit],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    Ok(Json(AuthResponse {
        user_id,
        email: req.email,
        api_key,
        tier,
    }))
}

pub async fn create_api_key(
    State(state): State<AppState>,
    Extension(key): Extension<ApiKey>,
    Json(req): Json<CreateKeyRequest>,
) -> Result<impl axum::response::IntoResponse> {
    let api_key = generate_api_key();
    let key_hash = hash_api_key(&api_key);
    let key_id = Uuid::new_v4();

    let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let name_str = req.name.clone().unwrap_or_default();
    let tier_str = key.tier.to_string();
    let requests_limit = get_tier_limit(key.tier);

    let key_id_str = key_id.to_string();
    let user_id_str = key.user_id.to_string();

    use tokio_postgres::types::ToSql;
    let params: Vec<&(dyn ToSql + Sync)> = vec![
        &key_id_str as &(dyn ToSql + Sync),
        &user_id_str as &(dyn ToSql + Sync),
        &key_hash as &(dyn ToSql + Sync),
        &name_str as &(dyn ToSql + Sync),
        &tier_str as &(dyn ToSql + Sync),
        &requests_limit as &(dyn ToSql + Sync),
    ];

    conn.execute(
        "INSERT INTO api_keys (id, user_id, key_hash, name, tier, requests_used, requests_limit, created_at) VALUES ($1, $2, $3, NULLIF($4, ''), $5, 0, $6, NOW())",
        &params,
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "id": key_id,
        "api_key": api_key,
        "name": req.name,
        "tier": key.tier,
    })))
}

pub async fn list_api_keys(
    State(state): State<AppState>,
    Extension(key): Extension<ApiKey>,
) -> Result<impl axum::response::IntoResponse> {
    let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let rows = conn.query(
        "SELECT id, user_id, key_hash, name, tier, requests_used, requests_limit, created_at, expires_at FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC",
        &[&key.user_id.to_string()],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let keys: Vec<ApiKeyResponse> = rows
        .iter()
        .map(|row| ApiKeyResponse {
            id: row.get("id"),
            name: row.get("name"),
            tier: parse_tier(row.get("tier")),
            requests_used: row.get("requests_used"),
            requests_limit: row.get("requests_limit"),
            created_at: row.get("created_at"),
            expires_at: row.get("expires_at"),
        })
        .collect();

    Ok(Json(keys))
}

pub async fn get_usage(
    State(_state): State<AppState>,
    Extension(key): Extension<ApiKey>,
) -> Result<impl axum::response::IntoResponse> {
    let usage = UsageResponse {
        tier: key.tier,
        requests_used: key.requests_used,
        requests_limit: key.requests_limit,
        requests_remaining: ((key.requests_limit - key.requests_used as i64).max(0)) as i64,
        reset_at: Utc::now() + chrono::Duration::hours(24),
    };

    Ok(Json(usage))
}

pub async fn search_papers(
    State(state): State<AppState>,
    Extension(key): Extension<ApiKey>,
    Query(params): Query<PaginationParams>,
) -> Result<impl axum::response::IntoResponse> {
    require_tier(key.tier, Tier::Free)?;

    let offset = (params.page - 1) * params.per_page;

    let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let rows = conn.query(
        "SELECT id, title, abstract, authors, categories, published FROM papers ORDER BY published DESC LIMIT $1 OFFSET $2",
        &[&(params.per_page as i64), &(offset as i64)],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let papers: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<_, uuid::Uuid>("id"),
                "title": row.get::<_, String>("title"),
                "abstract": row.get::<_, String>("abstract"),
                "authors": row.get::<_, String>("authors"),
                "categories": row.get::<_, String>("categories"),
                "published": row.get::<_, chrono::DateTime<chrono::Utc>>("published"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "papers": papers,
        "page": params.page,
        "per_page": params.per_page,
    })))
}

pub async fn get_paper(
    State(state): State<AppState>,
    Extension(key): Extension<ApiKey>,
    Path(id): Path<Uuid>,
) -> Result<impl axum::response::IntoResponse> {
    require_tier(key.tier, Tier::Free)?;

    let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let row = conn.query_opt(
        "SELECT id, title, abstract, authors, categories, published FROM papers WHERE id = $1",
        &[&id.to_string()],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    match row {
        Some(row) => Ok(Json(serde_json::json!({
            "id": row.get::<_, uuid::Uuid>("id"),
            "title": row.get::<_, String>("title"),
            "abstract": row.get::<_, String>("abstract"),
            "authors": row.get::<_, String>("authors"),
            "categories": row.get::<_, String>("categories"),
            "published": row.get::<_, chrono::DateTime<chrono::Utc>>("published"),
        }))),
        None => Err(ApiError::NotFound("Paper not found".to_string())),
    }
}

pub async fn detect_gap(
    State(_state): State<AppState>,
    Extension(key): Extension<ApiKey>,
    Json(_payload): Json<serde_json::Value>,
) -> Result<impl axum::response::IntoResponse> {
    require_tier(key.tier, Tier::Pro)?;

    Ok(Json(serde_json::json!({
        "status": "placeholder",
        "message": "Gap detection requires rairos-research integration"
    })))
}

pub async fn run_research(
    State(_state): State<AppState>,
    Extension(key): Extension<ApiKey>,
    Json(_payload): Json<serde_json::Value>,
) -> Result<impl axum::response::IntoResponse> {
    require_tier(key.tier, Tier::Team)?;

    Ok(Json(serde_json::json!({
        "status": "placeholder",
        "message": "Research execution requires rairos-research integration"
    })))
}

fn require_tier(current: Tier, required: Tier) -> Result<()> {
    let hierarchy = [Tier::Free, Tier::Pro, Tier::Team, Tier::Enterprise];
    let current_idx = hierarchy.iter().position(|t| *t == current).unwrap_or(0);
    let required_idx = hierarchy.iter().position(|t| *t == required).unwrap_or(0);

    if current_idx >= required_idx {
        Ok(())
    } else {
        Err(ApiError::Forbidden(format!(
            "This endpoint requires {} tier or higher",
            required
        )))
    }
}

fn parse_tier(s: String) -> Tier {
    match s.as_str() {
        "pro" => Tier::Pro,
        "team" => Tier::Team,
        "enterprise" => Tier::Enterprise,
        _ => Tier::Free,
    }
}

fn get_tier_limit(tier: Tier) -> i64 {
    match tier {
        Tier::Free => 100,
        Tier::Pro => 10_000,
        Tier::Team => 100_000,
        Tier::Enterprise => i64::MAX,
    }
}

use axum::extract::Extension;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_require_tier() {
        assert!(require_tier(Tier::Pro, Tier::Free).is_ok());
        assert!(require_tier(Tier::Pro, Tier::Pro).is_ok());
        assert!(require_tier(Tier::Free, Tier::Pro).is_err());
        assert!(require_tier(Tier::Enterprise, Tier::Team).is_ok());
    }

    #[test]
    fn test_get_tier_limit() {
        assert_eq!(get_tier_limit(Tier::Free), 100);
        assert_eq!(get_tier_limit(Tier::Pro), 10_000);
        assert_eq!(get_tier_limit(Tier::Team), 100_000);
    }
}
