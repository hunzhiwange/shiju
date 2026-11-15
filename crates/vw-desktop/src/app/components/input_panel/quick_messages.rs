use crate::app::{App, Message, message};
use desktop_chat_input::input_panel::quick_messages as domain;
fn input(app: &App) -> domain::QuickInput<'_> {
    let key = app.current_session_runtime_ref().and_then(|r| r.agent.as_deref()).unwrap_or(crate::app::state::MAIN_AGENT_KEY);
    domain::QuickInput {
        i18n: crate::app::i18n::I18n::new(app.desktop_locale),
        quick_messages: &app.quick_messages,
        quick_message_group: app.input_popovers.quick_message_group,
        recommended_prompt: app.input_popovers.recommended_prompt,
        recommended_prompts: app.agents_settings.entries.iter().find(|e| e.key == key).map(|e| e.recommended_prompts.as_slice()),
        show_workspace: app.workspace_mode == crate::app::state::WorkspaceMode::Work,
        project_is_git_repo: app.project_is_git_repo,
        recent_projects: &app.recent_projects,
        recent_projects_edits: &app.recent_projects_edits,
        project_path: app.project_path.as_deref(),
        work_space_picker_open: app.work_space_picker_open,
        #[cfg(not(target_arch = "wasm32"))]
        shiju_skill: app.input_popovers.shiju_skill,
        #[cfg(not(target_arch = "wasm32"))]
        installed: &app.expert_hub.installed,
        #[cfg(not(target_arch = "wasm32"))]
        package_status: &app.expert_hub.package_status,
        #[cfg(not(target_arch = "wasm32"))]
        tool_selector: app.current_session_runtime_ref().map(|r| &r.tool_selector),
    }
}
fn map_message(msg: desktop_chat_input::input_panel::quick_message::Message) -> Message {
    use desktop_chat_input::input_panel::quick_message::Message as Local;
    match msg {
        Local::QuickMessageSelected(value) => Message::Chat(message::ChatMessage::QuickMessageSelected(value)),
        Local::QuickMessageGroupChanged(value) => Message::Chat(message::ChatMessage::QuickMessageGroupChanged(value)),
        #[cfg(not(target_arch = "wasm32"))]
        Local::SelectShijuSkill(value) => Message::Chat(message::ChatMessage::SelectShijuSkill(value)),
        Local::AgentRecommendedPromptSelected(value) => Message::Chat(message::ChatMessage::AgentRecommendedPromptSelected(value)),
        Local::RecommendedPromptMenuChanged(value) => Message::Chat(message::ChatMessage::RecommendedPromptMenuChanged(value)),
        Local::SwitchWorkSpacePressed(value) => Message::Project(message::ProjectMessage::SwitchWorkSpacePressed(value)),
        #[cfg(not(target_arch = "wasm32"))]
        Local::ToggleShijuSkillMenu => Message::Chat(message::ChatMessage::ToggleShijuSkillMenu),
        #[cfg(not(target_arch = "wasm32"))]
        Local::RefreshInstalledSkills => Message::SkillHub(message::SkillHubMessage::RefreshInstalledSkills),
        #[cfg(not(target_arch = "wasm32"))]
        Local::OpenInstalledShijuSkills => Message::SkillHub(message::SkillHubMessage::OpenInstalledShijuSkills),
        #[cfg(not(target_arch = "wasm32"))]
        Local::PrepareNativeShijuSkill => Message::Account(message::AccountMessage::PrepareNativeShijuSkill),
        Local::WorkSpacePickerClosed => Message::View(message::ViewMessage::WorkSpacePickerClosed),
        Local::WorkSpacePickerToggled => Message::View(message::ViewMessage::WorkSpacePickerToggled),
        Local::OpenFolderPressed => Message::Project(message::ProjectMessage::OpenFolderPressed),
        Local::Batch(messages) => Message::Batch(messages.into_iter().map(map_message).collect()),
    }
}
pub(super) fn view(app: &App) -> Option<iced::Element<'_, Message>> {
    domain::view(input(app)).map(|view| view.map(map_message))
}

#[cfg(test)]
#[path = "quick_messages_tests.rs"]
mod quick_messages_tests;
