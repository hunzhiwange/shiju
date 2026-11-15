//! 桌面注入安装视图所需的项目路径并映射消息。
use crate::app::{App, Message};
use iced::Element;
use desktop_expert_hub::views::install_guide::{self, InstallInput};
pub(super) use install_guide::{InstallTargetPresentation, install_target_presentations, target_path_label, target_root_label, direct_install_button_label};
fn input(app: &App) -> InstallInput<'_> { InstallInput { state: &app.expert_hub.install_guide, desktop_locale: app.desktop_locale, project_path: &app.project_path, recent_projects: &app.recent_projects } }
pub(super) fn install_project_options(app: &App) -> Vec<String> { install_guide::install_project_options(&app.project_path, &app.recent_projects) }
fn map(message: desktop_expert_hub::views::Message) -> Message {
 match message {
  desktop_expert_hub::views::Message::SkillHub(value) => Message::SkillHub(crate::app::message::skill_hub::map_domain(value)),
  desktop_expert_hub::views::Message::Connectors(value) => Message::SkillHub(crate::app::message::SkillHubMessage::Connectors(value)),
  desktop_expert_hub::views::Message::StartBrowserLogin => Message::Account(crate::app::message::AccountMessage::StartBrowserLogin),
        desktop_expert_hub::views::Message::None => Message::None,
 }
}
pub(super) fn view(app: &App) -> Element<'_, Message> { install_guide::view(&input(app)).map(map) }
pub(super) fn with_uninstall_confirmation<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
 match install_guide::uninstall_confirmation(&input(app)) {
 Some((close, card)) => desktop_ui_controls::system_settings_common::settings_modal_overlay(Some(base), map(close), card.map(map)),
 None => base,
 }
}
