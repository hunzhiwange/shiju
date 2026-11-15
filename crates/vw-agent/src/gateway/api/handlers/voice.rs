//! Voice route composition with the existing host IO and task-local scopes.
use gateway_voice::host::VoiceHost;
use std::{future::Future, sync::Arc};
use vw_api_transport::TenantScope;

pub(crate) struct Host;
impl VoiceHost for Host {
    async fn with_tenant_scope<T: Send>(tenant: TenantScope, future: impl Future<Output = T> + Send) -> T {
        crate::session::ui_store::with_tenant_scope(tenant, future).await
    }
    async fn load_session(tenant: TenantScope, session: String) -> anyhow::Result<Option<session_info::Info>> {
        crate::global_database::run_isolated(move || {
            crate::session::ui_store::load_agent_session_any_for_tenant_fallible(&tenant, &session)
        }).await?
    }
    async fn load_snapshot(tenant: TenantScope, session: String) -> anyhow::Result<Option<(session_info::Info, Option<session_port::ui_types::ChatSession>)>> {
        crate::global_database::run_isolated(move || {
            let Some(info) = crate::session::ui_store::load_agent_session_any_for_tenant_fallible(&tenant, &session)? else {
                return Ok(None);
            };
            let snapshot = crate::session::ui_store::load_session_scoped_for_tenant_fallible(&tenant, &session, Some(&info.project_id))?;
            Ok(Some((info, snapshot)))
        }).await?
    }
    async fn approval_manager(tenant: TenantScope, directory: String) -> Result<Arc<agent_approval::ApprovalManager>, gateway_types::ApiError> {
        Self::with_tenant_scope(tenant, crate::gateway::instance::with_instance(directory, || {
            Box::pin(async { Ok(crate::gateway::approval_state::approval_manager_for_current_instance().await) })
        })).await
    }
    async fn collect_chat(tenant: TenantScope, directory: String, request: vw_api_chat::chat::GatewayChatStreamRequest, observer: impl FnMut(&serde_json::Value) + Send) -> Result<session_port::chat_stream::GatewayChatStreamCollected, gateway_types::ApiError> {
        Self::with_tenant_scope(tenant, super::session::stream::collect_gateway_chat_stream_with_observer(directory, request, observer)).await
    }
}
pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> axum::Router<S> {
    gateway_voice::voice::router::<S, Host>()
}
#[cfg(test)]
#[path = "../../../../../libs/gateway-voice/src/host_tests/voice_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "../../../../../libs/gateway-voice/src/host_tests/voice_realtime_http_tests.rs"]
mod realtime_http_tests;
