//! 处理聊天流式会话事件。
//! 本模块把网关轮询和流式增量落到会话状态，避免 UI 层理解传输细节。

use super::ChatMessage;
use crate::app::session_gateway;
use crate::app::{App, Message};
use iced::Task;

fn clear_question_modal(app: &mut App) {
    app.question_modal_request_id = None;
    app.question_modal_request = None;
    app.question_modal_answers.clear();
    app.question_modal_custom.clear();
}

fn apply_question_request(app: &mut App, requests: Vec<vw_shared::question::Request>) {
    let mut pending = requests;
    if pending.is_empty() {
        clear_question_modal(app);
        return;
    }
    pending.sort_by(|a, b| a.id.cmp(&b.id));
    let req = pending[0].clone();
    let reset = app.question_modal_request_id.as_deref() != Some(req.id.as_str());
    app.question_modal_request = Some(req.clone());
    if reset {
        app.question_modal_request_id = Some(req.id.clone());
        app.question_modal_answers = req
            .questions
            .iter()
            .map(|q| {
                let allow_multi = q.multiple.unwrap_or(false);
                if allow_multi {
                    return Vec::new();
                }
                if q.options.iter().any(|o| o.label == "once") {
                    return vec!["once".to_string()];
                }
                Vec::new()
            })
            .collect();
        app.question_modal_custom = vec![String::new(); req.questions.len()];
    } else {
        let qlen = req.questions.len();
        if app.question_modal_answers.len() != qlen {
            app.question_modal_answers.resize_with(qlen, Vec::new);
        }
        if app.question_modal_custom.len() != qlen {
            app.question_modal_custom.resize_with(qlen, String::new);
        }
    }
}

/// 模块内可见函数，执行 handle_question_poll_tick 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_question_poll_tick() -> Task<Message> {
    Task::perform(session_gateway::gateway_question_list_async(), |res| {
        Message::Chat(ChatMessage::QuestionListLoaded(res))
    })
}

/// 模块内可见函数，执行 handle_question_list_loaded 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_question_list_loaded(
    app: &mut App,
    res: Result<Vec<vw_shared::question::Request>, String>,
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
    let Some(req) = app.question_modal_request.as_ref() else {
        return Task::none();
    };
    let allow_multi = req.questions.get(q_idx).and_then(|q| q.multiple).unwrap_or(false);
    let Some(existing) = app.question_modal_answers.get_mut(q_idx) else {
        return Task::none();
    };
    if allow_multi {
        if let Some(pos) = existing.iter().position(|v| v == &label) {
            existing.remove(pos);
        } else {
            existing.push(label);
        }
    } else {
        existing.clear();
        existing.push(label);
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_question_custom_changed 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_question_custom_changed(
    app: &mut App,
    q_idx: usize,
    value: String,
) -> Task<Message> {
    let Some(existing) = app.question_modal_custom.get_mut(q_idx) else {
        return Task::none();
    };
    *existing = value;
    Task::none()
}

/// 模块内可见函数，执行 handle_question_submit 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_question_submit(app: &mut App) -> Task<Message> {
    let Some(request_id) = app.question_modal_request_id.clone() else {
        return Task::none();
    };
    let Some(req) = app.question_modal_request.clone() else {
        return Task::none();
    };

    let mut answers: Vec<Vec<String>> = Vec::with_capacity(req.questions.len());
    for i in 0..req.questions.len() {
        let allow_multi = req.questions.get(i).and_then(|q| q.multiple).unwrap_or(false);
        let allow_custom = req.questions.get(i).and_then(|q| q.custom).unwrap_or(false);
        let mut a = app.question_modal_answers.get(i).cloned().unwrap_or_default();
        let custom =
            app.question_modal_custom.get(i).cloned().unwrap_or_default().trim().to_string();
        if allow_custom && !custom.is_empty() {
            if allow_multi {
                if !a.contains(&custom) {
                    a.push(custom);
                }
            } else {
                a = vec![custom];
            }
        }
        answers.push(a);
    }

    if req.mcp_form.as_ref().is_some_and(|form| !crate::app::mcp_form::valid(form, &answers)) {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "请检查 MCP 表单的必填字段、类型与取值范围",
            vec![],
        );
    }
    clear_question_modal(app);
    Task::perform(
        async move { session_gateway::gateway_question_reply_async(&request_id, answers).await },
        |res| Message::Chat(ChatMessage::QuestionReplySubmitted(res)),
    )
}

/// 模块内可见函数，执行 handle_question_reject 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_question_reject(app: &mut App) -> Task<Message> {
    let Some(request_id) = app.question_modal_request_id.clone() else {
        return Task::none();
    };
    clear_question_modal(app);
    Task::perform(
        async move { session_gateway::gateway_question_reject_async(&request_id).await },
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
