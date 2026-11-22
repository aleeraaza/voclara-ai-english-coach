use axum::Json;
use utoipa::OpenApi;

use crate::{error, routes};

#[derive(OpenApi)]
#[openapi(
    info(title = "Voclara API", description = "Control plane for the Voclara speaking coach"),
    paths(routes::health::health, routes::health::ready),
    components(schemas(
        routes::health::HealthResponse,
        error::ErrorBody,
        error::ErrorDetail
    )),
    tags((name = "health", description = "Liveness and readiness"))
)]
pub struct ApiDoc;

pub async fn spec() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
