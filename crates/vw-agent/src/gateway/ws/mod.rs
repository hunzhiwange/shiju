//! Concrete prompt and hook runtime bindings for WebSocket chat.
use super::AppState;
use crate::agent::loop_::run_tool_call_loop;
use crate::approval::ApprovalManager;
use crate::security::SecurityPolicy;
use model_types::ChatMessage;
use std::sync::Arc;
use axum::{extract::{State, WebSocketUpgrade}, http::HeaderMap, response::IntoResponse};
struct Host;
impl agent_gateway::ws::GatewayWsHost for Host {
    type Sessions = super::state::SessionQueryEngineStore;
    type Channels = super::state::GatewayChannels;
    type Hooks = crate::hooks::HookRuntime;
    fn initial_history(state: &AppState) -> anyhow::Result<Vec<ChatMessage>> {
        agent_gateway::ws::initial_history(state, |config, skill_tool_enabled| {
            let skills = crate::skills::load_skills_with_config(&config.workspace_dir, config);
            crate::channels::build_workspace_protocol_messages(
                &config.workspace_dir, &skills, skill_tool_enabled, 8_192,
            )
        })
    }
    async fn run_loop<'a>(state: &'a AppState, history: &'a mut Vec<ChatMessage>, provider: &'a str, approval: Option<Arc<ApprovalManager>>, security: Option<Arc<SecurityPolicy>>, delta: Option<tokio::sync::mpsc::Sender<String>>) -> anyhow::Result<String> {
        run_websocket_tool_loop(state, history, provider, approval, security, delta).await
    }
}
#[allow(clippy::too_many_arguments)]
async fn run_websocket_tool_loop(
    state: &AppState,
    history: &mut Vec<ChatMessage>,
    provider_label: &str,
    approval_manager: Option<Arc<ApprovalManager>>,
    security: Option<Arc<SecurityPolicy>>,
    delta_tx: Option<tokio::sync::mpsc::Sender<String>>,
) -> anyhow::Result<String> {
    run_tool_call_loop(
        state.provider.as_ref(),
        history,
        state.tools_registry_exec.as_ref(),
        state.observer.as_ref(),
        provider_label,
        &state.model,
        state.temperature,
        true,
        approval_manager,
        "webchat",
        &state.multimodal,
        state.max_iterations,
        None,
        delta_tx,
        state.hook_runtime.as_ref().and_then(|runtime| runtime.runner_snapshot()),
        security,
        &[],
    )
    .await
}

pub async fn handle_ws_chat(state: State<AppState>, headers: HeaderMap, ws: WebSocketUpgrade) -> impl IntoResponse {
    agent_gateway::ws::handle_ws_chat::<Host>(state, headers, ws).await
}
#[cfg(test)]
#[path = "../../../../libs/agent-gateway/src/ws/mod_tests.rs"]
mod mod_tests;
