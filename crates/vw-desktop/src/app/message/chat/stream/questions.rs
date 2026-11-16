//! 处理聊天流式会话事件。
//! 本模块把网关轮询和流式增量落到会话状态，避免 UI 层理解传输细节。

use super::ChatMessage;
use desktop_session::session_gateway;
use crate::app::{App, Message};
use iced::Task;

fn clear_question_modal(app: &mut App) {
    desktop_session::message::chat::stream::questions::clear_question_modal(&mut question_state(
        app,
    ));
}

fn apply_question_request(app: &mut App, requests: Vec<question_types::Request>) {
    desktop_session::message::chat::stream::questions::apply_question_request(
        &mut question_state(app),
        requests,
    );
}

/// 模块内可见函数，执行 handle_question_poll_tick 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_question_poll_tick() -> Task<Message> {
    Task::perform(async move { session_gateway::gateway_question_list_async(crate::app::config::gateway_client_endpoint()).await }, |res| {
        Message::Chat(ChatMessage::QuestionListLoaded(res))
    })
}

/// 模块内可见函数，执行 handle_question_list_loaded 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_question_list_loaded(
    app: &mut App,
    res: Result<Vec<question_types::Request>, String>,
) -> Task<Message> {
    match res {
        Ok(requests) => apply_question_request(app, requests),
        Err(err) => {
            tracing::warn!(target: "vw_desktop", error = %err, "failed to load question list via gateway");
            clear_question_modal(app);
        }
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_question_option_toggled 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_question_option_toggled(
    app: &mut App,
    q_idx: usize,
    label: String,
) -> Task<Message> {
    desktop_session::message::chat::stream::questions::handle_question_option_toggled(
        &mut question_state(app),
        q_idx,
        label,
    );
    Task::none()
}

/// 模块内可见函数，执行 handle_question_custom_changed 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_question_custom_changed(
    app: &mut App,
    q_idx: usize,
    value: String,
) -> Task<Message> {
    desktop_session::message::chat::stream::questions::handle_question_custom_changed(
        &mut question_state(app),
        q_idx,
        value,
    );
    Task::none()
}

/// 模块内可见函数，执行 handle_question_submit 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_question_submit(app: &mut App) -> Task<Message> {
    let prepared = desktop_session::message::chat::stream::questions::prepare_reply(
        &mut question_state(app),
        |req, answers| {
            req.mcp_form.as_ref().is_none_or(|form| crate::app::mcp_form::valid(form, answers))
        },
    );
    let (request_id, answers) = match prepared {
        Ok(Some(reply)) => reply,
        Ok(None) => return Task::none(),
        Err(()) => {
            return app.show_localized_toast(
                crate::app::state::ToastKind::Error,
                "请检查 MCP 表单的必填字段、类型与取值范围",
                vec![],
            );
        }
    };
    Task::perform(
        async move { session_gateway::gateway_question_reply_async(crate::app::config::gateway_client_endpoint(), &request_id, answers).await },
        |res| Message::Chat(ChatMessage::QuestionReplySubmitted(res)),
    )
}

/// 模块内可见函数，执行 handle_question_reject 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_question_reject(app: &mut App) -> Task<Message> {
    let Some(request_id) =
        desktop_session::message::chat::stream::questions::reject_request(&mut question_state(app))
    else {
        return Task::none();
    };
    Task::perform(
        async move { session_gateway::gateway_question_reject_async(crate::app::config::gateway_client_endpoint(), &request_id).await },
        |res| Message::Chat(ChatMessage::QuestionRejected(res)),
    )
}

/// 模块内可见函数，执行 handle_question_reply_submitted 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_question_reply_submitted(res: Result<(), String>) -> Task<Message> {
    if let Err(err) = res {
        tracing::warn!(target: "vw_desktop", error = %err, "failed to submit question reply via gateway");
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_question_rejected 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_question_rejected(res: Result<(), String>) -> Task<Message> {
    if let Err(err) = res {
        tracing::warn!(target: "vw_desktop", error = %err, "failed to reject question via gateway");
    }
    Task::none()
}
#[cfg(test)]
#[path = "questions_tests.rs"]
mod questions_tests;

fn question_state(
    app: &mut App,
) -> desktop_session::message::chat::stream::questions::QuestionState<'_> {
    desktop_session::message::chat::stream::questions::QuestionState {
        question_modal_request_id: &mut app.session_state.question_modal_request_id,
        question_modal_request: &mut app.session_state.question_modal_request,
        question_modal_answers: &mut app.session_state.question_modal_answers,
        question_modal_custom: &mut app.session_state.question_modal_custom,
    }
}
