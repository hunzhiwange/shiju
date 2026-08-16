//! 处理视图面板的外部打开、弹出层与使用量展示交互。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::ViewMessage;
use crate::app::state::UsageStepRawModal;
use crate::app::{App, Message};
use iced::Task;

/// 执行 update 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    let message = match message {
        ViewMessage::UsageStepToggled(step_index) => {
            let turn_id = turn_id_for_step(app, step_index);
            ViewMessage::UsageTurnStepToggled(turn_id, step_index)
        }
        ViewMessage::UsageStepRawLoaded(session_id, step_index, result) => {
            let turn_id = turn_id_for_step(app, step_index);
            ViewMessage::UsageTurnStepRawLoaded(session_id, turn_id, step_index, result)
        }
        ViewMessage::UsageStepRawOpen(step_index) => {
            let turn_id = turn_id_for_step(app, step_index);
            ViewMessage::UsageTurnStepRawOpen(turn_id, step_index)
        }
        message => message,
    };
    match message {
        ViewMessage::UsageModelInfoLoaded(info) => {
            app.usage_model_info = info;
            Task::none()
        }
        ViewMessage::UsageTurnStepToggled(turn_id, step_index) => {
            if app.usage_step_expanded.contains(&step_index) {
                app.usage_step_expanded.remove(&step_index);
                if app
                    .usage_step_raw_modal
                    .as_ref()
                    .is_some_and(|modal| modal.step_index == step_index)
                {
                    app.usage_step_raw_modal = None;
                }
                return Task::none();
            }
            app.usage_step_expanded.insert(step_index);
            let Some(session_id) = app.active_session_id.clone() else {
                return Task::none();
            };
            let key = (session_id.clone(), turn_id.clone(), step_index);
            if app.active_session_view_state.step_llm_raw.contains_key(&key)
                || app.usage_step_raw_payloads.contains_key(&key)
                || app.usage_step_raw_loading.contains(&key)
            {
                return Task::none();
            }
            app.usage_step_raw_loading.insert(key);
            Task::perform(
                async move {
                    let result = crate::app::session_gateway::gateway_session_step_llm_raw_async(
                        &session_id,
                        &turn_id,
                        step_index,
                    )
                    .await;
                    (session_id, turn_id, step_index, result)
                },
                |(session_id, turn_id, step_index, result)| {
                    Message::View(ViewMessage::UsageTurnStepRawLoaded(
                        session_id,
                        turn_id,
                        step_index,
                        result,
                    ))
                },
            )
        }
        ViewMessage::UsageTurnStepRawLoaded(session_id, turn_id, step_index, result) => {
            let key = (session_id.clone(), turn_id, step_index);
            app.usage_step_raw_loading.remove(&key);
            if app.active_session_id.as_deref() == Some(session_id.as_str()) {
                match result {
                    Ok(Some(raw)) => {
                        app.active_session_view_state.llm_raw_step_indices.insert(key.clone());
                        app.active_session_view_state.step_llm_raw.insert(key.clone(), raw);
                        app.usage_step_raw_payloads.remove(&key);
                    }
                    result => {
                        app.usage_step_raw_payloads.insert(key, result);
                    }
                }
            }
            Task::none()
        }
        ViewMessage::UsageTurnStepRawOpen(turn_id, step_index) => {
            let Some(session_id) = app.active_session_id.as_deref() else {
                return Task::none();
            };
            let key = (session_id.to_string(), turn_id.clone(), step_index);
            let Some(value) = app.active_session_view_state.step_llm_raw.get(&key) else {
                return Task::none();
            };
            let content = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
            app.usage_step_raw_modal = Some(UsageStepRawModal { step_index, content });
            Task::none()
        }
        ViewMessage::UsageStepRawClose => {
            app.usage_step_raw_modal = None;
            Task::none()
        }
        _ => Task::none(),
    }
}

fn turn_id_for_step(app: &App, step_index: u32) -> String {
    app.active_session_view_state
        .steps
        .iter()
        .find(|step| step.index == step_index)
        .and_then(|step| step.turn_id.clone())
        .unwrap_or_else(|| format!("legacy-step-{step_index}"))
}

#[cfg(test)]
#[path = "usage_tests.rs"]
mod usage_tests;
