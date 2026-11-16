//! 桌面注入安装视图所需的项目路径并映射消息。
use crate::views::install_guide::{self, InstallInput};
use crate::views::workspace::{Message, ViewInput as WorkspaceInput};
use iced::Element;
pub(super) use install_guide::{
    InstallTargetPresentation, direct_install_button_label, install_target_presentations,
    target_path_label, target_root_label,
};
fn input<'a>(app: &WorkspaceInput<'a>) -> InstallInput<'a> {
    InstallInput {
        state: &app.expert_hub.install_guide,
        desktop_locale: app.desktop_locale,
        project_path: &app.project_path,
        recent_projects: &app.recent_projects,
    }
}
pub(super) fn install_project_options(app: &WorkspaceInput) -> Vec<String> {
    install_guide::install_project_options(&app.project_path, &app.recent_projects)
}
fn map(message: crate::views::Message) -> Message {
    match message {
        crate::views::Message::SkillHub(value) => Message::SkillHub(value),
        crate::views::Message::Connectors(value) => {
            Message::SkillHub(crate::views::workspace::SkillHubMessage::Connectors(value))
        }
        crate::views::Message::StartBrowserLogin => {
            Message::Account(crate::views::workspace::AccountMessage::StartBrowserLogin)
        }
        crate::views::Message::None => Message::None,
    }
}
pub(super) fn view<'a>(app: &WorkspaceInput<'a>) -> Element<'a, Message> {
    install_guide::view(&input(app)).map(map)
}
pub(super) fn with_uninstall_confirmation<'a>(
    app: &WorkspaceInput<'a>,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    match install_guide::uninstall_confirmation(&input(app)) {
        Some((close, card)) => desktop_ui_controls::system_settings_common::settings_modal_overlay(
            Some(base),
            map(close),
            card.map(map),
        ),
        None => base,
    }
}
