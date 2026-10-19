use super::{ProviderFetch, ProviderPoll};
use crate::app::agent::gateway::ApiError;

pub(super) async fn fetch() -> Result<ProviderFetch, ApiError> {
    Err(ApiError::not_implemented("WeChat QR authorization is not implemented"))
}

pub(super) async fn poll(_state: &str) -> Result<ProviderPoll, ApiError> {
    Err(ApiError::not_implemented("WeChat QR authorization is not implemented"))
}
