//! Resolve model capabilities before applying the payload policy.
use serde_json::Value;

pub use app_routing_policy::media_sanitize::{UNSUPPORTED_IMAGE_MARKER, repair};

/// Unknown capabilities leave the payload intact; model-name guesses are disabled.
pub fn prepare(body: &mut Value, settings: &Value, model: &str) -> usize {
    let metadata = vw_api_types::model_catalog::ModelMetadataDto::default();
    let capability = crate::models::capabilities::resolve_model_capabilities(
        model, Some(settings), &metadata, false,
    );
    if capability.image_input != vw_api_types::model_catalog::ModelCapabilityState::Unsupported {
        return 0;
    }
    app_routing_policy::media_sanitize::replace(body)
}
