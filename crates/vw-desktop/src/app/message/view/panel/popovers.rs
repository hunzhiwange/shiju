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
        ViewMessage::ToggleInputModelProvider(provider_id) => {
            let collapsed = &mut app.model_settings.input_collapsed_provider_ids;
            if !collapsed.insert(provider_id.clone()) {
                collapsed.remove(&provider_id);
            }
            let value = serde_json::Value::Array(
                collapsed.iter().cloned().map(serde_json::Value::String).collect(),
            );
            app.model_popover_hover = None;
            app.save_desktop_preference("input_model_collapsed_provider_ids", value)
        }
        ViewMessage::ToggleModelPopover => {
            let visibility = desktop_chat_input::popovers::toggle(
                visibility(app), desktop_chat_input::popovers::Popover::Model,
            );
            apply_visibility(app, visibility);
            let new = app.show_model_popover;
            if new {
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
            let visibility = desktop_chat_input::popovers::toggle(
                visibility(app), desktop_chat_input::popovers::Popover::Mode,
            );
            apply_visibility(app, visibility);
            Task::none()
        }
        ViewMessage::ToggleSendModePopover => {
            if !app.current_session_is_requesting() {
                app.show_send_mode_popover = false;
                return Task::none();
            }
            let visibility = desktop_chat_input::popovers::toggle(
                visibility(app), desktop_chat_input::popovers::Popover::SendMode,
            );
            apply_visibility(app, visibility);
            Task::none()
        }
        ViewMessage::ToggleFilePopover => {
            let visibility = desktop_chat_input::popovers::toggle(
                visibility(app), desktop_chat_input::popovers::Popover::File,
            );
            apply_visibility(app, visibility);
            Task::none()
        }
        ViewMessage::ToggleAcpPopover => {
            let visibility = desktop_chat_input::popovers::toggle(
                visibility(app), desktop_chat_input::popovers::Popover::Acp,
            );
            apply_visibility(app, visibility);
            let new = app.show_acp_popover;
            if new {
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
            let visibility = desktop_chat_input::popovers::toggle(
                visibility(app), desktop_chat_input::popovers::Popover::ReasoningEffort,
            );
            apply_visibility(app, visibility);
            Task::none()
        }
        ViewMessage::ToggleUsagePopover => {
            let visibility = desktop_chat_input::popovers::toggle(
                visibility(app), desktop_chat_input::popovers::Popover::Usage,
            );
            apply_visibility(app, visibility);
            Task::none()
        }
        ViewMessage::ToggleSessionToolSelectorPopover => {
            let visibility = desktop_chat_input::popovers::toggle(
                visibility(app), desktop_chat_input::popovers::Popover::SessionToolSelector,
            );
            apply_visibility(app, visibility);
            Task::none()
        }
        ViewMessage::ToggleSessionActionsPopover => {
            let visibility = desktop_chat_input::popovers::toggle(
                visibility(app), desktop_chat_input::popovers::Popover::SessionActions,
            );
            apply_visibility(app, visibility);
            Task::none()
        }
        ViewMessage::ClosePopovers => {
            apply_visibility(app, desktop_chat_input::popovers::Visibility::default());
            app.model_popover_hover = None;
            Task::none()
        }
        ViewMessage::CloseModelPopover => {
            let next = desktop_chat_input::popovers::close(visibility(app), desktop_chat_input::popovers::Popover::Model);
            apply_visibility(app, next);
            app.model_popover_hover = None;
            Task::none()
        }
        ViewMessage::CloseModePopover => {
            let next = desktop_chat_input::popovers::close(visibility(app), desktop_chat_input::popovers::Popover::Mode);
            apply_visibility(app, next);
            Task::none()
        }
        ViewMessage::CloseSendModePopover => {
            let next = desktop_chat_input::popovers::close(visibility(app), desktop_chat_input::popovers::Popover::SendMode);
            apply_visibility(app, next);
            Task::none()
        }
        ViewMessage::CloseFilePopover => {
            let next = desktop_chat_input::popovers::close(visibility(app), desktop_chat_input::popovers::Popover::File);
            apply_visibility(app, next);
            Task::none()
        }
        ViewMessage::CloseAcpPopover => {
            let next = desktop_chat_input::popovers::close(visibility(app), desktop_chat_input::popovers::Popover::Acp);
            apply_visibility(app, next);
            Task::none()
        }
        ViewMessage::CloseReasoningEffortPopover => {
            let next = desktop_chat_input::popovers::close(visibility(app), desktop_chat_input::popovers::Popover::ReasoningEffort);
            apply_visibility(app, next);
            Task::none()
        }
        ViewMessage::CloseUsagePopover => {
            let next = desktop_chat_input::popovers::close(visibility(app), desktop_chat_input::popovers::Popover::Usage);
            apply_visibility(app, next);
            Task::none()
        }
        ViewMessage::CloseExecutorPopover => {
            let next = desktop_chat_input::popovers::close(visibility(app), desktop_chat_input::popovers::Popover::Executor);
            apply_visibility(app, next);
            Task::none()
        }
        ViewMessage::ToggleExecutorPopover => {
            let visibility = desktop_chat_input::popovers::toggle(
                visibility(app), desktop_chat_input::popovers::Popover::Executor,
            );
            apply_visibility(app, visibility);
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

fn visibility(app: &App) -> desktop_chat_input::popovers::Visibility {
    desktop_chat_input::popovers::Visibility {
        model: app.show_model_popover,
        mode: app.show_mode_popover,
        send_mode: app.show_send_mode_popover,
        file: app.show_file_popover,
        acp: app.show_acp_popover,
        reasoning_effort: app.show_reasoning_effort_popover,
        usage: app.show_usage_popover,
        session_tool_selector: app.show_session_tool_selector_popover,
        session_actions: app.show_session_actions_popover,
        executor: app.show_executor_popover,
    }
}

fn apply_visibility(app: &mut App, visibility: desktop_chat_input::popovers::Visibility) {
    app.show_model_popover = visibility.model;
    app.show_mode_popover = visibility.mode;
    app.show_send_mode_popover = visibility.send_mode;
    app.show_file_popover = visibility.file;
    app.show_acp_popover = visibility.acp;
    app.show_reasoning_effort_popover = visibility.reasoning_effort;
    app.show_usage_popover = visibility.usage;
    app.show_session_tool_selector_popover = visibility.session_tool_selector;
    app.show_session_actions_popover = visibility.session_actions;
    app.show_executor_popover = visibility.executor;
}
