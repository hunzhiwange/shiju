//! Host composition for the original prompt, MCP and usage operation registries.
use axum::{Extension, http::HeaderMap, response::{IntoResponse, Response}};
use std::sync::Arc;
use vw_api_transport::TenantScope;
use vw_agent_apps::apps::lifecycle::operations::Operation;
pub(super) use gateway_agent_apps::lifecycle::status;
enum OwnedOperation {
    #[cfg(target_os = "macos")]
    Prompt(Arc<super::super::agent_app_prompt_sync::Job>),
    Mcp(Arc<super::super::mcp_management_resync::Job>),
}

impl gateway_types::app_operations::AppOperation for OwnedOperation {
    fn operation(&self) -> &Operation {
        match self {
            #[cfg(target_os = "macos")]
            Self::Prompt(entry) => &entry.operation,
            Self::Mcp(entry) => &entry.operation,
        }
    }

    fn response(&self, id: String) -> Response {
        match self {
            #[cfg(target_os = "macos")]
            Self::Prompt(entry) => entry.response(id),
            Self::Mcp(entry) => entry.response(id),
        }
    }
}

type McpJobs = Option<Extension<super::super::mcp_management_resync::Jobs>>;

#[allow(clippy::too_many_arguments)]
async fn resolve_operation(
    policy: Option<std::path::PathBuf>,
    tenant: TenantScope,
    headers: HeaderMap,
    host: &str,
    id: &str,
    #[cfg(target_os = "macos")] prompts: Option<Extension<super::super::agent_app_prompt_sync::Jobs>>,
    mcp: McpJobs,
) -> Result<Option<std::sync::Arc<dyn gateway_types::app_operations::AppOperation>>, Response> {
    #[cfg(target_os = "macos")]
    if let Some(Extension(jobs)) = prompts
        && let Some(entry) = jobs.owned(id, &tenant, host, &headers, policy.as_deref()).map_err(IntoResponse::into_response)? {
        return Ok(Some(Arc::new(OwnedOperation::Prompt(entry))));
    }
    if let Some(Extension(jobs)) = mcp
        && let Some(entry) = jobs
            .owned(
                id,
                &tenant,
                host,
                &headers,
                &super::super::mcp_management_read::PolicyFile(policy.clone()),
            )
            .map_err(IntoResponse::into_response)?
    {
        return Ok(Some(Arc::new(OwnedOperation::Mcp(entry))));
    }
    Ok(None)
}

#[derive(Clone)]
struct Sources {
    #[cfg(target_os = "macos")]
    prompts: Option<super::super::agent_app_prompt_sync::Jobs>,
    mcp: Option<super::super::mcp_management_resync::Jobs>,
}
impl gateway_types::app_operations::AppOperationSource for Sources {
    fn resolve(&self, tenant: TenantScope, headers: HeaderMap, host: String, id: String,
        policy: Option<std::path::PathBuf>, cancel: bool) -> gateway_types::app_operations::Resolution<'_> {
        let _ = cancel;
        Box::pin(async move {
            resolve_operation(policy, tenant, headers, &host, &id,
                #[cfg(target_os = "macos")]
                self.prompts.clone().map(Extension),
                self.mcp.clone().map(Extension),
            ).await
        })
    }
}
pub(super) fn operation_sources(
    #[cfg(target_os = "macos")] prompts: Option<super::super::agent_app_prompt_sync::Jobs>,
    mcp: Option<super::super::mcp_management_resync::Jobs>,
) -> gateway_types::app_operations::OperationSources {
    gateway_types::app_operations::OperationSources(Arc::new(Sources {
        #[cfg(target_os = "macos")]
        prompts, mcp,
    }))
}

async fn inject_sources(mut request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let source = operation_sources(
        #[cfg(target_os = "macos")]
        request.extensions().get::<super::super::agent_app_prompt_sync::Jobs>().cloned(),
        request.extensions().get::<super::super::mcp_management_resync::Jobs>().cloned(),
    );
    request.extensions_mut().insert(source);
    next.run(request).await
}
pub(super) fn router() -> axum::Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from))
}
pub(super) fn router_with_policy(policy: Option<std::path::PathBuf>) -> axum::Router {
    gateway_agent_apps::lifecycle::router_with_policy(policy)
        .layer(axum::middleware::from_fn(inject_sources))
}
