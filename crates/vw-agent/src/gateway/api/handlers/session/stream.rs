//! Native environment injection for gateway-session streaming.
use serde_json::Value;
use std::{future::Future, path::Path};
use vw_api_transport::TenantScope;

use gateway_session::stream_host::{PromptInput, StreamHost};
pub(super) struct Host;
impl gateway_session::message_host::MessageHost for Host {
    fn current_tenant_scope() -> TenantScope {
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
        tenant: TenantScope,
        directory: String,
        action: impl FnOnce() -> gateway_instance::host::HostFuture<Result<T, gateway_types::ApiError>>
        + Send
        + 'static,
    ) -> gateway_instance::host::HostFuture<Result<T, gateway_types::ApiError>> {
        Box::pin(super::shared::with_tenant_instance(tenant, directory, action))
    }
}
impl StreamHost for Host {
    type ToolContext = Option<std::sync::Arc<crate::tools::ToolUseContext>>;
    type Approval = std::sync::Arc<crate::approval::ApprovalManager>;
    fn tool_context() -> Self::ToolContext {
        crate::tools::current_tool_use_context()
    }
    async fn approval_manager() -> Self::Approval {
        crate::gateway::approval_state::approval_manager_for_current_instance().await
    }
    async fn with_tenant_scope<T: Send>(
        tenant: TenantScope,
        future: impl Future<Output = T> + Send,
    ) -> T {
        crate::session::ui_store::with_tenant_scope(tenant, future).await
    }
    fn now_ms() -> u64 {
        crate::session::session::now_ms()
    }
    fn is_successful_finish_reason(reason: Option<&str>) -> bool {
        crate::session::runtime::is_successful_finish_reason(reason)
    }
    fn delegated_model_ref(config: &config_agent::DelegateAgentConfig) -> Option<String> {
        crate::agent::resolve_model_ref(config)
            .map(|model| format!("{}/{}", model.provider_id, model.model_id))
    }
    async fn agent(name: &str) -> Option<config_agent::AgentDefinitionConfig> {
        crate::agent::get(name).await
    }
    async fn config() -> anyhow::Result<config_aggregate::Config> {
        Ok(crate::config::get_current().await?)
    }
    async fn project_config(
        tenant: &TenantScope,
        path: &Path,
    ) -> anyhow::Result<config_aggregate::Config> {
        Ok(crate::config::get_project_for_tenant(tenant, path).await?)
    }
    async fn native_agent_defaults(
        config: &config_aggregate::Config,
        name: &str,
        model: Option<&str>,
        path: &Path,
    ) -> anyhow::Result<Option<session_engine::capability_policy::NativeAgentDefaults>> {
        crate::session::capability_policy::native_agent_defaults(config, name, model, path).await
    }
    async fn prepare_binding(
        session: &str,
        agent: Option<&str>,
        backend: Option<&str>,
        model: Option<&str>,
        path: &Path,
    ) -> anyhow::Result<(session_engine::agent_binding::AgentBinding, config_aggregate::Config)>
    {
        crate::session::agent_binding::prepare(session, agent, backend, model, path).await
    }
    async fn resolve_full_access(
        session: &str,
        explicit: Option<bool>,
        initial: bool,
    ) -> anyhow::Result<bool> {
        crate::session::agent_binding::resolve_full_access(session, explicit, initial).await
    }
    async fn fork_execution(
        project: &str,
        session: &str,
    ) -> anyhow::Result<Option<session_engine::fork_execution::ForkExecution>> {
        crate::session::fork_execution::load_scoped(project, session).await
    }
    async fn start_fork_execution(project: &str, session: &str) -> anyhow::Result<()> {
        crate::session::fork_execution::start(project, session).await
    }
    fn locked_acp_model(
        session: &str,
        directory: &str,
    ) -> Result<Option<vw_api_acp_model::acp_model_source::AcpSessionModelSelection>, String> {
        crate::session::llm::locked_acp_model(session, directory)
    }
    async fn forked_session(session: &str) -> anyhow::Result<Option<session_info::Info>> {
        Ok(crate::session::session::forked_session(session).await?)
    }
    async fn load_fork_history(
        info: &session_info::Info,
    ) -> anyhow::Result<(Vec<session_ui_types::ChatMessage>, Vec<Value>)> {
        Ok(crate::session::session::load_fork_history(info).await?)
    }
    async fn project_from_directory(
        tenant: &TenantScope,
        directory: &str,
    ) -> anyhow::Result<(agent_project::Info, String)> {
        Ok(crate::project::from_directory(tenant, directory).await?)
    }
    async fn refresh_stream_diff_summary(
        session: &str,
        parts: &[session_engine::message::Part],
    ) -> anyhow::Result<Option<session_info::Summary>> {
        Ok(crate::session::summary::refresh_stream_diff_summary(session, parts).await?)
    }
    async fn refresh_session_diff_summary(session: &str) -> anyhow::Result<session_info::Summary> {
        Ok(crate::session::summary::refresh_session_diff_summary(session).await?)
    }
    fn spawn_first_response_title(
        runtime: &tokio::runtime::Handle,
        tenant: TenantScope,
        context: agent_project::instance::Context,
        session: String,
        preview: String,
        model: Option<String>,
    ) {
        crate::session::title::spawn_first_response_title(
            runtime, tenant, context, session, preview, model,
        )
    }
    fn start_prompt(
        tenant: TenantScope,
        context: Self::ToolContext,
        approval: Self::Approval,
        input: PromptInput,
        mut before_emit: impl FnMut(&session_port::PromptEvent) + Send + 'static,
    ) -> session_port::PromptTurn {
        crate::session::ui_store::with_tenant_scope_sync(tenant, || {
            let start = || {
                let (request, persisted_history, prompt_rx) =
                    gateway_session::prompt_adapter::prepare_request(approval, input);
                crate::session::runtime::start_prompt_with_runner(
                    request,
                    move |request, mut emit| {
                        let _approval_prompt_rx = prompt_rx;
                        crate::session::processor::run_with_persisted_history(
                            request,
                            persisted_history,
                            move |event| {
                                gateway_session::prompt_adapter::observe_event(&event, &mut before_emit);
                                emit(event)
                            },
                        )
                    },
                )
            };
            match context {
                Some(context) => crate::tools::scope_tool_use_context_sync(context, start),
                None => start(),
            }
        })
    }
}

