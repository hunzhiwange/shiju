//! Concrete prompt and hook runtime bindings for WebSocket chat.
use super::AppState;
use crate::agent::loop_::run_tool_call_loop;
use crate::approval::ApprovalManager;
use crate::security::SecurityPolicy;
use model_types::ChatMessage;
use std::sync::Arc;
use axum::{extract::{State, WebSocketUpgrade}, http::HeaderMap, response::IntoResponse};
#[cfg(test)]
use agent_gateway::ws::{WS_CHAT_SUBPROTOCOL, EMPTY_WS_RESPONSE_FALLBACK, extract_ws_auth_skey, parse_ws_private_event, parse_ws_delta_event, WsDeltaEvent};
#[cfg(test)]
use agent_loop::progress::{DRAFT_PROGRESS_SENTINEL, DRAFT_WS_EVENT_SENTINEL};
#[cfg(test)]
use axum::http::header;
#[cfg(test)]
fn extract_ws_bearer_token(headers: &HeaderMap) -> Option<String> { extract_ws_auth_skey(headers) }
struct Host;
impl agent_gateway::ws::GatewayWsHost for Host {
    type Sessions = super::state::SessionQueryEngineStore;
    type Channels = super::state::GatewayChannels;
    type Hooks = crate::hooks::HookRuntime;
    fn initial_history(state: &AppState) -> anyhow::Result<Vec<ChatMessage>> {
    // 为此 WebSocket 会话维护对话历史
    let mut history: Vec<ChatMessage> = Vec::new();

    let timezone_name = state.config.lock().app_ui.system_settings.timezone.clone();
    let timezone = crate::util::locale::parse_timezone(&timezone_name)?;

    // 为会话构建一次系统提示
    let (system_prompt, protocol_messages) = {
        let config_guard = state.config.lock();
        let system_prompt = crate::app::agent::channels::build_system_prompt(
            &config_guard.workspace_dir,
            &state.model,
            &[],
            &[],
            Some(&config_guard.identity),
            None,
            timezone,
        );
        let skills = crate::app::agent::skills::load_skills_with_config(
            &config_guard.workspace_dir,
            &config_guard,
        );
        let skill_tool_enabled =
            state.tools_registry_exec.iter().any(|tool| tool.name().eq_ignore_ascii_case("skill"));
        let protocol_messages = crate::app::agent::channels::build_workspace_protocol_messages(
            &config_guard.workspace_dir,
            &skills,
            skill_tool_enabled,
            8_192,
        );
        (system_prompt, protocol_messages)
    };

    // 临时协议消息不在 WebSocket 输出中可见；它们排在 system/history 之前，
    // 与 session processor 的 AGENTS、技能目录、真实历史顺序一致。
    history.extend(protocol_messages);
    history.push(ChatMessage::system(&system_prompt));

    Ok(history)
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
