use crate::gateway::{ApiError, instance::InstanceQuery};
use axum::{
    Json,
    extract::{Path, Query},
    http::HeaderMap,
};
use gateway_session::ui_ops::SessionWithRaw;
use serde_json::Value;
use session_ui_types as ui_models;
use vw_api_session::session::*;
// Native store injection for UI snapshot handlers.
pub(super) struct Host;
impl gateway_session::message_host::MessageHost for Host {
    fn current_tenant_scope() -> vw_api_transport::TenantScope {
        crate::session::ui_store::current_tenant_scope()
    }
    fn resolve_directory(
        query: &gateway_instance::host::InstanceQuery,
        headers: &axum::http::HeaderMap,
    ) -> String {
        crate::gateway::instance::resolve_directory(
            &crate::gateway::instance::InstanceQuery { directory: query.directory.clone() },
            headers,
        )
    }
    fn with_tenant_instance<T: Send + 'static>(
        tenant: vw_api_transport::TenantScope,
        directory: String,
        action: impl FnOnce() -> gateway_instance::host::HostFuture<Result<T, gateway_types::ApiError>>
        + Send
        + 'static,
    ) -> gateway_instance::host::HostFuture<Result<T, gateway_types::ApiError>> {
        Box::pin(super::shared::with_tenant_instance(tenant, directory, action))
    }
}
use session_store::SessionMessageDeltaResult;
use session_ui_types::*;
use std::{collections::HashSet, path::PathBuf};
use vw_api_transport::TenantScope;
impl gateway_session::ui_host::UiHost for Host {
    fn current_session_scope() -> Option<String> {
        crate::session::ui_store::current_session_scope()
    }
    fn apply_session_message_delta_scoped_for_tenant(
        tenant: &TenantScope,
        delta: &SessionMessageDelta,
        scope: Option<&str>,
    ) -> anyhow::Result<SessionMessageDeltaResult> {
        crate::session::ui_store::apply_session_message_delta_scoped_for_tenant(
            tenant, delta, scope,
        )
    }
    fn load_archived_session_ids_scoped_for_tenant(
        tenant: &TenantScope,
        scope: Option<&str>,
    ) -> HashSet<String> {
        crate::session::ui_store::load_archived_session_ids_scoped_for_tenant(tenant, scope)
    }
    fn load_llm_raw_step_indices_scoped_for_tenant(
        tenant: &TenantScope,
        session_id: &str,
        scope: Option<&str>,
    ) -> std::io::Result<Vec<u32>> {
        crate::session::ui_store::load_llm_raw_step_indices_scoped_for_tenant(
            tenant, session_id, scope,
        )
    }
    fn load_llm_raw_step_scoped_for_tenant(
        tenant: &TenantScope,
        session_id: &str,
        step_index: u32,
        scope: Option<&str>,
    ) -> std::io::Result<Option<serde_json::Value>> {
        crate::session::ui_store::load_llm_raw_step_scoped_for_tenant(
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
        crate::session::ui_store::load_llm_raw_turn_step_scoped_for_tenant(
            tenant, session_id, turn_id, step_index, scope,
        )
    }
    fn load_session_agent_identity_scoped_for_tenant(
        tenant: &TenantScope,
        session: &str,
        project: Option<&str>,
    ) -> std::io::Result<Option<session_ui_types::SessionAgentIdentity>> {
        crate::session::ui_store::load_session_agent_identity_scoped_for_tenant(
            tenant, session, project,
        )
    }
    fn load_session_any_for_tenant(tenant: &TenantScope, id: &str) -> Option<ChatSession> {
        crate::session::ui_store::load_session_any_for_tenant(tenant, id)
    }
    fn load_session_message_page_scoped_for_tenant(
        tenant: &TenantScope,
        session_id: &str,
        scope: Option<&str>,
        before_seq_no: Option<u64>,
        limit: usize,
    ) -> anyhow::Result<SessionMessagePage> {
        crate::session::ui_store::load_session_message_page_scoped_for_tenant(
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
        crate::session::ui_store::load_session_meta_scoped_for_tenant(tenant, id, scope)
    }
    fn load_session_scoped_for_tenant(
        tenant: &TenantScope,
        id: &str,
        scope: Option<&str>,
    ) -> Option<ChatSession> {
        crate::session::ui_store::load_session_scoped_for_tenant(tenant, id, scope)
    }
    fn load_session_ui_meta_scoped_for_tenant(
        tenant: &TenantScope,
        id: &str,
        scope: &str,
    ) -> anyhow::Result<Option<ChatSessionMeta>> {
        crate::session::ui_store::load_session_ui_meta_scoped_for_tenant(tenant, id, scope)
    }
    fn load_sessions_scoped_for_tenant(
        tenant: &TenantScope,
        scope: Option<&str>,
    ) -> Vec<ChatSessionMeta> {
        crate::session::ui_store::load_sessions_scoped_for_tenant(tenant, scope)
    }
    fn resolve_session_scope_id_for_tenant(
        tenant: &TenantScope,
        path: Option<&str>,
        project_id: Option<&str>,
    ) -> Option<String> {
        crate::session::ui_store::resolve_session_scope_id_for_tenant(tenant, path, project_id)
    }
    fn save_archived_session_ids_scoped_for_tenant(
        tenant: &TenantScope,
        ids: &HashSet<String>,
        scope: Option<&str>,
    ) {
        crate::session::ui_store::save_archived_session_ids_scoped_for_tenant(tenant, ids, scope)
    }
    fn save_session_ui_content_scoped_for_tenant(
        tenant: &TenantScope,
        session: &ChatSession,
        scope: Option<&str>,
    ) -> anyhow::Result<PathBuf> {
        crate::session::ui_store::save_session_ui_content_scoped_for_tenant(tenant, session, scope)
    }
    fn session_file_path(id: &str) -> Option<PathBuf> {
        crate::session::ui_store::session_file_path(id)
    }
    fn session_preview_meta_for_tenant(tenant: &TenantScope, id: &str) -> Option<ChatSessionMeta> {
        crate::session::ui_store::session_preview_meta_for_tenant(tenant, id)
    }
    fn set_session_scope(path: Option<&str>) {
        crate::session::ui_store::set_session_scope(path)
    }
    async fn settings(
        project: &str,
        session: &str,
    ) -> anyhow::Result<session_engine::binding_store::SessionSettings> {
        crate::session::agent_binding::SessionSettings::load(project, session).await.map(|value| {
            session_engine::binding_store::SessionSettings {
                binding: value.binding.clone(),
                full_access: value.full_access,
            }
        })
    }
    fn reserve(
        tenant: &TenantScope,
        scope: &str,
        id: &str,
        directory: &str,
        created_ms: u64,
    ) -> anyhow::Result<()> {
        super::stream::reserve_stream_agent_session(tenant, scope, id, directory, created_ms)
    }
}
pub(super) async fn session_ui_summary(
    arg0: Path<String>,
    arg1: Query<InstanceQuery>,
    arg2: HeaderMap,
) -> Result<Json<GatewaySessionUiSummary>, ApiError> {
    gateway_session::ui_ops::session_ui_summary::<Host>(
        arg0,
        Query(gateway_instance::host::InstanceQuery { directory: arg1.0.directory }),
        arg2,
    )
    .await
}
pub(super) async fn session_ui_messages(
    arg0: Path<String>,
    arg1: Query<GatewaySessionMessagePageQuery>,
    arg2: HeaderMap,
) -> Result<Json<GatewaySessionMessagePage<ui_models::SessionMessageRecord>>, ApiError> {
    gateway_session::ui_ops::session_ui_messages::<Host>(arg0, arg1, arg2).await
}
pub(super) async fn session_ui_message_delta(
    arg0: Path<String>,
    arg1: Query<InstanceQuery>,
    arg2: HeaderMap,
    arg3: Json<ui_models::SessionMessageDelta>,
) -> Result<Json<GatewaySessionMessageDeltaResponse>, ApiError> {
    gateway_session::ui_ops::session_ui_message_delta::<Host>(
        arg0,
        Query(gateway_instance::host::InstanceQuery { directory: arg1.0.directory }),
        arg2,
        arg3,
    )
    .await
}
pub(super) async fn session_ui_get(
    arg0: Path<String>,
    arg1: Query<InstanceQuery>,
    arg2: HeaderMap,
) -> Result<Json<Option<SessionWithRaw>>, ApiError> {
    gateway_session::ui_ops::session_ui_get::<Host>(
        arg0,
        Query(gateway_instance::host::InstanceQuery { directory: arg1.0.directory }),
        arg2,
    )
    .await
}
pub(super) async fn session_ui_get_without_raw(
    arg0: Path<String>,
    arg1: Query<InstanceQuery>,
    arg2: HeaderMap,
) -> Result<Json<Option<SessionWithRaw>>, ApiError> {
    gateway_session::ui_ops::session_ui_get_without_raw::<Host>(
        arg0,
        Query(gateway_instance::host::InstanceQuery { directory: arg1.0.directory }),
        arg2,
    )
    .await
}
pub(super) async fn session_step_llm_raw_get(
    arg0: Path<(String, String, u32)>,
    arg1: Query<InstanceQuery>,
    arg2: HeaderMap,
) -> Result<Json<Option<Value>>, ApiError> {
    gateway_session::ui_ops::session_step_llm_raw_get::<Host>(
        arg0,
        Query(gateway_instance::host::InstanceQuery { directory: arg1.0.directory }),
        arg2,
    )
    .await
}
pub(super) async fn session_ui_save_request(
    arg0: Path<String>,
    arg1: Query<InstanceQuery>,
    arg2: HeaderMap,
    arg3: Json<Value>,
) -> Result<Json<bool>, ApiError> {
    gateway_session::ui_ops::session_ui_save_request::<Host>(
        arg0,
        Query(gateway_instance::host::InstanceQuery { directory: arg1.0.directory }),
        arg2,
        arg3,
    )
    .await
}
pub(super) async fn session_ui_save(
    arg0: Path<String>,
    arg1: Query<InstanceQuery>,
    arg2: HeaderMap,
    arg3: Json<ui_models::ChatSession>,
) -> Result<Json<bool>, ApiError> {
    gateway_session::ui_ops::session_ui_save::<Host>(
        arg0,
        Query(gateway_instance::host::InstanceQuery { directory: arg1.0.directory }),
        arg2,
        arg3,
    )
    .await
}
pub(super) async fn session_ui_previews(
    arg0: Query<InstanceQuery>,
    arg1: HeaderMap,
) -> Result<Json<Vec<ui_models::ChatSessionMeta>>, ApiError> {
    gateway_session::ui_ops::session_ui_previews::<Host>(
        Query(gateway_instance::host::InstanceQuery { directory: arg0.0.directory }),
        arg1,
    )
    .await
}
pub(super) async fn session_ui_preview_meta(
    arg0: Path<String>,
    arg1: Query<InstanceQuery>,
    arg2: HeaderMap,
) -> Result<Json<Option<ui_models::ChatSessionMeta>>, ApiError> {
    gateway_session::ui_ops::session_ui_preview_meta::<Host>(
        arg0,
        Query(gateway_instance::host::InstanceQuery { directory: arg1.0.directory }),
        arg2,
    )
    .await
}
pub(super) async fn session_archived_get(
    arg0: Query<InstanceQuery>,
    arg1: HeaderMap,
) -> Result<Json<Vec<String>>, ApiError> {
    gateway_session::ui_ops::session_archived_get::<Host>(
        Query(gateway_instance::host::InstanceQuery { directory: arg0.0.directory }),
        arg1,
    )
    .await
}
pub(super) async fn session_archived_put(
    arg0: Query<InstanceQuery>,
    arg1: HeaderMap,
    arg2: Json<Vec<String>>,
) -> Result<Json<bool>, ApiError> {
    gateway_session::ui_ops::session_archived_put::<Host>(
        Query(gateway_instance::host::InstanceQuery { directory: arg0.0.directory }),
        arg1,
        arg2,
    )
    .await
}
pub(super) async fn session_path_get(arg0: Path<String>) -> Result<Json<Option<String>>, ApiError> {
    gateway_session::ui_ops::session_path_get::<Host>(arg0).await
}
pub(super) async fn session_scope_get() -> Result<Json<Option<String>>, ApiError> {
    gateway_session::ui_ops::session_scope_get::<Host>().await
}
pub(super) async fn session_scope_put(
    arg0: Json<GatewaySessionScopeBody>,
) -> Result<Json<bool>, ApiError> {
    gateway_session::ui_ops::session_scope_put::<Host>(arg0).await
}
pub(super) async fn session_ui_get_any(
    arg0: Path<String>,
) -> Result<Json<Option<ui_models::ChatSession>>, ApiError> {
    gateway_session::ui_ops::session_ui_get_any::<Host>(arg0).await
}
#[cfg(test)]
#[path = "ui_ops_tests.rs"]
mod ui_ops_tests;
