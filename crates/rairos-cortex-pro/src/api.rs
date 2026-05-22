//! HTTP API server for SparksMatter research workflow.
//!
//! This module provides an HTTP API to interact with the multi-agent
//! research workflow.
//!
//! # Endpoints
//!
//! - `POST /api/research` - Start a new research task
//! - `GET /api/research/{id}` - Get research status/results
//! - `GET /api/research/{id}/report` - Get research report
//! - `GET /api/health` - Health check
//!
//! # Run
//!
//! ```bash
//! cargo run --bin rairos-api --features "tools,api"
//! ```

use std::sync::Arc;
use std::collections::HashMap;
use std::net::SocketAddr;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Application state shared across requests
#[derive(Clone)]
struct AppState {
    /// Active research tasks
    tasks: Arc<RwLock<HashMap<String, ResearchTask>>>,
}

/// A research task
#[derive(Clone, Serialize, Deserialize)]
struct ResearchTask {
    /// Task ID
    id: String,
    /// Research query
    query: String,
    /// Current status
    status: TaskStatus,
    /// Hypothesis (if generated)
    hypothesis: Option<String>,
    /// Research plan (JSON)
    plan: Option<String>,
    /// Execution results
    execution_results: Option<String>,
    /// Final report
    report: Option<String>,
    /// Error message if failed
    error: Option<String>,
}

/// Task status
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Request to start a new research task
#[derive(Deserialize)]
struct CreateTaskRequest {
    /// Research query
    query: String,
}

/// Response for task creation
#[derive(Serialize)]
struct CreateTaskResponse {
    /// Task ID
    task_id: String,
    /// Status
    status: TaskStatus,
}

/// Response for task status
#[derive(Serialize)]
struct TaskStatusResponse {
    /// Task ID
    id: String,
    /// Research query
    query: String,
    /// Status
    status: TaskStatus,
    /// Hypothesis
    hypothesis: Option<String>,
    /// Plan steps count
    plan_steps: Option<usize>,
    /// Report available
    report_available: bool,
    /// Error
    error: Option<String>,
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

/// Create a new router with the given state
fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/research", post(create_research))
        .route("/api/research/{id}", get(get_research))
        .route("/api/research/{id}/report", get(get_report))
        .with_state(state)
}

/// Health check handler
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Create a new research task
async fn create_research(
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<CreateTaskResponse>, StatusCode> {
    let task_id = Uuid::new_v4().to_string();

    let task = ResearchTask {
        id: task_id.clone(),
        query: req.query.clone(),
        status: TaskStatus::Pending,
        hypothesis: None,
        plan: None,
        execution_results: None,
        report: None,
        error: None,
    };

    state.tasks.write().await.insert(task_id.clone(), task);

    // In a full implementation, we would start the research workflow here
    // using tokio::spawn

    Ok(Json(CreateTaskResponse {
        task_id,
        status: TaskStatus::Pending,
    }))
}

/// Get research task status
async fn get_research(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TaskStatusResponse>, StatusCode> {
    let tasks = state.tasks.read().await;

    let task = tasks.get(&id).ok_or(StatusCode::NOT_FOUND)?;

    let plan_steps;
    if let Some(ref p) = task.plan {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(p) {
            plan_steps = v.get("steps")
                .and_then(|s| s.as_array())
                .map(|arr| arr.len());
        } else {
            plan_steps = None;
        }
    } else {
        plan_steps = None;
    }

    Ok(Json(TaskStatusResponse {
        id: task.id.clone(),
        query: task.query.clone(),
        status: task.status.clone(),
        hypothesis: task.hypothesis.clone(),
        plan_steps,
        report_available: task.report.is_some(),
        error: task.error.clone(),
    }))
}

/// Get research report
async fn get_report(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tasks = state.tasks.read().await;

    let task = tasks.get(&id).ok_or(StatusCode::NOT_FOUND)?;

    let report = task.report.clone().ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(serde_json::json!({
        "task_id": id,
        "report": report,
        "hypothesis": task.hypothesis,
        "plan": task.plan,
    })))
}

/// Start the API server
pub async fn start_server(addr: SocketAddr) {
    let state = AppState {
        tasks: Arc::new(RwLock::new(HashMap::new())),
    };

    let router = create_router(state);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("🚀 API server running on http://{}", addr);
    tracing::info!("📋 Endpoints:");
    tracing::info!("   POST /api/research    - Start research task");
    tracing::info!("   GET  /api/research/{{id}}    - Get task status");
    tracing::info!("   GET  /api/research/{{id}}/report - Get report");
    tracing::info!("   GET  /api/health              - Health check");

    axum::serve(listener, router).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health() {
        let response = health().await;
        assert_eq!(response.status, "ok");
    }

    #[tokio::test]
    async fn test_create_and_get_task() {
        let state = AppState {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        };

        let req = CreateTaskRequest {
            query: "Find thermoelectric materials".to_string(),
        };

        let response = create_research(State(state.clone()), Json(req)).await.unwrap();
        let task_id = response.task_id;

        let status = get_research(State(state), Path(task_id)).await.unwrap();
        assert_eq!(status.query, "Find thermoelectric materials");
    }
}
