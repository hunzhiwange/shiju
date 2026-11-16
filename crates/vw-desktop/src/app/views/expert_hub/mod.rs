//! 桌面注入页面上下文并映射跨域事件。
use crate::app::{App, Message, Screen};
use desktop_expert_hub::views::workspace as domain;
use iced::Element;
fn input(app: &App) -> domain::ViewInput<'_> {
    domain::ViewInput {
        expert_hub: &app.expert_hub,
        shiju_apps: &app.shiju_apps,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
        project_path: &app.navigation_tabs.project_path,
        recent_projects: &app.recent_projects,
        has_project_chat: app.navigation_tabs.open_tabs.iter().any(|tab| {
            tab.project_path.as_deref() == app.navigation_tabs.project_path.as_deref()
                && tab.screen == Screen::Project
        }),
        screen: match app.navigation_tabs.screen {
            Screen::ExpertHub => domain::Screen::ExpertHub,
            Screen::ShijuSkills => domain::Screen::ShijuSkills,
            Screen::ShijuApps => domain::Screen::ShijuApps,
            _ => domain::Screen::Other,
        },
    }
}
fn map(event: domain::Message) -> Message {
    match event {
        domain::Message::SkillHub(m) => {
            Message::SkillHub(crate::app::message::skill_hub::map_domain(m))
        }
        domain::Message::Connectors(m) => {
            Message::SkillHub(crate::app::message::SkillHubMessage::Connectors(m))
        }
        domain::Message::ShijuApps(m) => Message::ShijuApps(m),
        domain::Message::Account(domain::AccountMessage::StartBrowserLogin) => {
            Message::Account(crate::app::message::AccountMessage::StartBrowserLogin)
        }
        domain::Message::View(domain::ViewMessage::OpenUrlExternal(url)) => {
            Message::View(crate::app::message::ViewMessage::OpenUrlExternal(url))
        }
        domain::Message::CopyCode(text) => Message::CopyCode(text),
        domain::Message::AppendToInput(text) => {
            crate::app::components::text_editor_context_menu::text_context_message(
                desktop_ui_controls::text_editor_context_menu::TextContextEffect::Append(text),
            )
        }
        domain::Message::Selection { session_id, message_idx, text_idx, text } => {
            Message::ChatMarkdownSelectionChanged { session_id, message_idx, text_idx, text }
        }
        domain::Message::None => Message::None,
    }
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    domain::view(&input(app)).map(map)
}
pub(crate) fn shiju_skills_view(app: &App) -> Element<'_, Message> {
    domain::shiju_skills_view(&input(app)).map(map)
}
pub(crate) mod shiju_apps {
    use super::*;
    pub(crate) fn view(app: &App) -> Element<'_, Message> {
        domain::shiju_apps::view(&input(app)).map(map)
    }
}

#[cfg(test)]
mod shiju_knowledge_reader_tests;
#[cfg(test)]
mod shiju_knowledge_view_tests;
