//! Rairos API Gateway
//!
//! API Gateway for Rairos API-first commercial platform.
//! Provides authentication, rate limiting, and API routing.

pub mod auth;
pub mod error;
pub mod metrics;
pub mod models;
pub mod openapi;
pub mod ratelimit;
pub mod routes;
pub mod state;
pub mod stripe;
pub mod webhook;

pub use error::{ApiError, Result};
pub use state::AppState;

use axum::Router;

pub fn create_app(state: AppState) -> Router {
    routes::create_api_router(state)
}
