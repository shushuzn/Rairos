//! API Routes

use axum::{
    extract::{Path, State, Query},
    middleware::from_fn_with_state,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;
use utoipa::OpenApi;

use crate::auth::{auth_middleware, create_api_key_for_user, generate_api_key, hash_api_key};
use crate::error::{ApiError, Result};
use crate::models::{
    ApiKey, ApiKeyResponse, AuthResponse, CreateKeyRequest, DailyUsage,
    EndpointUsage, LoginRequest, PaginationParams, RegisterRequest,
    RotateKeyRequest, RotateKeyResponse, Tier, UsageDashboard, UsageResponse,
};
use crate::state::AppState;

fn cors_layer() -> CorsLayer {
    let allowed_origins_str = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "https://rairos.ai,https://www.rairos.ai".to_string());

    let origins: Vec<String> = allowed_origins_str
        .split(',')
        .map(|o| o.trim().to_string())
        .filter(|o| !o.is_empty())
        .collect();

    let layer = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any)
        .max_age(std::time::Duration::from_secs(86400));

    if origins.iter().any(|o| o == "*") {
        layer.allow_origin(Any)
    } else {
        let origins_static: Vec<String> = origins.clone();
        let allow_origin = tower_http::cors::AllowOrigin::predicate(move |origin, _| {
            origins_static.iter().any(|o| {
                origin.to_str().map(|s| s == o.as_str()).unwrap_or(false)
            })
        });
        layer.allow_origin(allow_origin)
    }
}

pub fn create_api_router(state: AppState) -> Router {
    Router::new()
        .layer(cors_layer())
        .route("/health", get(health_check))
        .route("/metrics", get(metrics))
        .route("/docs/openapi.json", get(openapi_json))
        .route("/docs", get(serve_swagger_ui))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/keys", post(create_api_key).layer(from_fn_with_state(state.clone(), auth_middleware)))
        .route("/keys", get(list_api_keys).layer(from_fn_with_state(state.clone(), auth_middleware)))
        .route("/keys/rotate", post(rotate_api_key).layer(from_fn_with_state(state.clone(), auth_middleware)))
        .route("/usage", get(get_usage).layer(from_fn_with_state(state.clone(), auth_middleware)))
        .route("/usage/dashboard", get(get_usage_dashboard).layer(from_fn_with_state(state.clone(), auth_middleware)))
        .route("/papers/search", get(search_papers).layer(from_fn_with_state(state.clone(), auth_middleware)))
        .route("/papers/:id", get(get_paper).layer(from_fn_with_state(state.clone(), auth_middleware)))
        .route("/gap/detect", post(detect_gap).layer(from_fn_with_state(state.clone(), auth_middleware)))
        .route("/research/run", post(run_research).layer(from_fn_with_state(state.clone(), auth_middleware)))
        .route("/subscription/checkout", post(create_checkout).layer(from_fn_with_state(state.clone(), auth_middleware)))
        .route("/subscription/portal", post(create_portal).layer(from_fn_with_state(state.clone(), auth_middleware)))
        .route("/subscription/webhook", post(stripe_webhook))
        .route("/subscription/status", get(get_subscription_status).layer(from_fn_with_state(state.clone(), auth_middleware)))
        .route("/tiers", get(get_tiers))
        .with_state(state)
}

pub async fn openapi_json() -> impl axum::response::IntoResponse {
    let openapi = crate::openapi::ApiDoc::openapi();
    let json = serde_json::to_string(&openapi).unwrap_or_default();
    ([(axum::http::header::CONTENT_TYPE, "application/json")], json)
}

pub async fn serve_swagger_ui() -> impl axum::response::IntoResponse {
    let html = include_str!("../swagger_ui.html").to_string();
    axum::response::Html(html)
}

