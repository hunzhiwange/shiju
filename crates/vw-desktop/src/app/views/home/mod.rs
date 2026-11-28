//! 首页宿主字段及跨域导航消息适配。
mod common;

pub(in crate::app) use common::{parse_hex_color, project_avatar};
use crate::app::{App, Message, message};

#[cfg(not(target_arch = "wasm32"))]
use crate::app::skill_hub::ExpertHubSection;

pub fn view(app: &App) -> iced::Element<'_, Message> {
    let state = desktop_navigation::views::home::recent::State {
        desktop_locale: app.general_settings.desktop_locale,
        window_width: app.window_size.0,
        recent_projects: &app.workspace_state.recent_projects,
        recent_projects_edits: &app.workspace_state.recent_projects_edits,
        recent_projects_meta: &app.workspace_state.recent_projects_meta,
        search_query: app.navigation_tabs.home_recent_project_search.as_deref(),
        project_menu: app.navigation_tabs.home_recent_project_menu.as_ref(),
        project_path: app.navigation_tabs.project_path.as_deref(),
    };
    desktop_navigation::views::home::view(&state, app.navigation_tabs.home_apps_bar_scroll_id.clone())
        .map(|event| {
            use desktop_navigation::views::home::{Message as Home, header, recent};
            match event {
                Home::Header(header::Message::OpenFolderPressed) =>
                    Message::Project(message::ProjectMessage::OpenFolderPressed),
                Home::Header(header::Message::OpenProjectPressed) =>
                    Message::Project(message::ProjectMessage::OpenProjectPressed),
                Home::Header(header::Message::OpenProjects) =>
                    Message::View(message::ViewMessage::OpenProjects),
                Home::Recent(recent::Message::OpenCron) =>
                    Message::View(message::ViewMessage::OpenCron),
                Home::Header(header::Message::OpenApps) =>
                    Message::View(message::ViewMessage::OpenApps),
                #[cfg(not(target_arch = "wasm32"))]
                Home::Header(header::Message::OpenFileEditor) =>
                    Message::View(message::ViewMessage::OpenFileEditor),
                Home::Recent(recent::Message::OpenThirdPartyAgents) =>
                    Message::View(message::ViewMessage::OpenThirdPartyAgents),
                Home::Recent(recent::Message::OpenAgentSettings) =>
                    Message::View(message::ViewMessage::OpenSystemSettingsTab(
                        crate::app::components::system_settings::SystemTab::Agents,
                    )),
                #[cfg(not(target_arch = "wasm32"))]
                Home::Recent(recent::Message::OpenTaskPool) =>
                    Message::TaskBoard(message::TaskBoardMessage::OpenFromHome),
                #[cfg(not(target_arch = "wasm32"))]
                Home::Recent(recent::Message::OpenCliWorkbench) =>
                    Message::CliWorkbench(crate::app::cli_workbench::Event::Open {
                        backend: None,
                        native_app: None,
                    }),
                #[cfg(not(target_arch = "wasm32"))]
                Home::Recent(recent::Message::OpenGitWorkbench) =>
                    Message::GitWorkbench(crate::app::git_workbench::Event::Open),
                #[cfg(not(target_arch = "wasm32"))]
                Home::Recent(recent::Message::OpenShijuSkills) =>
                    Message::SkillHub(message::SkillHubMessage::OpenShijuSkills),
                #[cfg(not(target_arch = "wasm32"))]
                Home::Recent(recent::Message::OpenShijuApps) =>
                    Message::ShijuApps(message::ShijuAppsMessage::Open),
                #[cfg(not(target_arch = "wasm32"))]
                Home::Header(header::Message::OpenShijuKnowledge) =>
                    Message::SkillHub(message::SkillHubMessage::SelectSection(ExpertHubSection::Knowledge)),
                #[cfg(not(target_arch = "wasm32"))]
                Home::Recent(recent::Message::OpenDigitalEmployees) =>
                    Message::DigitalEmployees(crate::app::digital_employees::Event::Open),
                #[cfg(not(target_arch = "wasm32"))]
                Home::Header(header::Message::OpenExperts) =>
                    Message::SkillHub(message::SkillHubMessage::SelectSection(ExpertHubSection::Experts)),
                #[cfg(not(target_arch = "wasm32"))]
                Home::Header(header::Message::OpenSkills) =>
                    Message::SkillHub(message::SkillHubMessage::SelectSection(ExpertHubSection::Skills)),
                #[cfg(not(target_arch = "wasm32"))]
                Home::Header(header::Message::OpenConnectors) =>
                    Message::SkillHub(message::SkillHubMessage::SelectSection(ExpertHubSection::Connectors)),
                Home::Header(header::Message::HomeAppsBarScrollChanged(x)) =>
                    Message::View(message::ViewMessage::HomeAppsBarScrollChanged(x)),
                Home::Recent(recent::Message::OpenRecentPressed(path)) =>
                    Message::Project(message::ProjectMessage::OpenRecentPressed(path)),
                Home::Recent(recent::Message::SearchToggled) =>
                    Message::View(message::ViewMessage::HomeRecentProjectSearchToggled),
                Home::Recent(recent::Message::SearchChanged(query)) =>
                    Message::View(message::ViewMessage::HomeRecentProjectSearchChanged(query)),
                Home::Recent(recent::Message::ProjectMenu(event)) =>
                    Message::View(message::ViewMessage::HomeRecentProjectMenu(event)),
            }
        })
}
