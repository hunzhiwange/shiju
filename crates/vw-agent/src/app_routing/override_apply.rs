//! Per-provider edits to the converted upstream request.
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header};
use serde_json::{Map, Value};
use vw_agent_apps::providers::validation::custom_user_agent;
use vw_agent_apps::routing::private_params::filter_private_params;
use vw_api_types::app_providers::validation::ProviderRequestOverrides;

/// Options have already been validated when the target was bound to the grant.
pub(super) fn apply(
    body: &mut Value,
    headers: &mut HeaderMap,
    overrides: &ProviderRequestOverrides,
) -> Result<(), StatusCode> {
    if let Some(patch) = &overrides.body {
        let object = body.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?;
        merge_body(object, patch);
        // Provider options can introduce private fields as well.
        filter_private_params(body);
    }
    if let Some(values) = &overrides.headers {
        for (name, value) in values {
            let name = HeaderName::from_bytes(name.trim().as_bytes())
                .map_err(|_| StatusCode::BAD_GATEWAY)?;
            let value = if name == header::USER_AGENT {
                custom_user_agent(Some(value)).map_err(|_| StatusCode::BAD_GATEWAY)?
            } else {
                Some(
                    HeaderValue::from_bytes(value.as_bytes())
                        .map_err(|_| StatusCode::BAD_GATEWAY)?,
                )
            };
            if let Some(mut value) = value {
                value.set_sensitive(true);
                headers.insert(name, value);
            }
        }
    }
    Ok(())
}

fn merge_body(target: &mut Map<String, Value>, patch: &Map<String, Value>) {
    for (key, patch_value) in patch {
        let target_value = target.entry(key.clone()).or_insert(Value::Null);
        match (target_value, patch_value) {
            (Value::Object(target), Value::Object(patch)) => merge_body(target, patch),
            (target, patch) => *target = patch.clone(),
        }
    }
}
