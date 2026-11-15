//! Host instance and tenant context integration.
use std::{future::Future, pin::Pin};
use axum::http::HeaderMap;
use gateway_types::ApiError;
use serde::Deserialize;
use vw_api_transport::TenantScope;
use crate::{AgentInfo, SkillInfo};
pub type HostFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;
#[derive(Debug, Deserialize)]
pub struct InstanceQuery { pub directory: Option<String> }
pub trait MiscHost: Send + Sync + 'static {
    fn resolve_directory(query: &InstanceQuery, headers: &HeaderMap) -> String;
    fn with_instance<T: Send + 'static>(directory: String,
        action: impl FnOnce() -> HostFuture<Result<T, ApiError>> + Send + 'static,
    ) -> HostFuture<Result<T, ApiError>>;
    fn with_tenant_scope<T: Send + 'static>(tenant: TenantScope,
        future: impl Future<Output = T> + Send + 'static,
    ) -> HostFuture<T>;
    fn commands() -> HostFuture<Vec<agent_commands::Info>>;
    fn agents() -> HostFuture<Vec<AgentInfo>>;
    fn skills() -> HostFuture<Vec<SkillInfo>>;
    fn get_current() -> HostFuture<anyhow::Result<agent_config::runtime::Config>>;
    fn get_global(tenant: TenantScope) -> HostFuture<anyhow::Result<agent_config::runtime::Config>>;
    fn update_global(tenant: TenantScope, patch: serde_json::Value) -> HostFuture<anyhow::Result<()>>;
}
