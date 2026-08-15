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
    match message {
        ViewMessage::UsageModelInfoLoaded(info) => {
            app.usage_model_info = info;
            Task::none()
        }
        ViewMessage::UsageStepToggled(step_index) => {
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
            let key = (session_id.clone(), step_index);
            if app.usage_step_raw_payloads.contains_key(&key)
                || app.usage_step_raw_loading.contains(&key)
            {
                return Task::none();
            }
            app.usage_step_raw_loading.insert(key);
            Task::perform(
                async move {
                    let result = crate::app::session_gateway::gateway_session_step_llm_raw_async(
                        &session_id,
                        step_index,
                    )
                    .await;
                    (session_id, step_index, result)
                },
                |(session_id, step_index, result)| {
                    Message::View(ViewMessage::UsageStepRawLoaded(session_id, step_index, result))
                },
            )
        }
        ViewMessage::UsageStepRawLoaded(session_id, step_index, result) => {
            let key = (session_id.clone(), step_index);
            app.usage_step_raw_loading.remove(&key);
            if app.active_session_id.as_deref() == Some(session_id.as_str()) {
                if let Ok(Some(raw)) = &result {
                    app.active_session_view_state.llm_raw_step_indices.insert(step_index);
                    app.active_session_view_state.step_llm_raw.insert(step_index, raw.clone());
                }
                app.usage_step_raw_payloads.insert(key, result);
            }
            Task::none()
        }
        ViewMessage::UsageStepRawOpen(step_index) => {
            let Some(session_id) = app.active_session_id.as_deref() else {
                return Task::none();
            };
            let key = (session_id.to_string(), step_index);
            let Some(Ok(Some(value))) = app.usage_step_raw_payloads.get(&key) else {
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

#[cfg(test)]
#[path = "usage_tests.rs"]
mod usage_tests;
