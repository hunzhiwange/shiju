use session_store::SessionMessageDeltaResult;
use session_ui_types::*;
use std::{collections::HashSet, path::PathBuf};
use vw_api_transport::TenantScope;

/// UI persistence operations supplied by the native session store adapter.
pub trait UiHost: crate::message_host::MessageHost {
    fn current_session_scope() -> Option<String>;
    fn apply_session_message_delta_scoped_for_tenant(
        tenant: &TenantScope,
        delta: &SessionMessageDelta,
        scope: Option<&str>,
    ) -> anyhow::Result<SessionMessageDeltaResult>;
    fn load_archived_session_ids_scoped_for_tenant(
        tenant: &TenantScope,
        scope: Option<&str>,
    ) -> HashSet<String>;
    fn load_llm_raw_step_indices_scoped_for_tenant(
        tenant: &TenantScope,
        session_id: &str,
        scope: Option<&str>,
    ) -> std::io::Result<Vec<u32>>;
    fn load_llm_raw_step_scoped_for_tenant(
        tenant: &TenantScope,
        session_id: &str,
        step_index: u32,
        scope: Option<&str>,
    ) -> std::io::Result<Option<serde_json::Value>>;
    fn load_llm_raw_turn_step_scoped_for_tenant(
        tenant: &TenantScope,
        session_id: &str,
        turn_id: &str,
        step_index: u32,
        scope: Option<&str>,
    ) -> std::io::Result<Option<serde_json::Value>>;
    fn load_session_agent_identity_scoped_for_tenant(
        tenant: &TenantScope,
        session: &str,
        project: Option<&str>,
    ) -> std::io::Result<Option<vw_api_chat_identity::SessionAgentIdentity>>;
    fn load_session_any_for_tenant(tenant: &TenantScope, id: &str) -> Option<ChatSession>;
    fn load_session_message_page_scoped_for_tenant(
        tenant: &TenantScope,
        session_id: &str,
        scope: Option<&str>,
        before_seq_no: Option<u64>,
        limit: usize,
    ) -> anyhow::Result<SessionMessagePage>;
    fn load_session_meta_scoped_for_tenant(
        tenant: &TenantScope,
        id: &str,
        scope: Option<&str>,
    ) -> anyhow::Result<Option<ChatSessionMeta>>;
    fn load_session_scoped_for_tenant(
        tenant: &TenantScope,
        id: &str,
        scope: Option<&str>,
    ) -> Option<ChatSession>;
    fn load_session_ui_meta_scoped_for_tenant(
        tenant: &TenantScope,
        id: &str,
        scope: &str,
    ) -> anyhow::Result<Option<ChatSessionMeta>>;
    fn load_sessions_scoped_for_tenant(
        tenant: &TenantScope,
        scope: Option<&str>,
    ) -> Vec<ChatSessionMeta>;
    fn resolve_session_scope_id_for_tenant(
        tenant: &TenantScope,
        path: Option<&str>,
        project_id: Option<&str>,
    ) -> Option<String>;
    fn save_archived_session_ids_scoped_for_tenant(
        tenant: &TenantScope,
        ids: &HashSet<String>,
        scope: Option<&str>,
    );
    fn save_session_ui_content_scoped_for_tenant(
        tenant: &TenantScope,
        session: &ChatSession,
        scope: Option<&str>,
    ) -> anyhow::Result<PathBuf>;
    fn session_file_path(id: &str) -> Option<PathBuf>;
    fn session_preview_meta_for_tenant(tenant: &TenantScope, id: &str) -> Option<ChatSessionMeta>;
    fn set_session_scope(path: Option<&str>);
    fn settings(
        project: &str,
        session: &str,
    ) -> impl std::future::Future<
        Output = anyhow::Result<session_engine::binding_store::SessionSettings>,
    > + Send;
    fn reserve(
        tenant: &TenantScope,
        scope: &str,
        id: &str,
        directory: &str,
        created_ms: u64,
    ) -> anyhow::Result<()>;
}