pub async fn health_check() -> impl axum::response::IntoResponse {
    ([(axum::http::header::CONTENT_TYPE, "application/json")], r#"{"status":"ok"}"#)
}

pub async fn metrics(State(state): State<AppState>) -> impl axum::response::IntoResponse {
    let output = state.metrics.export_prometheus();
    ([(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")], output)
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

    let (api_key, _key_id) = create_api_key_for_user(&state, user_id, Tier::Free, None).await?;

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
    let tier = Tier::from_str(&tier_str);

    let (api_key, _key_id) = create_api_key_for_user(&state, user_id, tier, None).await?;

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
            tier: Tier::from_str(row.get::<_, String>("tier")),
            requests_used: row.get("requests_used"),
            requests_limit: row.get("requests_limit"),
            created_at: row.get("created_at"),
            expires_at: row.get("expires_at"),
        })
        .collect();

    Ok(Json(keys))
}

pub async fn rotate_api_key(
    State(state): State<AppState>,
    Extension(key): Extension<ApiKey>,
    Json(req): Json<RotateKeyRequest>,
) -> Result<impl axum::response::IntoResponse> {
    let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let _old_key_row = conn.query_opt(
        "SELECT id, user_id, tier FROM api_keys WHERE id = $1 AND user_id = $2",
        &[&req.key_id.to_string(), &key.user_id.to_string()],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?
    .ok_or_else(|| ApiError::NotFound("API key not found".to_string()))?;

    let new_api_key = generate_api_key();
    let new_key_hash = hash_api_key(&new_api_key);
    let new_key_id = Uuid::new_v4();
    let grace_period_ends = Utc::now() + chrono::Duration::hours(req.grace_period_hours);

    let new_key_id_str = new_key_id.to_string();
    let user_id_str = key.user_id.to_string();
    let old_key_id_str = req.key_id.to_string();
    let tier_str = key.tier.to_string();
    let requests_limit = get_tier_limit(key.tier);

    use tokio_postgres::types::ToSql;
    let params: Vec<&(dyn ToSql + Sync)> = vec![
        &new_key_id_str as &(dyn ToSql + Sync),
        &user_id_str as &(dyn ToSql + Sync),
        &new_key_hash as &(dyn ToSql + Sync),
        &tier_str as &(dyn ToSql + Sync),
        &requests_limit as &(dyn ToSql + Sync),
        &old_key_id_str as &(dyn ToSql + Sync),
        &grace_period_ends as &(dyn ToSql + Sync),
    ];

    conn.execute(
        "INSERT INTO api_keys (id, user_id, key_hash, tier, requests_used, requests_limit, created_at, rotated_from, rotated_at, grace_period_ends) VALUES ($1, $2, $3, $4, 0, $5, NOW(), $6, NOW(), $7)",
        &params,
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    conn.execute(
        "UPDATE api_keys SET expires_at = $1 WHERE id = $2",
        &[&grace_period_ends, &req.key_id.to_string()],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let response = RotateKeyResponse {
        new_key: new_api_key,
        new_key_id,
        old_key_id: req.key_id,
        grace_period_ends,
        message: format!("Key rotated successfully. Old key expires at {}.", grace_period_ends),
    };

    Ok(Json(response))
}

pub async fn get_usage(
    State(_state): State<AppState>,
    Extension(key): Extension<ApiKey>,
) -> Result<impl axum::response::IntoResponse> {
    let usage = UsageResponse {
        tier: key.tier,
        requests_used: key.requests_used,
        requests_limit: key.requests_limit,
        requests_remaining: ((key.requests_limit - key.requests_used).max(0)),
        reset_at: Utc::now() + chrono::Duration::hours(24),
    };

    Ok(Json(usage))
}

pub async fn get_usage_dashboard(
    State(state): State<AppState>,
    Extension(key): Extension<ApiKey>,
) -> Result<impl axum::response::IntoResponse> {
    let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let _total_requests: i64 = conn.query_one(
        "SELECT COALESCE(SUM(requests_used), 0) as total FROM api_keys WHERE user_id = $1",
        &[&key.user_id.to_string()],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?
    .get("total");

    let requests_today: i64 = conn.query_one(
        "SELECT COUNT(*) as count FROM usage_events WHERE api_key_id = $1 AND created_at >= NOW() - INTERVAL '1 day'",
        &[&key.id.to_string()],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?
    .get("count");

    let requests_this_week: i64 = conn.query_one(
        "SELECT COUNT(*) as count FROM usage_events WHERE api_key_id = $1 AND created_at >= NOW() - INTERVAL '7 days'",
        &[&key.id.to_string()],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?
    .get("count");

    let requests_this_month: i64 = conn.query_one(
        "SELECT COUNT(*) as count FROM usage_events WHERE api_key_id = $1 AND created_at >= NOW() - INTERVAL '30 days'",
        &[&key.id.to_string()],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?
    .get("count");

    let endpoint_rows = conn.query(
        "SELECT endpoint, COUNT(*) as count, COALESCE(AVG(latency_ms), 0) as avg_latency, MAX(created_at) as last_called FROM usage_events WHERE api_key_id = $1 GROUP BY endpoint ORDER BY count DESC LIMIT 10",
        &[&key.id.to_string()],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let endpoint_breakdown: Vec<EndpointUsage> = endpoint_rows.iter().map(|row| {
        EndpointUsage {
            endpoint: row.get("endpoint"),
            count: row.get("count"),
            avg_latency_ms: row.get("avg_latency"),
            last_called: row.get("last_called"),
        }
    }).collect();

    let daily_rows = conn.query(
        "SELECT DATE(created_at) as date, COUNT(*) as count FROM usage_events WHERE api_key_id = $1 AND created_at >= NOW() - INTERVAL '30 days' GROUP BY DATE(created_at) ORDER BY date",
        &[&key.id.to_string()],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let daily_trend: Vec<DailyUsage> = daily_rows.iter().map(|row| {
        let date: chrono::NaiveDate = row.get("date");
        DailyUsage {
            date: date.to_string(),
            count: row.get("count"),
        }
    }).collect();

    let usage_percent = if key.requests_limit > 0 {
        (key.requests_used as f64 / key.requests_limit as f64) * 100.0
    } else {
        0.0
    };

    let dashboard = UsageDashboard {
        total_requests: key.requests_used,
        requests_today,
        requests_this_week,
        requests_this_month,
        limit: key.requests_limit,
        remaining: ((key.requests_limit - key.requests_used).max(0)),
        usage_percent,
        tier: key.tier,
        reset_at: Utc::now() + chrono::Duration::hours(24),
        endpoint_breakdown,
        daily_trend,
    };

    Ok(Json(dashboard))
}

pub async fn search_papers(
    State(state): State<AppState>,
    Extension(key): Extension<ApiKey>,
    Query(params): Query<PaginationParams>,
) -> Result<impl axum::response::IntoResponse> {
    require_tier(key.tier, Tier::Free)?;

    let offset = (params.page - 1) * params.per_page;
    let search_query = params.q.as_deref().filter(|s| !s.is_empty());

    let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let (count_row, rows) = if let Some(query) = search_query {
        let search_pattern = format!("%{}%", query);
        let rows = conn.query(
            "SELECT id, title, abstract, authors, categories, published FROM papers WHERE title ILIKE $1 OR abstract ILIKE $1 ORDER BY published DESC LIMIT $2 OFFSET $3",
            &[&search_pattern, &(params.per_page as i64), &(offset as i64)],
        ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let count_row = conn.query_one(
            "SELECT COUNT(*) as count FROM papers WHERE title ILIKE $1 OR abstract ILIKE $1",
            &[&search_pattern],
        ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        (count_row, rows)
    } else {
        let rows = conn.query(
            "SELECT id, title, abstract, authors, categories, published FROM papers ORDER BY published DESC LIMIT $1 OFFSET $2",
            &[&(params.per_page as i64), &(offset as i64)],
        ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let count_row = conn.query_one(
            "SELECT COUNT(*) as count FROM papers",
            &[],
        ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        (count_row, rows)
    };

    let total: i64 = count_row.get("count");

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

    state.metrics.record_request("/papers/search", &key.tier.to_string());

    Ok(Json(serde_json::json!({
        "papers": papers,
        "page": params.page,
        "per_page": params.per_page,
        "total": total
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
    State(state): State<AppState>,
    Extension(key): Extension<ApiKey>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl axum::response::IntoResponse> {
    require_tier(key.tier, Tier::Pro)?;

    let topic = payload
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if topic.is_empty() {
        return Err(ApiError::ValidationError("query is required".to_string()));
    }

    let gaps = detect_gap_impl(topic, &state.db).await?;

    state.metrics.record_request("/gap/detect", &key.tier.to_string());

    Ok(Json(serde_json::json!({
        "query": topic,
        "gaps_found": gaps.len(),
        "gaps": gaps
    })))
}

pub async fn run_research(
    State(state): State<AppState>,
    Extension(key): Extension<ApiKey>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl axum::response::IntoResponse> {
    require_tier(key.tier, Tier::Team)?;

    let query = payload
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if query.is_empty() {
        return Err(ApiError::ValidationError("query is required".to_string()));
    }

    let gaps = detect_gap_impl(query, &state.db).await?;

    state.metrics.record_request("/research/run", &key.tier.to_string());

    Ok(Json(serde_json::json!({
        "query": query,
        "status": "completed",
        "gaps_found": gaps.len(),
        "gaps": gaps,
        "papers_analyzed": 0,
        "next_steps": [
            "Review identified gaps",
            "Select gap for deeper analysis",
            "Run detailed research on selected gap"
        ]
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

fn get_tier_limit(tier: Tier) -> i64 {
    tier.requests_limit()
}

pub async fn create_checkout(
    State(state): State<AppState>,
    Extension(key): Extension<ApiKey>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl axum::response::IntoResponse> {
    let price_id = payload
        .get("price_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::ValidationError("price_id is required".to_string()))?;

    let success_url = payload
        .get("success_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://rairos.ai/dashboard?success=true");

    let cancel_url = payload
        .get("cancel_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://rairos.ai/pricing?cancelled=true");

    let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let row = conn.query_opt(
        "SELECT stripe_customer_id FROM users WHERE id = $1",
        &[&key.user_id.to_string()],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let customer_id: Option<String> = row.and_then(|r| r.get("stripe_customer_id"));

    let checkout_url = format!(
        "https://checkout.stripe.com/checkout/{}?customer={}&price={}&success={}&cancel={}",
        uuid::Uuid::new_v4(),
        customer_id.unwrap_or_else(|| "new".to_string()),
        price_id,
        success_url,
        cancel_url
    );

    let session_id = uuid::Uuid::new_v4().to_string();

    Ok(Json(serde_json::json!({
        "checkout_url": checkout_url,
        "session_id": session_id
    })))
}

pub async fn create_portal(
    State(state): State<AppState>,
    Extension(key): Extension<ApiKey>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl axum::response::IntoResponse> {
    let return_url = payload
        .get("return_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://rairos.ai/dashboard");

    let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let row = conn.query_opt(
        "SELECT stripe_customer_id FROM users WHERE id = $1",
        &[&key.user_id.to_string()],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let customer_id: String = match row {
        Some(r) => match r.get::<_, Option<String>>("stripe_customer_id") {
            Some(cid) => cid,
            None => return Err(ApiError::ValidationError("No Stripe customer found".to_string())),
        },
        None => return Err(ApiError::ValidationError("User not found".to_string())),
    };

    let portal_url = format!(
        "https://billing.stripe.com/session/{}?return={}",
        customer_id,
        return_url
    );

    Ok(Json(serde_json::json!({
        "portal_url": portal_url
    })))
}

pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Body,
) -> Result<impl axum::response::IntoResponse> {
    let signature_header = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let raw_bytes = axum::body::to_bytes(body, 10_000_000)
        .await
        .map_err(|e| ApiError::ValidationError(format!("Failed to read body: {}", e)))?;

    let raw_body = String::from_utf8(raw_bytes.to_vec())
        .map_err(|e| ApiError::ValidationError(format!("Invalid UTF-8: {}", e)))?;

    if let Some(ref secret) = state.stripe_webhook_secret {
        crate::webhook::verify_stripe_signature(&raw_body, signature_header, secret)
            .map_err(|e| {
                tracing::warn!("Stripe signature verification failed: {}", e);
                ApiError::Unauthorized
            })?;
    }

    let payload: serde_json::Value = serde_json::from_str(&raw_body)
        .map_err(|e| ApiError::ValidationError(format!("Invalid JSON: {}", e)))?;

    let event_type = payload
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    tracing::info!("Received Stripe webhook: {}", event_type);

    let event_id = payload
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let existing = conn.query_opt(
        "SELECT id FROM webhook_events WHERE event_id = $1",
        &[&event_id],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    if existing.is_some() {
        tracing::debug!("Duplicate webhook event: {}", event_id);
        return Ok(Json(serde_json::json!({ "received": true, "duplicate": true })));
    }

    match event_type {
        "checkout.session.completed" => {
            if let Some(data) = payload.get("data").and_then(|d| d.get("object")) {
                if let Some(session) = extract_checkout_session_data(data) {
                    tracing::info!("Checkout completed for customer {} subscription {}",
                        session.customer_id, session.subscription_id);

                    if let Some(uid) = session.user_id {
                        conn.execute(
                            "UPDATE users SET stripe_customer_id = $1, stripe_subscription_id = $2, tier = $3 WHERE id = $4",
                            &[&session.customer_id, &session.subscription_id, &session.tier, &uid],
                        ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

                        tracing::info!("Updated user {} to tier {} via checkout", uid, session.tier);
                    }
                }
            }
        }
        "customer.subscription.created" | "customer.subscription.updated" => {
            if let Some(data) = payload.get("data").and_then(|d| d.get("object")) {
                if let Some(sub) = extract_subscription_data(data) {
                    tracing::info!("Subscription {} (customer {}) status: {}",
                        sub.subscription_id, sub.customer_id, sub.status);

                    conn.execute(
                        "UPDATE users SET tier = $1 WHERE stripe_customer_id = $2",
                        &[&sub.tier, &sub.customer_id],
                    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

                    tracing::info!("Updated customer {} to tier {} via subscription update",
                        sub.customer_id, sub.tier);
                }
            }
        }
        "customer.subscription.deleted" => {
            if let Some(data) = payload.get("data").and_then(|d| d.get("object")) {
                if let Some(customer_id) = data.get("customer").and_then(|v| v.as_str()) {
                    tracing::info!("Subscription deleted for customer {}", customer_id);

                    conn.execute(
                        "UPDATE users SET tier = 'free', stripe_subscription_id = NULL WHERE stripe_customer_id = $1",
                        &[&customer_id],
                    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

                    tracing::info!("Downgraded customer {} to free tier", customer_id);
                }
            }
        }
        "invoice.payment_failed" => {
            if let Some(data) = payload.get("data").and_then(|d| d.get("object")) {
                if let Some(customer_id) = data.get("customer").and_then(|v| v.as_str()) {
                    tracing::warn!("Payment failed for customer {}", customer_id);

                    conn.execute(
                        "UPDATE users SET tier = 'free' WHERE stripe_customer_id = $1 AND tier != 'free'",
                        &[&customer_id],
                    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;
                }
            }
        }
        _ => {
            tracing::debug!("Unhandled event type: {}", event_type);
        }
    }

    conn.execute(
        "INSERT INTO webhook_events (event_id, event_type, processed_at) VALUES ($1, $2, NOW())",
        &[&event_id, &event_type],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "received": true
    })))
}

pub async fn get_subscription_status(
    State(state): State<AppState>,
    Extension(key): Extension<ApiKey>,
) -> Result<impl axum::response::IntoResponse> {
    let conn = state.db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let row = conn.query_opt(
        "SELECT tier, stripe_customer_id FROM users WHERE id = $1",
        &[&key.user_id.to_string()],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    match row {
        Some(row) => {
            let tier_str: String = row.get("tier");
            let stripe_customer_id: Option<String> = row.get("stripe_customer_id");

            Ok(Json(serde_json::json!({
                "tier": tier_str,
                "stripe_customer_id": stripe_customer_id,
                "subscription_active": stripe_customer_id.is_some()
            })))
        }
        None => Err(ApiError::NotFound("User not found".to_string())),
    }
}

pub async fn get_tiers() -> impl axum::response::IntoResponse {
    Json(serde_json::json!({
        "tiers": crate::stripe::get_subscription_tiers()
    }))
}

use axum::extract::Extension;

const GAP_PATTERNS: &[(&str, &str, &[&str])] = &[
    ("method_limitation", "Method Limitation", &["limitation", "drawback", "however", "not suitable", "not efficient", "poor performance", "bottleneck"]),
    ("unexplored_application", "Unexplored Application", &["future work", "open question", "not explore", "remains unexplored", "left for future"]),
    ("contradiction", "Contradiction", &["inconsistent", "contradict", "debate", "conflicting", "mixed results"]),
    ("evaluation_gap", "Evaluation Gap", &["no benchmark", "lack evaluation", "not compare", "no standard", "not evaluated"]),
    ("scalability_issue", "Scalability Issue", &["scalab", "large scale", "not scalable", "computational cost"]),
    ("theoretical_gap", "Theoretical Gap", &["theoretical", "lack formal", "no theory"]),
    ("dataset_gap", "Dataset Gap", &["dataset lack", "no data", "limited data"]),
    ("generalization_gap", "Generalization Gap", &["generaliz", "transfer", "domain adapt"]),
];

fn detect_gaps_from_text(text: &str, topic: &str) -> Vec<serde_json::Value> {
    let text_lower = text.to_lowercase();
    let topic_lower = topic.to_lowercase();
    let mut found_gaps: Vec<serde_json::Value> = Vec::new();

    for (gap_type, label, patterns) in GAP_PATTERNS {
        for pattern in *patterns {
            if text_lower.contains(&pattern.to_lowercase()) {
                found_gaps.push(serde_json::json!({
                    "gap_type": gap_type,
                    "label": label,
                    "evidence": format!("Found '{}' in context of {}", pattern, topic),
                    "topic": topic,
                    "severity": "medium"
                }));
                break;
            }
        }
    }

    if text_lower.contains(&topic_lower) && found_gaps.is_empty() {
        found_gaps.push(serde_json::json!({
            "gap_type": "research_opportunity",
            "label": "Research Opportunity",
            "evidence": format!("{} mentioned but no specific gaps detected", topic),
            "topic": topic,
            "severity": "low"
        }));
    }

    found_gaps
}

pub async fn detect_gap_impl(
    topic: &str,
    db: &bb8::Pool<bb8_postgres::PostgresConnectionManager<tokio_postgres::NoTls>>,
) -> std::result::Result<Vec<serde_json::Value>, ApiError> {
    let conn = db.get().await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let rows = conn.query(
        "SELECT title, abstract FROM papers WHERE title ILIKE $1 OR abstract ILIKE $1 LIMIT 50",
        &[&format!("%{}%", topic)],
    ).await.map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let mut all_gaps: Vec<serde_json::Value> = Vec::new();

    for row in rows.iter() {
        let title: String = row.get("title");
        let abstract_text: String = row.get("abstract");
        let combined = format!("{} {}", title, abstract_text);

        let gaps = detect_gaps_from_text(&combined, topic);
        all_gaps.extend(gaps);
    }

    all_gaps.truncate(10);
    Ok(all_gaps)
}

struct CheckoutSessionData {
    customer_id: String,
    subscription_id: String,
    user_id: Option<String>,
    tier: String,
}

fn extract_checkout_session_data(data: &serde_json::Value) -> Option<CheckoutSessionData> {
    let customer_id = data.get("customer")?.as_str()?.to_string();
    let subscription_id = data.get("subscription")?.as_str()?.to_string();
    let user_id = data
        .get("metadata")?
        .get("user_id")?
        .as_str()?
        .to_string()
        .into();
    let tier = crate::stripe::get_tier_by_checkout_session(data)
        .map(|t| t.name.to_string())
        .unwrap_or_else(|| "free".to_string());

    Some(CheckoutSessionData {
        customer_id,
        subscription_id,
        user_id,
        tier,
    })
}

struct SubscriptionData {
    subscription_id: String,
    customer_id: String,
    status: String,
    tier: String,
}

fn extract_subscription_data(data: &serde_json::Value) -> Option<SubscriptionData> {
    let subscription_id = data.get("id")?.as_str()?.to_string();
    let customer_id = data.get("customer")?.as_str()?.to_string();
    let status = data.get("status")?.as_str()?.to_string();

    let tier = if status == "active" || status == "trialing" {
        data.get("items")
            .and_then(|items| items.get("data"))
            .and_then(|items| items.as_array())
            .and_then(|items| items.first())
            .and_then(|item| item.get("price"))
            .and_then(|price| price.get("id"))
            .and_then(|id| id.as_str())
            .and_then(|price_id| crate::stripe::get_tier_by_price_id(price_id))
            .map(|t| t.name.to_string())
            .unwrap_or_else(|| "free".to_string())
    } else {
        "free".to_string()
    };

    Some(SubscriptionData {
        subscription_id,
        customer_id,
        status,
        tier,
    })
}

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

    #[test]
    fn test_extract_checkout_session_data_valid() {
        let json = serde_json::json!({
            "customer": "cus_123",
            "subscription": "sub_456",
            "metadata": {
                "user_id": "user_789"
            }
        });
        let result = extract_checkout_session_data(&json);
        assert!(result.is_some());
        let data = result.unwrap();
        assert_eq!(data.customer_id, "cus_123");
        assert_eq!(data.subscription_id, "sub_456");
        assert_eq!(data.user_id, Some("user_789".to_string()));
    }

    #[test]
    fn test_extract_checkout_session_data_missing_customer() {
        let json = serde_json::json!({
            "subscription": "sub_456",
            "metadata": {}
        });
        let result = extract_checkout_session_data(&json);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_checkout_session_data_missing_subscription() {
        let json = serde_json::json!({
            "customer": "cus_123",
            "metadata": {}
        });
        let result = extract_checkout_session_data(&json);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_checkout_session_data_missing_metadata() {
        let json = serde_json::json!({
            "customer": "cus_123",
            "subscription": "sub_456"
        });
        let result = extract_checkout_session_data(&json);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_subscription_data_valid_active() {
        let json = serde_json::json!({
            "id": "sub_123",
            "customer": "cus_456",
            "status": "active",
            "items": {
                "data": [{
                    "price": {
                        "id": "price_pro_monthly"
                    }
                }]
            }
        });
        let result = extract_subscription_data(&json);
        assert!(result.is_some());
        let data = result.unwrap();
        assert_eq!(data.subscription_id, "sub_123");
        assert_eq!(data.customer_id, "cus_456");
        assert_eq!(data.status, "active");
    }

    #[test]
    fn test_extract_subscription_data_missing_status() {
        let json = serde_json::json!({
            "id": "sub_123",
            "customer": "cus_456"
        });
        let result = extract_subscription_data(&json);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_subscription_data_past_due() {
        let json = serde_json::json!({
            "id": "sub_123",
            "customer": "cus_456",
            "status": "past_due"
        });
        let result = extract_subscription_data(&json);
        assert!(result.is_some());
        let data = result.unwrap();
        assert_eq!(data.status, "past_due");
        assert_eq!(data.tier, "free");
    }
}
