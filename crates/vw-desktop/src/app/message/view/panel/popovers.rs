//! 处理视图面板的外部打开、弹出层与使用量展示交互。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::ViewMessage;
use crate::app::{App, Message};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::{load_app_config, save_app_config};
use iced::Task;

/// 执行 update 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::ToggleModelPopover => {
            let new = !app.show_model_popover;
            app.show_model_popover = new;
            if new {
                app.show_mode_popover = false;
                app.show_send_mode_popover = false;
                app.show_acp_popover = false;
                app.show_reasoning_effort_popover = false;
                app.show_file_popover = false;
                app.show_usage_popover = false;
                app.show_session_tool_selector_popover = false;
                app.show_executor_popover = false;
                if !app.model_settings.loading && app.model_settings.providers.is_empty() {
                    return Task::done(Message::Settings(
                        crate::app::message::SettingsMessage::ModelsRefresh,
                    ));
                }
            }
            Task::none()
        }
        ViewMessage::ToggleModePopover => {
            let new = !app.show_mode_popover;
            app.show_mode_popover = new;
            if new {
                app.show_model_popover = false;
                app.show_send_mode_popover = false;
                app.show_acp_popover = false;
                app.show_reasoning_effort_popover = false;
                app.show_file_popover = false;
                app.show_usage_popover = false;
                app.show_session_tool_selector_popover = false;
                app.show_executor_popover = false;
            }
            Task::none()
        }
        ViewMessage::ToggleSendModePopover => {
            if !app.current_session_is_requesting() {
                app.show_send_mode_popover = false;
                return Task::none();
            }
            let new = !app.show_send_mode_popover;
            app.show_send_mode_popover = new;
            if new {
                app.show_model_popover = false;
                app.show_mode_popover = false;
                app.show_acp_popover = false;
                app.show_reasoning_effort_popover = false;
                app.show_file_popover = false;
                app.show_usage_popover = false;
                app.show_session_tool_selector_popover = false;
                app.show_executor_popover = false;
            }
            Task::none()
        }
        ViewMessage::ToggleFilePopover => {
            let new = !app.show_file_popover;
            app.show_file_popover = new;
            if new {
                app.show_model_popover = false;
                app.show_mode_popover = false;
                app.show_send_mode_popover = false;
                app.show_acp_popover = false;
                app.show_reasoning_effort_popover = false;
                app.show_usage_popover = false;
                app.show_session_tool_selector_popover = false;
                app.show_executor_popover = false;
            }
            Task::none()
        }
        ViewMessage::ToggleAcpPopover => {
            let new = !app.show_acp_popover;
            app.show_acp_popover = new;
            if new {
                app.show_model_popover = false;
                app.show_mode_popover = false;
                app.show_send_mode_popover = false;
                app.show_file_popover = false;
                app.show_usage_popover = false;
                app.show_reasoning_effort_popover = false;
                app.show_session_tool_selector_popover = false;
                app.show_session_actions_popover = false;
                app.show_executor_popover = false;
                if app.acp_agents.is_empty() {
                    return Task::perform(
                        async {
                            crate::app::config::load_enabled_acp_config_async().await.map(|cfg| {
                                let mut acp_agents = cfg.keys().cloned().collect::<Vec<_>>();
                                acp_agents.sort_by(|left, right| {
                                    let left_key = (left != "codex", left.as_str());
                                    let right_key = (right != "codex", right.as_str());
                                    left_key.cmp(&right_key)
                                });
                                acp_agents
                            })
                        },
                        Message::BootstrapAcpAgentsLoaded,
                    );
                }
            }
            Task::none()
        }
        ViewMessage::ToggleReasoningEffortPopover => {
            let new = !app.show_reasoning_effort_popover;
            app.show_reasoning_effort_popover = new;
            if new {
                app.show_model_popover = false;
                app.show_mode_popover = false;
                app.show_send_mode_popover = false;
                app.show_file_popover = false;
                app.show_acp_popover = false;
                app.show_usage_popover = false;
                app.show_session_tool_selector_popover = false;
                app.show_session_actions_popover = false;
                app.show_executor_popover = false;
            }
            Task::none()
        }
        ViewMessage::ToggleUsagePopover => {
            let new = !app.show_usage_popover;
            app.show_usage_popover = new;
            if new {
                app.show_model_popover = false;
                app.show_mode_popover = false;
                app.show_send_mode_popover = false;
                app.show_file_popover = false;
                app.show_acp_popover = false;
                app.show_reasoning_effort_popover = false;
                app.show_session_tool_selector_popover = false;
                app.show_session_actions_popover = false;
                app.show_executor_popover = false;
            }
            Task::none()
        }
        ViewMessage::ToggleSessionToolSelectorPopover => {
            let new = !app.show_session_tool_selector_popover;
            app.show_session_tool_selector_popover = new;
            if new {
                app.show_model_popover = false;
                app.show_mode_popover = false;
                app.show_send_mode_popover = false;
                app.show_file_popover = false;
                app.show_acp_popover = false;
                app.show_reasoning_effort_popover = false;
                app.show_usage_popover = false;
                app.show_session_actions_popover = false;
                app.show_executor_popover = false;
            }
            Task::none()
        }
        ViewMessage::ToggleSessionActionsPopover => {
            let new = !app.show_session_actions_popover;
            app.show_session_actions_popover = new;
            if new {
                app.show_mode_popover = false;
                app.show_model_popover = false;
                app.show_send_mode_popover = false;
                app.show_file_popover = false;
                app.show_acp_popover = false;
                app.show_reasoning_effort_popover = false;
                app.show_usage_popover = false;
                app.show_session_tool_selector_popover = false;
                app.show_executor_popover = false;
            }
            Task::none()
        }
        ViewMessage::ClosePopovers => {
            app.show_mode_popover = false;
            app.show_model_popover = false;
            app.show_send_mode_popover = false;
            app.show_file_popover = false;
            app.show_acp_popover = false;
            app.show_reasoning_effort_popover = false;
            app.show_usage_popover = false;
            app.show_session_tool_selector_popover = false;
            app.show_session_actions_popover = false;
            app.show_executor_popover = false;
            app.model_popover_hover = None;
            Task::none()
        }
        ViewMessage::CloseModelPopover => {
            app.show_model_popover = false;
            app.model_popover_hover = None;
            Task::none()
        }
        ViewMessage::CloseModePopover => {
            app.show_mode_popover = false;
            Task::none()
        }
        ViewMessage::CloseSendModePopover => {
            app.show_send_mode_popover = false;
            Task::none()
        }
        ViewMessage::CloseFilePopover => {
            app.show_file_popover = false;
            Task::none()
        }
        ViewMessage::CloseAcpPopover => {
            app.show_acp_popover = false;
            Task::none()
        }
        ViewMessage::CloseReasoningEffortPopover => {
            app.show_reasoning_effort_popover = false;
            Task::none()
        }
        ViewMessage::CloseUsagePopover => {
            app.show_usage_popover = false;
            Task::none()
        }
        ViewMessage::CloseExecutorPopover => {
            app.show_executor_popover = false;
            Task::none()
        }
        ViewMessage::ToggleExecutorPopover => {
            let new = !app.show_executor_popover;
            app.show_executor_popover = new;
            if new {
                app.show_mode_popover = false;
                app.show_model_popover = false;
                app.show_send_mode_popover = false;
                app.show_file_popover = false;
                app.show_acp_popover = false;
                app.show_reasoning_effort_popover = false;
                app.show_usage_popover = false;
                app.show_session_tool_selector_popover = false;
                app.show_session_actions_popover = false;
            }
            Task::none()
        }
        ViewMessage::SelectChatSendBehavior(behavior) => {
            app.chat_send_behavior = behavior;
            app.show_send_mode_popover = false;
            let should_send = app.current_session_is_requesting();
            #[cfg(target_arch = "wasm32")]
            {
                let save_task = Task::perform(
                    async move {
                        let mut cfg = crate::app::config::load_app_config_async().await?;
                        if let Some(obj) = cfg.as_object_mut() {
                            obj.insert(
                                "chat_send_behavior".to_string(),
                                serde_json::Value::String(behavior.as_str().to_string()),
                            );
                            obj.remove("dialogue_flow_follow_up_behavior");
                        } else {
                            cfg = serde_json::json!({
                                "chat_send_behavior": behavior.as_str(),
                            });
                        }
                        crate::app::config::save_app_config_async(cfg).await
                    },
                    |result| {
                        if let Err(error) = result {
                            tracing::warn!(target: "vw_desktop", error = %error, "failed to save chat send behavior");
                        }
                        Message::None
                    },
                );
                if should_send {
                    return Task::batch([
                        save_task,
                        Task::done(Message::Chat(crate::app::message::ChatMessage::SendPressed)),
                    ]);
                }
                return save_task;
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let mut cfg: serde_json::Value = load_app_config();
                if !cfg.is_object() {
                    cfg = serde_json::json!({
                        "chat_send_behavior": behavior.as_str(),
                    });
                } else {
                    let obj: &mut serde_json::Map<String, serde_json::Value> =
                        cfg.as_object_mut().expect("checked object above");
                    obj.insert(
                        "chat_send_behavior".to_string(),
                        serde_json::Value::String(behavior.as_str().to_string()),
                    );
                    obj.remove("dialogue_flow_follow_up_behavior");
                }
                save_app_config(&cfg);
                if should_send {
                    Task::done(Message::Chat(crate::app::message::ChatMessage::SendPressed))
                } else {
                    Task::none()
                }
            }
        }
        ViewMessage::ModelPopoverHoverChanged(v) => {
            app.model_popover_hover = v;
            Task::none()
        }
        _ => Task::none(),
    }
}

#[cfg(test)]
#[path = "popovers_tests.rs"]
mod popovers_tests;
