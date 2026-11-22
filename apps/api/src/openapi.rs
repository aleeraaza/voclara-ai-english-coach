use axum::Json;
use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use crate::{error, routes};

#[derive(OpenApi)]
#[openapi(
    info(title = "Voclara API", description = "Control plane for the Voclara speaking coach"),
    paths(
        routes::health::health,
        routes::health::ready,
        routes::auth::otp_send,
        routes::auth::otp_verify,
        routes::auth::google,
        routes::auth::refresh_tokens,
        routes::auth::logout,
        routes::me::get_me,
        routes::me::update_me,
        routes::sessions::start_session,
        routes::sessions::complete_session,
    ),
    components(schemas(
        routes::health::HealthResponse,
        routes::auth::OtpSendRequest,
        routes::auth::OtpSendResponse,
        routes::auth::OtpVerifyRequest,
        routes::auth::GoogleLoginRequest,
        routes::auth::RefreshRequest,
        routes::auth::TokenResponse,
        routes::me::MeResponse,
        routes::me::MinutesInfo,
        routes::me::StreakInfo,
        routes::me::UpdateMeRequest,
        routes::sessions::StartSessionRequest,
        routes::sessions::StartSessionResponse,
        routes::sessions::CompleteSessionRequest,
        routes::sessions::TranscriptTurn,
        routes::sessions::TurnRole,
        routes::sessions::CompleteSessionResponse,
        crate::providers::voice::Transport,
        error::ErrorBody,
        error::ErrorDetail,
    )),
    modifiers(&BearerAuth),
    tags(
        (name = "health", description = "Liveness and readiness"),
        (name = "auth", description = "Phone OTP, Google sign-in, token refresh"),
        (name = "me", description = "The signed-in user"),
        (name = "sessions", description = "Live speaking sessions")
    )
)]
pub struct ApiDoc;

/// Declares the `bearer` scheme that protected paths reference.
struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

pub async fn spec() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
