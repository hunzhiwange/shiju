//! Misc route host composition and integration tests.
use axum::{Router, http::HeaderMap};
use gateway_misc::host::{HostFuture, MiscHost};
use gateway_misc::{AgentInfo, SkillInfo};
use crate::gateway::ApiError;
use vw_api_transport::TenantScope;
struct Host;
impl MiscHost for Host {
    fn resolve_directory(query: &gateway_misc::host::InstanceQuery, headers: &HeaderMap) -> String {
        crate::gateway::instance::resolve_directory(&crate::gateway::instance::InstanceQuery { directory: query.directory.clone() }, headers)
    }
    fn with_instance<T: Send + 'static>(directory: String, action: impl FnOnce() -> HostFuture<Result<T, ApiError>> + Send + 'static) -> HostFuture<Result<T, ApiError>> {
        Box::pin(crate::gateway::instance::with_instance(directory, action))
    }
    fn with_tenant_scope<T: Send + 'static>(tenant: TenantScope, future: impl std::future::Future<Output = T> + Send + 'static) -> HostFuture<T> {
        Box::pin(crate::session::ui_store::with_tenant_scope(tenant, future))
    }
    fn commands() -> HostFuture<Vec<agent_commands::Info>> { Box::pin(crate::command::command::list()) }
    fn agents() -> HostFuture<Vec<AgentInfo>> {
        Box::pin(async { crate::agent::list().await.into_iter().map(|a| AgentInfo { key: a.key, definition: a.definition }).collect() })
    }
    fn skills() -> HostFuture<Vec<SkillInfo>> {
        Box::pin(async { crate::skill::all().await.into_iter().map(|s| SkillInfo { name: s.name, description: s.description, location: s.location, content: s.content }).collect() })
    }
    fn get_current() -> HostFuture<anyhow::Result<agent_config::runtime::Config>> {
        Box::pin(async { Ok(crate::config::get_current().await?) })
    }
    fn get_global(tenant: TenantScope) -> HostFuture<anyhow::Result<agent_config::runtime::Config>> {
        Box::pin(async move { Ok(crate::config::get_global_for_tenant(&tenant).await?) })
    }
    fn update_global(tenant: TenantScope, patch: serde_json::Value) -> HostFuture<anyhow::Result<()>> {
        Box::pin(async move { crate::config::update_global_for_tenant(&tenant, patch).await?; Ok(()) })
    }
}
pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> { gateway_misc::router::<S, Host>() }
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use gateway_misc::not_implemented;
#[cfg(test)]
use axum::extract::Extension;
#[cfg(test)]
#[path = "misc_tests.rs"]
mod misc_tests;
