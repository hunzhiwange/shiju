//! Apply payload policy using explicit model capability evidence.
use serde_json::Value;

/// Unknown capabilities leave the payload intact; model-name guesses are disabled.
pub fn prepare(body: &mut Value, settings: &Value, model: &str) -> usize {
    let metadata = vw_api_types::model_catalog::ModelMetadataDto::default();
    let capability = crate::capabilities::resolve_model_capabilities(
        model, Some(settings), &metadata, false,
    );
    if capability.image_input != vw_api_types::model_catalog::ModelCapabilityState::Unsupported {
        return 0;
    }
    app_routing_policy::media_sanitize::replace(body)
}
