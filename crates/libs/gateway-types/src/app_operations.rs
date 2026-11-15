//! Existing app operation lookup and response boundary for gateway composition.
use std::{future::Future, path::PathBuf, pin::Pin, sync::Arc};
use axum::{http::HeaderMap, response::Response};
use vw_api_transport::TenantScope;
use vw_agent_apps::apps::lifecycle::operations::Operation;

pub trait AppOperation: Send + Sync {
    fn operation(&self) -> &Operation;
    fn response(&self, id: String) -> Response;
}
pub type Resolution<'a> = Pin<Box<dyn Future<Output = Result<Option<Arc<dyn AppOperation>>, Response>> + Send + 'a>>;
pub trait AppOperationSource: Send + Sync {
    fn resolve(&self, tenant: TenantScope, headers: HeaderMap, host: String, id: String,
        policy: Option<PathBuf>, cancel: bool) -> Resolution<'_>;
}
#[derive(Clone)]
pub struct OperationSources(pub Arc<dyn AppOperationSource>);
