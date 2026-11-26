//! Popover host ports: model/ACP loading, preference saving, and chat submission.
use super::ViewMessage;
use crate::app::{App, Message};
use desktop_chat_input::popovers::Popover;
use desktop_navigation::panel::popovers as domain;
use iced::Task;
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    let message = match message {
        ViewMessage::SelectChatSendBehavior(behavior) => {
            app.session_state.chat_send_behavior = behavior;
            app.popover_visibility = desktop_chat_input::popovers::close(app.popover_visibility, Popover::SendMode);
            let save_task = app.save_desktop_preferences(serde_json::Map::from_iter([
                ("chat_send_behavior".to_owned(), serde_json::json!(behavior.as_str())),
                ("dialogue_flow_follow_up_behavior".to_owned(), serde_json::Value::Null),
            ]));
            return if app.current_session_is_requesting() {
                Task::batch([
                    save_task,
                    Task::done(Message::Chat(crate::app::message::ChatMessage::SendPressed)),
                ])
            } else {
                save_task
            };
        }
        ViewMessage::ToggleModelPopover => domain::Message::Toggle(Popover::Model),
        ViewMessage::ToggleModePopover => domain::Message::Toggle(Popover::Mode),
        ViewMessage::ToggleSendModePopover => domain::Message::Toggle(Popover::SendMode),
        ViewMessage::ToggleFilePopover => domain::Message::Toggle(Popover::File),
        ViewMessage::SelectFileMenuSection(section) => {
            use desktop_chat_input::popovers::FileMenuSection;
            use desktop_chat_tools::state::SessionToolSelectorTab;
            if !app.popover_visibility.file || app.popover_visibility.file_section == section {
                return Task::none();
            }
            app.popover_visibility.file_section = section;
            let tab = match section {
                Some(FileMenuSection::Agent) => SessionToolSelectorTab::Agent,
                Some(FileMenuSection::Tools) => SessionToolSelectorTab::Tools,
                Some(FileMenuSection::Experts | FileMenuSection::Skills) => SessionToolSelectorTab::Skills,
                _ => return Task::none(),
            };
            app.current_session_runtime_mut().tool_selector.set_query(String::new());
            return crate::app::message::chat::update(app, crate::app::message::ChatMessage::SessionToolSelectorTabSelected(tab));
        }
        ViewMessage::ToggleAcpPopover => domain::Message::Toggle(Popover::Acp),
        ViewMessage::ToggleUsagePopover => domain::Message::Toggle(Popover::Usage),
        ViewMessage::ToggleSessionToolSelectorPopover => {
            domain::Message::Toggle(Popover::SessionToolSelector)
        }
        ViewMessage::ToggleSessionActionsPopover => {
            domain::Message::Toggle(Popover::SessionActions)
        }
        ViewMessage::ToggleExecutorPopover => domain::Message::Toggle(Popover::Executor),
        ViewMessage::CloseModelPopover => domain::Message::Close(Popover::Model),
        ViewMessage::CloseModePopover => domain::Message::Close(Popover::Mode),
        ViewMessage::CloseSendModePopover => domain::Message::Close(Popover::SendMode),
        ViewMessage::CloseFilePopover => domain::Message::Close(Popover::File),
        ViewMessage::CloseAcpPopover => domain::Message::Close(Popover::Acp),
        ViewMessage::CloseUsagePopover => domain::Message::Close(Popover::Usage),
        ViewMessage::CloseExecutorPopover => domain::Message::Close(Popover::Executor),
        ViewMessage::ClosePopovers => domain::Message::CloseAll,
        ViewMessage::ToggleInputModelProvider(id) => domain::Message::ToggleProvider(id),
        ViewMessage::ModelPopoverHoverChanged(hover) => domain::Message::HoverChanged(hover),
        _ => return Task::none(),
    };
    let focus_model = matches!(&message, domain::Message::Toggle(Popover::Model))
        && !app.popover_visibility.model
        && app.mobile_layout.mode() != crate::app::state::LayoutMode::Mobile;
    let inputs = domain::Inputs {
        requesting: app.current_session_is_requesting(),
        models_empty: app.model_settings.providers.is_empty(),
        models_loading: app.model_settings.loading,
        acp_agents_empty: app.session_state.acp_agents.is_empty(),
    };
    let effect = domain::update(
        domain::Context {
            visibility: &mut app.popover_visibility,
            hover: &mut app.model_popover_hover,
            collapsed_providers: &mut app.model_settings.input_collapsed_provider_ids,
        },
        inputs,
        message,
    );
    let task = match effect {
        None => Task::none(),
        Some(domain::Effect::SaveCollapsedProviders(value)) => {
            app.save_desktop_preference("input_model_collapsed_provider_ids", value)
        }
        Some(domain::Effect::RefreshModelSources { load_models }) => {
            let source =
                Task::done(Message::Chat(crate::app::message::ChatMessage::AcpSourceRefresh));
            if load_models {
                Task::batch([
                    source,
                    Task::done(Message::Settings(
                        crate::app::message::SettingsMessage::ModelsRefresh,
                    )),
                ])
            } else {
                source
            }
        }
        Some(domain::Effect::LoadAcpAgents) => Task::perform(
            async {
                crate::app::config::load_enabled_acp_config_async().await.map(|cfg| {
                    let mut agents = cfg.keys().cloned().collect::<Vec<_>>();
                    agents.sort_by(|left, right| {
                        (left != "codex", left.as_str()).cmp(&(right != "codex", right.as_str()))
                    });
                    agents
                })
            },
            Message::BootstrapAcpAgentsLoaded,
        ),
    };
    if focus_model && app.popover_visibility.model {
        return Task::batch([
            task,
            iced::widget::operation::focus(
                desktop_chat_input::input_panel::model_views::model_search_id(),
            ),
        ]);
    }
    task
}
#[cfg(test)]
#[path = "popovers_tests.rs"]
mod popovers_tests;
