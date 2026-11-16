//! Trusted application actions supplied at the native request boundary.
use std::{future::Future, pin::Pin};
pub trait Host: Sync {
    fn begin_usage(&self, provider: &str, model: &str) -> Option<agent_usage::native::NativeRequest>;
    fn authorize_model_request<'a>(&'a self, model: &'a str)
        -> Pin<Box<dyn Future<Output = anyhow::Result<bool>> + Send + 'a>>;
}
