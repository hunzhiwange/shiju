//! 处理聊天流式会话事件。
//! 本模块把网关轮询和流式增量落到会话状态，避免 UI 层理解传输细节。

use super::{ChatMessage, load_session_or_default, now_ms};
use crate::app::{App, Message, models};
use desktop_session::message::chat::stream::steps::{
    compute_step_cost, merge_tool_lifecycle, upsert_step_finish, upsert_step_start,
};
use gateway_chat_stream::GatewayChatToolEvent;
use iced::Task;

/// 模块内可见函数，执行 handle_agent_step_start 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn start_llm_timer_at(
    runtime: &mut crate::app::state::SessionRuntimeState,
    now: web_time::Instant,
) {
    desktop_session::message::chat::stream::steps::start_llm_timer_at(
        &mut runtime.request_lifecycle,
        now,
    );
}

pub(super) fn pause_llm_timer_at(
    runtime: &mut crate::app::state::SessionRuntimeState,
    now: web_time::Instant,
) {
    desktop_session::message::chat::stream::steps::pause_llm_timer_at(
        &mut runtime.request_lifecycle,
        now,
    );
}

pub(super) fn handle_agent_step_start(
    app: &mut App,
    id: u64,
    session_id: String,
    step_index: u32,
    created_ms: u64,
    model: Option<String>,
) -> Task<Message> {
    let Some(_found_session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };
    if session_id.is_empty() {
        return Task::none();
    }
    start_llm_timer_at(app.get_session_runtime_mut(&session_id), web_time::Instant::now());
    let mut session = load_session_or_default(app, session_id.clone());
    upsert_step_start(&mut session, step_index, created_ms, model, None);
    // 运行时负责持久化步骤；此处仅更新投影，避免整份快照与消息增量竞争。
    if app.active_session_id.as_ref() == Some(&session_id) {
        if let Some(model) = session
            .steps
            .iter()
            .find(|step| step.index == step_index)
            .and_then(|step| step.model.as_deref())
        {
            crate::app::session::bind_message_model_to_step(&mut app.chat, step_index, model);
        }
        app.active_session_view_state.updated_ms = session.updated_ms;
        if let Some(step) = session.steps.iter().find(|step| step.index == step_index) {
            app.upsert_active_session_step(step.clone());
        }
        app.rebuild_active_session_message_meta();
    } else if !session.messages.is_empty() {
        app.store_session_chat_snapshot(
            session_id.clone(),
            crate::app::session::shared_chat_messages(session.messages.clone()),
            session.message_ids.clone(),
        );
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_agent_step_finish 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_agent_step_finish(
    app: &mut App,
    id: u64,
    session_id: String,
    step_index: u32,
    finished_ms: u64,
    usage: models::TokenUsage,
    finish_reason: Option<String>,
    model: Option<String>,
) -> Task<Message> {
    let Some(_found_session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };
    if session_id.is_empty() {
        return Task::none();
    }
    pause_llm_timer_at(app.get_session_runtime_mut(&session_id), web_time::Instant::now());

    if app.active_session_id.as_ref() == Some(&session_id) {
        app.usage.input_tokens += usage.input_tokens;
        app.usage.output_tokens += usage.output_tokens;
        app.usage.cached_tokens += usage.cached_tokens;
        app.usage.reasoning_tokens += usage.reasoning_tokens;
    }

    let mut session = load_session_or_default(app, session_id.clone());
    upsert_step_finish(
        &mut session,
        step_index,
        finished_ms,
        usage.clone(),
        finish_reason,
        model.clone(),
    );
    // 运行时负责持久化步骤；此处仅更新投影，避免整份快照与消息增量竞争。
    if app.active_session_id.as_ref() == Some(&session_id) {
        if let Some(model) = session
            .steps
            .iter()
            .find(|step| step.index == step_index)
            .and_then(|step| step.model.as_deref())
        {
            crate::app::session::bind_message_model_to_step(&mut app.chat, step_index, model);
        }
        app.active_session_view_state.updated_ms = session.updated_ms;
        if let Some(step) = session.steps.iter().find(|step| step.index == step_index) {
            app.upsert_active_session_step(step.clone());
        }
        app.rebuild_active_session_message_meta();
    } else if !session.messages.is_empty() {
        app.store_session_chat_snapshot(
            session_id.clone(),
            crate::app::session::shared_chat_messages(session.messages.clone()),
            session.message_ids.clone(),
        );
    }

    let raw_refresh_task =
        super::super::trajectory::refresh_missing_raw_task(app, &session_id, Some(step_index));

    Task::batch(vec![
        Task::perform(compute_step_cost(model, usage), move |(resolved_model, cost)| {
            Message::Chat(ChatMessage::AgentStepCostLoaded(
                id,
                session_id,
                step_index,
                resolved_model,
                cost,
            ))
        }),
        raw_refresh_task,
    ])
}

pub(super) fn handle_agent_tool_lifecycle(
    app: &mut App,
    id: u64,
    event: &GatewayChatToolEvent,
) -> Task<Message> {
    let Some(session_id) = app.find_session_by_request_id(id) else {
        return Task::none();
    };
    let mut session = load_session_or_default(app, session_id.clone());
    merge_tool_lifecycle(&mut session, event);
    // 运行时负责持久化步骤；此处仅更新投影，避免整份快照与消息增量竞争。
    if app.active_session_id.as_ref() == Some(&session_id) {
        app.active_session_view_state.updated_ms = session.updated_ms;
        if let Some(step) = session.steps.iter().find(|step| step.index == event.step_index) {
            app.upsert_active_session_step(step.clone());
        }
    } else if !session.messages.is_empty() {
        app.store_session_chat_snapshot(
            session_id,
            crate::app::session::shared_chat_messages(session.messages.clone()),
            session.message_ids.clone(),
        );
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_agent_step_cost_loaded 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_agent_step_cost_loaded(
    app: &mut App,
    session_id: String,
    step_index: u32,
    resolved_model: Option<String>,
    cost: Option<f64>,
) -> Task<Message> {
    if session_id.is_empty() {
        return Task::none();
    }
    let mut session = load_session_or_default(app, session_id.clone());
    let message_model = resolved_model.clone();
    desktop_session::message::chat::stream::steps::apply_step_cost(
        &mut session,
        step_index,
        resolved_model,
        cost,
        now_ms(),
    );
    // 运行时负责持久化步骤；此处仅更新投影，避免整份快照与消息增量竞争。
    if app.active_session_id.as_ref() == Some(&session_id) {
        if let Some(model) = message_model.as_deref() {
            crate::app::session::bind_message_model_to_step(&mut app.chat, step_index, model);
        }
        app.active_session_view_state.updated_ms = session.updated_ms;
        if let Some(step) = session.steps.iter().find(|step| step.index == step_index) {
            app.upsert_active_session_step(step.clone());
        }
        app.rebuild_active_session_message_meta();
    } else if !session.messages.is_empty() {
        app.store_session_chat_snapshot(
            session_id.clone(),
            crate::app::session::shared_chat_messages(session.messages.clone()),
            session.message_ids.clone(),
        );
    }
    Task::none()
}
#[cfg(test)]
#[path = "steps_tests.rs"]
mod steps_tests;
