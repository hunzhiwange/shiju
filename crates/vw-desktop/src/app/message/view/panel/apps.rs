//! Application navigation effects are assembled with their owning domains here.
use super::ViewMessage;
use crate::app::{App, Message};
use desktop_navigation::panel::apps as domain;
use iced::Task;
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    let local = match message {
        ViewMessage::OpenApps => domain::Message::OpenApps,
        #[cfg(not(target_arch = "wasm32"))]
        ViewMessage::OpenFileEditor => {
            use crate::app::message::preview;
            use desktop_navigation::panel::tabs::EDITOR_TAB_ID;

            app.navigation_panels.show_system_settings = false;
            preview::dismiss_preview_popup_menus(app);
            preview::tab::ensure_standalone_tab(app);
            return super::tabs::update(app, ViewMessage::TabSelected(EDITOR_TAB_ID.to_owned()));
        }
        ViewMessage::OpenUsage => domain::Message::OpenUsage,
        ViewMessage::OpenKnowledge => domain::Message::OpenKnowledge,
        ViewMessage::OpenWorkflowTool => domain::Message::OpenWorkflowTool,
        ViewMessage::OpenThirdPartyAgents => domain::Message::OpenThirdPartyAgents,
        ViewMessage::OpenWorkflowManager => {
            return super::super::update(
                app,
                ViewMessage::OpenSystemSettingsTab(desktop_settings_shell::SystemTab::TaskWorkflow),
            );
        }
        ViewMessage::AppsOpenMostRecent => domain::Message::AppsOpenMostRecent,
        ViewMessage::AppsSearchChanged(value) => domain::Message::AppsSearchChanged(value),
        _ => return Task::none(),
    };
    let effect = domain::update(
        domain::Context {
            open_tabs: &mut app.navigation_tabs.open_tabs,
            active_tab_id: &mut app.navigation_tabs.active_tab_id,
            screen: &mut app.navigation_tabs.screen,
            apps_search_query: &mut app.navigation_tabs.apps_search_query,
        },
        crate::apps::workflow::WORKFLOW_TOOL_TAB_ID,
        app.workspace_state.recent_projects.first().map(String::as_str),
        local,
    );
    match effect {
        None => Task::none(),
        Some(domain::Effect::OpenThirdPartyAgents) => {
            app.navigation_panels.show_system_settings = false;
            Task::batch([
                crate::app::message::settings::update(
                    app,
                    crate::app::message::SettingsMessage::AppsCatalog(
                        crate::app::message::settings::AppsCatalogMessage::EnsureLoaded,
                    ),
                ),
                crate::app::components::third_party_settings::ensure_agent_providers(app),
                crate::app::views::usage::center::update(
                    app,
                    desktop_usage::center::event::Event::TodayRefresh,
                ),
            ])
        }
        Some(domain::Effect::OpenUsage) => {
            app.usage_center.close();
            app.usage_panel.usage_model_info = None;
            app.usage_panel.usage_step_raw_payloads.clear();
            app.usage_panel.usage_step_raw_loading.clear();
            app.usage_panel.usage_step_raw_modal = None;
            let runtime = app.current_session_runtime();
            let model = if runtime.auto_model { None } else { Some(runtime.model) };
            Task::perform(desktop_usage::model_info::load(model), |info| {
                Message::View(ViewMessage::UsageModelInfoLoaded(info))
            })
        }
        Some(domain::Effect::OpenKnowledge) => {
            crate::app::message::knowledge_tool::ensure_loaded(app)
        }
        Some(domain::Effect::OpenWorkflowTool) => crate::apps::workflow::ensure_initialized(app),
        Some(domain::Effect::OpenRecent(path)) => Task::perform(async {}, move |_| {
            Message::Project(crate::app::message::project::ProjectMessage::OpenRecentPressed(path))
        }),
    }
}
