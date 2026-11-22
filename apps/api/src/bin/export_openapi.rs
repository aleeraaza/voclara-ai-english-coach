//! Write the OpenAPI spec to a file, so TypeScript types can be generated
//! without running the server:
//!
//!     cargo run -p voclara-api --bin export_openapi -- packages/api-types/openapi.json

use anyhow::Context;
use utoipa::OpenApi;
use voclara_api::openapi::ApiDoc;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "packages/api-types/openapi.json".to_owned());
    let json = ApiDoc::openapi()
        .to_pretty_json()
        .context("serialising OpenAPI spec")?;
    std::fs::write(&path, format!("{json}\n")).with_context(|| format!("writing {path}"))?;
    println!("wrote {path}");
    Ok(())
}
