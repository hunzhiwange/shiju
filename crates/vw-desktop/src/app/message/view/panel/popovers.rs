//! 处理视图面板的外部打开、弹出层与使用量展示交互。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::ViewMessage;
use crate::app::{App, Message};
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
                let source =
                    Task::done(Message::Chat(crate::app::message::ChatMessage::AcpSourceRefresh));
                if !app.model_settings.loading && app.model_settings.providers.is_empty() {
                    return Task::batch([
                        source,
                        Task::done(Message::Settings(
                            crate::app::message::SettingsMessage::ModelsRefresh,
                        )),
                    ]);
                }
                return source;
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
            let save_task = app.save_desktop_preferences(serde_json::Map::from_iter([
                ("chat_send_behavior".to_owned(), serde_json::json!(behavior.as_str())),
                ("dialogue_flow_follow_up_behavior".to_owned(), serde_json::Value::Null),
            ]));
            if app.current_session_is_requesting() {
                Task::batch([
                    save_task,
                    Task::done(Message::Chat(crate::app::message::ChatMessage::SendPressed)),
                ])
            } else {
                save_task
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
