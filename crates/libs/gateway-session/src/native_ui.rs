//! Native UI persistence; instance bootstrap remains supplied by the host.
pub struct NativeUiHost<H>(std::marker::PhantomData<H>);
impl<H: crate::message_host::MessageHost> crate::message_host::MessageHost for NativeUiHost<H> {
    fn current_tenant_scope() -> vw_api_transport::TenantScope { H::current_tenant_scope() }
    fn resolve_directory(query: &gateway_instance::host::InstanceQuery, headers: &axum::http::HeaderMap) -> String { H::resolve_directory(query, headers) }
    fn with_tenant_instance<T: Send + 'static>(tenant: vw_api_transport::TenantScope, directory: String, action: impl FnOnce() -> gateway_instance::host::HostFuture<Result<T, gateway_types::ApiError>> + Send + 'static) -> gateway_instance::host::HostFuture<Result<T, gateway_types::ApiError>> { H::with_tenant_instance(tenant, directory, action) }
}
use session_store::SessionMessageDeltaResult;
use session_ui_types::*;
use std::{collections::HashSet, path::PathBuf};
use vw_api_transport::TenantScope;
impl<H: crate::message_host::MessageHost> crate::ui_host::UiHost for NativeUiHost<H> {
    fn current_session_scope() -> Option<String> {
        session_store::ui_store::current_session_scope()
    }
    fn apply_session_message_delta_scoped_for_tenant(
        tenant: &TenantScope,
        delta: &SessionMessageDelta,
        scope: Option<&str>,
    ) -> anyhow::Result<SessionMessageDeltaResult> {
        session_store::ui_store::apply_session_message_delta_scoped_for_tenant(
            tenant, delta, scope,
        )
    }
    fn load_archived_session_ids_scoped_for_tenant(
        tenant: &TenantScope,
        scope: Option<&str>,
    ) -> HashSet<String> {
        session_store::ui_store::load_archived_session_ids_scoped_for_tenant(tenant, scope)
    }
    fn load_llm_raw_step_indices_scoped_for_tenant(
        tenant: &TenantScope,
        session_id: &str,
        scope: Option<&str>,
    ) -> std::io::Result<Vec<u32>> {
        session_store::ui_store::load_llm_raw_step_indices_scoped_for_tenant(
            tenant, session_id, scope,
        )
    }
    fn load_llm_raw_step_scoped_for_tenant(
        tenant: &TenantScope,
        session_id: &str,
        step_index: u32,
        scope: Option<&str>,
    ) -> std::io::Result<Option<serde_json::Value>> {
        session_store::ui_store::load_llm_raw_step_scoped_for_tenant(
            tenant, session_id, step_index, scope,
        )
    }
    fn load_llm_raw_turn_step_scoped_for_tenant(
        tenant: &TenantScope,
        session_id: &str,
        turn_id: &str,
        step_index: u32,
        scope: Option<&str>,
    ) -> std::io::Result<Option<serde_json::Value>> {
        session_store::ui_store::load_llm_raw_turn_step_scoped_for_tenant(
            tenant, session_id, turn_id, step_index, scope,
        )
    }
    fn load_session_agent_identity_scoped_for_tenant(
        tenant: &TenantScope,
        session: &str,
        project: Option<&str>,
    ) -> std::io::Result<Option<vw_api_chat_identity::SessionAgentIdentity>> {
        let Some(project) = project else { return Ok(None); };
        session_engine::prompt::block_on(session_engine::binding_store::load_scoped(
            agent_paths::paths().home.clone(), tenant.clone(), project, session,
        )).map(|binding| binding.map(|binding| binding.identity())).map_err(std::io::Error::other)
    }
    fn load_session_any_for_tenant(tenant: &TenantScope, id: &str) -> Option<ChatSession> {
        session_store::ui_store::load_session_any_for_tenant(tenant, id)
    }
    fn load_session_message_page_scoped_for_tenant(
        tenant: &TenantScope,
        session_id: &str,
        scope: Option<&str>,
        before_seq_no: Option<u64>,
        limit: usize,
    ) -> anyhow::Result<SessionMessagePage> {
        session_store::ui_store::load_session_message_page_scoped_for_tenant(
            tenant,
            session_id,
            scope,
            before_seq_no,
            limit,
        )
    }
    fn load_session_meta_scoped_for_tenant(
        tenant: &TenantScope,
        id: &str,
        scope: Option<&str>,
    ) -> anyhow::Result<Option<ChatSessionMeta>> {
        session_store::ui_store::load_session_meta_scoped_for_tenant(tenant, id, scope)
    }
    fn load_session_scoped_for_tenant(
        tenant: &TenantScope,
        id: &str,
        scope: Option<&str>,
    ) -> Option<ChatSession> {
        session_store::ui_store::load_session_scoped_for_tenant(tenant, id, scope)
    }
    fn load_session_ui_meta_scoped_for_tenant(
        tenant: &TenantScope,
        id: &str,
        scope: &str,
    ) -> anyhow::Result<Option<ChatSessionMeta>> {
        session_store::ui_store::load_session_ui_meta_scoped_for_tenant(tenant, id, scope)
    }
    fn load_sessions_scoped_for_tenant(
        tenant: &TenantScope,
        scope: Option<&str>,
    ) -> Vec<ChatSessionMeta> {
        session_store::ui_store::load_sessions_scoped_for_tenant(tenant, scope)
    }
    fn resolve_session_scope_id_for_tenant(
        tenant: &TenantScope,
        path: Option<&str>,
        project_id: Option<&str>,
    ) -> Option<String> {
        session_store::ui_store::resolve_session_scope_id_for_tenant(tenant, path, project_id)
    }
    fn save_archived_session_ids_scoped_for_tenant(
        tenant: &TenantScope,
        ids: &HashSet<String>,
        scope: Option<&str>,
    ) {
        session_store::ui_store::save_archived_session_ids_scoped_for_tenant(tenant, ids, scope)
    }
    fn save_session_ui_content_scoped_for_tenant(
        tenant: &TenantScope,
        session: &ChatSession,
        scope: Option<&str>,
    ) -> anyhow::Result<PathBuf> {
        session_store::ui_store::save_session_ui_content_scoped_for_tenant(tenant, session, scope)
    }
    fn session_file_path(id: &str) -> Option<PathBuf> {
        session_store::ui_store::session_file_path(id)
    }
    fn session_preview_meta_for_tenant(tenant: &TenantScope, id: &str) -> Option<ChatSessionMeta> {
        session_store::ui_store::session_preview_meta_for_tenant(tenant, id)
    }
    fn set_session_scope(path: Option<&str>) {
        session_store::ui_store::set_session_scope(path)
    }
    async fn settings(
        project: &str,
        session: &str,
    ) -> anyhow::Result<session_engine::binding_store::SessionSettings> {
        session_engine::binding_store::SessionSettings::load(
            agent_paths::paths().home.clone(), session_store::ui_store::current_tenant_scope(), project, session,
        ).await
    }
    fn reserve(
        tenant: &TenantScope,
        scope: &str,
        id: &str,
        directory: &str,
        created_ms: u64,
    ) -> anyhow::Result<()> {
        crate::stream::reserve_stream_agent_session(tenant, scope, id, directory, created_ms)
    }
}

use axum::extract::{Extension, Request};
use axum::middleware::Next;
use axum::response::Response;
pub async fn session_tenant_scope(
    Extension(tenant): Extension<TenantScope>,
    request: Request,
    next: Next,
) -> Response {
    session_store::ui_store::with_tenant_scope(tenant, next.run(request)).await
}
