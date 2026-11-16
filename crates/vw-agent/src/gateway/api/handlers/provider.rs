//! Provider routes with the existing tenant/project context adapter.
use axum::{Router, http::HeaderMap};
use gateway_provider::{HostFuture, InstanceQuery, ProviderHost};
use gateway_types::ApiError;
use vw_api_transport::id::TenantScope;

struct Host;

impl ProviderHost for Host {
    fn settings_provider_list() -> HostFuture<Result<
        (std::collections::HashMap<String, agent_provider::provider::Info>, std::collections::HashMap<String, String>),
        agent_provider::provider::CatalogError,
    >> {
        Box::pin(crate::provider::provider::settings_provider_list())
    }

    fn resolve_directory(query: &InstanceQuery, headers: &HeaderMap) -> String {
        crate::gateway::instance::resolve_directory(
            &crate::gateway::instance::InstanceQuery { directory: query.directory.clone() },
            headers,
        )
    }

    fn with_context<T: Send + 'static>(
        tenant: TenantScope,
        directory: String,
        action: impl FnOnce() -> HostFuture<Result<T, ApiError>> + Send + 'static,
    ) -> HostFuture<Result<T, ApiError>> {
        Box::pin(crate::session::ui_store::with_tenant_scope(
            tenant,
            crate::gateway::instance::with_instance(directory, action),
        ))
    }
}

pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    gateway_provider::router::<S, Host>()
}

#[cfg(test)]
use gateway_provider::{
    ProviderListResponse,
};
#[cfg(test)]
use agent_provider as provider;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use axum::{Extension, Json, extract::Query};
#[cfg(test)]
async fn provider_list(
    tenant: Extension<TenantScope>,
    query: Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<ProviderListResponse>, ApiError> {
    gateway_provider::provider_list::<Host>(tenant, query, headers).await
}

#[cfg(test)]
#[path = "../../../../../libs/gateway-provider/src/provider_tests.rs"]
mod provider_tests;
