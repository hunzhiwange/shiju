//! An authorized saved provider and its immutable upstream target.
use super::{dispatch::JsonTarget, route_auth::VerifiedRouteBinding};
use serde_json::Value;

/// The transaction resolves the target from the same authorized scope and revision.
pub struct RouteTarget {
    pub messages_api_key: bool,
    pub binding: VerifiedRouteBinding,
    pub revision: String,
    pub provider_id: String,
    pub media_settings: Value,
    pub target: JsonTarget,
}

impl RouteTarget {
    /// Keep only the model catalog fields consumed by media capability resolution.
    pub fn media_settings_for(row: &Value) -> Value {
        let settings = &row["settingsConfig"];
        serde_json::json!({
            "modelCatalog": settings["modelCatalog"],
            "models": settings["models"],
        })
    }
}
