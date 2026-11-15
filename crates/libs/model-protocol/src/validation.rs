//! Strict shared wire-shape validation.
use axum::http::StatusCode;
use serde_json::Value;

pub fn object_keys(value: &Value, keys: &[&str]) -> Result<(), StatusCode> {
    let object = value.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    if object.keys().any(|key| !keys.contains(&key.as_str())) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(())
}
pub fn validate_chat_reasoning(message: &Value) -> Result<(), StatusCode> {
    if message
        .get("reasoning_content")
        .is_some_and(|reasoning| !reasoning.is_null() && reasoning.as_str() != Some(""))
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(())
}
