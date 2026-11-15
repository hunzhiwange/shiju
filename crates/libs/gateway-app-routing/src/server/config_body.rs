//! Strict object decoding for routing settings.
use super::ControlError;
use axum::{Json, extract::rejection::JsonRejection};
use serde::de::DeserializeOwned;
use serde_json::value::RawValue;

pub type ConfigBody = Result<Json<Box<RawValue>>, JsonRejection>;

pub fn decode<T: DeserializeOwned>(body: ConfigBody) -> Result<T, ControlError> {
    let Json(body) = body.map_err(|_| ControlError::InvalidConfig)?;
    if !body.get().starts_with('{') {
        return Err(ControlError::InvalidConfig);
    }
    serde_json::from_str(body.get()).map_err(|_| ControlError::InvalidConfig)
}
