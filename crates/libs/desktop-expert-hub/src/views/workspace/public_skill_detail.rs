//! 桌面注入只读领域输入并映射消息。
use crate::views::workspace::{Message, ViewInput as WorkspaceInput};
use crate::views::{self, public_skill_detail as domain};
use desktop_skill_hub::skill_hub::*;
use iced::Element;

fn map(message: views::Message) -> Message {
    match message {
        views::Message::SkillHub(value) => Message::SkillHub(value),
        views::Message::Connectors(value) => {
            Message::SkillHub(crate::views::workspace::SkillHubMessage::Connectors(value))
        }
        views::Message::StartBrowserLogin => {
            Message::Account(crate::views::workspace::AccountMessage::StartBrowserLogin)
        }
        views::Message::None => Message::None,
    }
}
pub(super) fn view<'a>(app: &WorkspaceInput<'a>) -> Element<'a, Message> {
    domain::view(&domain::DetailInput {
        expert_hub: &app.expert_hub,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
        project_path: &app.project_path,
        recent_projects: &app.recent_projects,
        selection: crate::message::public::selectable_expert_skill(
            &app.expert_hub.public_market,
            app.project_path.as_deref(),
            app.has_project_chat,
        )
        .map(|_| ()),
    })
    .map(map)
}
pub(super) fn installed_in_catalog(
    market: &PublicSkillMarketState,
    target: &PublicDetailTarget,
) -> bool {
    domain::installed_in_catalog(market, target)
}
pub(super) fn detail_scope_action_message(
    detail: &PublicDetailState,
    scope: SkillInstallScope,
    project_available: bool,
) -> Option<Message> {
    domain::detail_scope_action_message(detail, scope, project_available).map(map)
}
pub(super) fn detail_target_action_message(
    detail: &PublicDetailState,
    target: SkillInstallTarget,
) -> Option<Message> {
    domain::detail_target_action_message(detail, target).map(map)
}
pub(super) fn detail_project_action_message(
    selected_project_path: Option<&str>,
    install_busy: bool,
    project_path: String,
) -> Option<Message> {
    domain::detail_project_action_message(selected_project_path, install_busy, project_path)
        .map(map)
}
