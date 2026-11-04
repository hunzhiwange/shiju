//! Minimal fixed-version Kiro control plane. Model traffic uses the existing Responses boundary.
use axum::http::StatusCode;
use serde_json::{Value, json};

pub(super) fn control(target: &str, body: &Value, model: &str) -> Result<Value, StatusCode> {
    match target {
        "KiroControlPlaneBearerService.ListAvailableModels" => Ok(json!({
            "models":[{"modelId":model,"modelName":model,"modelProvider":"session"}],
            "defaultModel":{"modelId":model}
        })),
        "KiroRuntimeService.GetFeatureConfiguration" => {
            // This wire key and value were verified against the bundled 3.0.0 engine.
            // Select its native Responses implementation; do not emulate model inference.
            use sha2::Digest;
            let key = hex::encode(sha2::Sha256::digest(
                b"kiro-feature-config-key-salt-3e9b1d7acreate_response_api",
            ));
            Ok(json!({"configuration":{key:"create_response"}}))
        }
        // There is no Kiro account or hosted tool service in a session-model binding.
        // Empty optional account metadata conveys no vendor entitlements or policy grants.
        "KiroControlPlaneBearerService.GetProfile"
        | "KiroControlPlaneBearerService.GetEffectiveEnterpriseManagementResourceWithToken" => {
            Ok(json!({}))
        }
        "KiroRuntimeService.InvokeMCP" if body["method"] == "tools/list" => {
            Ok(json!({"jsonrpc":"2.0","id":body["id"],"result":{"tools":[]}}))
        }
        _ => Err(StatusCode::UNPROCESSABLE_ENTITY),
    }
}

pub(super) fn responses(body: &mut Value) -> Result<(), StatusCode> {
    let object = body.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?;
    if object.get("origin").is_some_and(|v| v != "AI_EDITOR" && v != "CLI") {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    if object.get("metadata").is_some_and(|v| {
        v.as_object().is_none_or(|v| {
            v.iter().any(|(key, value)| !key.starts_with("kiro:") || !value.is_string())
        })
    }) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    // Routing attribution is not model input and contains transient Kiro session IDs.
    object.remove("origin");
    object.remove("metadata");
    Ok(())
}
