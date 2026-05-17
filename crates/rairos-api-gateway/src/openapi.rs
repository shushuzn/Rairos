//! OpenAPI Documentation
//!
//! Generates OpenAPI specification for the Rairos API.

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    components(
        schemas(
            crate::models::Tier,
            crate::models::ApiKeyResponse,
            crate::models::UsageResponse,
            crate::models::UsageDashboard,
            crate::models::EndpointUsage,
            crate::models::DailyUsage,
            crate::models::RotateKeyResponse,
            crate::models::RegisterRequest,
            crate::models::LoginRequest,
            crate::models::CreateKeyRequest,
            crate::models::RotateKeyRequest,
            crate::models::AuthResponse,
            crate::models::PaginationParams,
        )
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "keys", description = "API Key management"),
        (name = "usage", description = "Usage statistics"),
        (name = "papers", description = "Paper search and retrieval"),
        (name = "research", description = "Research and gap detection"),
        (name = "subscription", description = "Stripe subscription management"),
        (name = "docs", description = "API documentation")
    ),
    info(
        title = "Rairos API",
        version = "1.0.0",
        description = "API for the Rairos research platform",
        contact(
            name = "Rairos Support",
            email = "api@rairos.ai"
        )
    )
)]
pub struct ApiDoc;