use gateway_session::stream as domain;

#[cfg(test)]
use super::shared::with_tenant_instance;
use crate::gateway::instance::InstanceQuery;
#[cfg(test)]
use crate::{project::instance, session as agent_session};
#[cfg(test)]
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json, extract::Query, http::HeaderMap};
#[cfg(test)]
use gateway_session::stream::*;
pub(crate) use gateway_session::stream::{
    GatewayChatStreamCollected, StreamTurnMessageIds, split_stream_model_ref,
};
use gateway_types::ApiError;
use session_engine::message;
use session_ui_types as ui_models;
#[cfg(test)]
use tokio::sync::{mpsc, mpsc::UnboundedReceiver};
use vw_api_chat::chat::GatewayChatStreamRequest;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn collect_gateway_chat_stream(
    dir: String,
    body: GatewayChatStreamRequest,
) -> Result<GatewayChatStreamCollected, ApiError> {
    domain::collect_gateway_chat_stream::<Host>(dir, body).await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn collect_gateway_chat_stream_with_observer(
    dir: String,
    body: GatewayChatStreamRequest,
    observe_payload: impl FnMut(&Value),
) -> Result<GatewayChatStreamCollected, ApiError> {
    domain::collect_gateway_chat_stream_with_observer::<Host>(dir, body, observe_payload).await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn collect_background_gateway_chat_stream(
    dir: String,
    body: GatewayChatStreamRequest,
    acp_full_access: bool,
    observe_payload: impl FnMut(&Value),
) -> Result<GatewayChatStreamCollected, ApiError> {
    domain::collect_background_gateway_chat_stream::<Host>(
        dir,
        body,
        acp_full_access,
        observe_payload,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn collect_scheduler_gateway_chat_stream(
    dir: String,
    body: GatewayChatStreamRequest,
    cancellation: tokio_util::sync::CancellationToken,
    observe_payload: impl FnMut(&Value),
) -> Result<GatewayChatStreamCollected, ApiError> {
    domain::collect_scheduler_gateway_chat_stream::<Host>(dir, body, cancellation, observe_payload)
        .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn persist_stream_chat_turn(
    session_id: &str,
    query: &str,
    assistant_text: &str,
    model: Option<&str>,
    usage: &ui_models::TokenUsage,
    finish_reason: Option<&str>,
    preallocated_ids: Option<&StreamTurnMessageIds>,
    extra_assistant_parts: &[message::Part],
) -> Result<(String, String), ApiError> {
    domain::persist_stream_chat_turn::<Host>(
        session_id,
        query,
        assistant_text,
        model,
        usage,
        finish_reason,
        preallocated_ids,
        extra_assistant_parts,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn persist_child_agent_turn(
    session_id: &str,
    query: &str,
    assistant_text: &str,
    agent_key: &str,
    error: Option<&str>,
) -> Result<(String, String), ApiError> {
    domain::persist_child_agent_turn::<Host>(session_id, query, assistant_text, agent_key, error)
        .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn commit_collected_child_agent_turn(
    session_id: &str,
    prompt: &str,
    output: &str,
    (assistant_id, user_id): (String, String),
) -> anyhow::Result<()> {
    domain::commit_collected_child_agent_turn::<Host>(
        session_id,
        prompt,
        output,
        (assistant_id, user_id),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn reserve_stream_agent_session(
    tenant: &TenantScope,
    project_id: &str,
    session_id: &str,
    directory: &str,
    created_ms: u64,
) -> anyhow::Result<()> {
    domain::reserve_stream_agent_session(
        tenant, project_id, session_id, directory, created_ms,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn chat_stream(
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(body): Json<GatewayChatStreamRequest>,
) -> Result<impl IntoResponse, ApiError> {
    domain::chat_stream::<Host>(
        Query(gateway_instance::host::InstanceQuery { directory: query.directory }),
        headers,
        Json(body),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn chat_stream_stop(
    Extension(tenant): Extension<TenantScope>,
    axum::extract::Path(request_id): axum::extract::Path<String>,
) -> Json<Value> {
    domain::chat_stream_stop(axum::Extension(tenant), axum::extract::Path(request_id)).await
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
async fn resolve_delegate_request_overrides(
    agent_name: Option<String>,
    allowed_tools: Option<Vec<String>>,
    explicit_model: Option<String>,
) -> Result<DelegateRequestOverrides, ApiError> {
    domain::resolve_delegate_request_overrides::<Host>(agent_name, allowed_tools, explicit_model)
        .await
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
async fn prepare_gateway_agent_request(
    body: &mut GatewayChatStreamRequest,
    background_full_access: Option<bool>,
) -> Result<DelegateRequestOverrides, ApiError> {
    domain::prepare_gateway_agent_request::<Host>(body, background_full_access).await
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
async fn session_model(
    body: &GatewayChatStreamRequest,
    binding: &agent_session::agent_binding::AgentBinding,
    session: &str,
) -> Result<Option<String>, ApiError> {
    domain::session_model::<Host>(body, binding, session).await
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
async fn start_gateway_chat_stream(
    tenant: TenantScope,
    dir: String,
    body: GatewayChatStreamRequest,
    background_full_access: Option<bool>,
    abort_token: Option<tokio_util::sync::CancellationToken>,
) -> Result<UnboundedReceiver<Value>, ApiError> {
    domain::start_gateway_chat_stream::<Host>(
        tenant,
        dir,
        body,
        background_full_access,
        abort_token,
    )
    .await
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
async fn start_gateway_chat_producer(
    tenant: TenantScope,
    dir: String,
    body: GatewayChatStreamRequest,
    background_full_access: Option<bool>,
    abort_token: Option<tokio_util::sync::CancellationToken>,
    prompt_cancel: Option<SharedPromptCancel>,
    send: impl FnMut(Value, bool) -> bool + Send + 'static,
) -> Result<(), ApiError> {
    domain::start_gateway_chat_producer::<Host>(
        tenant,
        dir,
        body,
        background_full_access,
        abort_token,
        prompt_cancel,
        send,
    )
    .await
}




#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn delegated_model_override(
    config: &crate::config::DelegateAgentConfig,
    explicit_model: Option<String>,
) -> Option<String> {
    domain::delegated_model_override::<Host>(config, explicit_model)
}

#[cfg(test)]
type SharedPromptCancel =
    std::sync::Arc<std::sync::Mutex<Option<session_port::PromptCancelHandle>>>;

#[cfg(test)]
#[path = "stream_tests.rs"]
mod stream_tests;
#[cfg(test)]
pub(crate) use stream_tests::collect_video_edit_test_stream;
