//! Original host IO and task-local context operations used by voice routes.
use std::{future::Future, sync::Arc};
use vw_api_transport::TenantScope;
use session_info::Info;
use session_port::{ui_types::ChatSession, chat_stream::GatewayChatStreamCollected};
use gateway_types::ApiError;
use agent_approval::ApprovalManager;
use vw_api_chat::chat::GatewayChatStreamRequest;

pub trait VoiceHost: Send + Sync + 'static {
    fn with_tenant_scope<T: Send>(
        tenant: TenantScope,
        future: impl Future<Output = T> + Send,
    ) -> impl Future<Output = T> + Send;

    fn load_session(tenant: TenantScope, session: String)
        -> impl Future<Output = anyhow::Result<Option<Info>>> + Send;

    fn load_snapshot(tenant: TenantScope, session: String)
        -> impl Future<Output = anyhow::Result<Option<(Info, Option<ChatSession>)>>> + Send;

    fn approval_manager(tenant: TenantScope, directory: String)
        -> impl Future<Output = Result<Arc<ApprovalManager>, ApiError>> + Send;

    fn collect_chat(
        tenant: TenantScope,
        directory: String,
        request: GatewayChatStreamRequest,
        observer: impl FnMut(&serde_json::Value) + Send,
    ) -> impl Future<Output = Result<GatewayChatStreamCollected, ApiError>> + Send;
}
