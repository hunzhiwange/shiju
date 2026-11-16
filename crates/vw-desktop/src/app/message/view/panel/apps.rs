//! Application navigation effects are assembled with their owning domains here.
use super::ViewMessage;
use crate::app::{App, Message};
use desktop_navigation::panel::apps as domain;
use iced::Task;
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    let local = match message {
        ViewMessage::OpenApps => domain::Message::OpenApps,
        ViewMessage::OpenUsage => domain::Message::OpenUsage,
        ViewMessage::OpenKnowledge => domain::Message::OpenKnowledge,
        ViewMessage::OpenWorkflowTool => domain::Message::OpenWorkflowTool,
        ViewMessage::OpenWorkflowManager => domain::Message::OpenWorkflowManager,
        ViewMessage::AppsOpenMostRecent => domain::Message::AppsOpenMostRecent,
        ViewMessage::AppsSearchChanged(value) => domain::Message::AppsSearchChanged(value),
        _ => return Task::none(),
    };
    let effect = domain::update(
        domain::Context {
            open_tabs: &mut app.open_tabs,
            active_tab_id: &mut app.active_tab_id,
            screen: &mut app.screen,
            apps_search_query: &mut app.apps_search_query,
        },
        crate::apps::workflow::WORKFLOW_TOOL_TAB_ID,
        app.recent_projects.first().map(String::as_str),
        local,
    );
    match effect {
        None => Task::none(),
        Some(domain::Effect::OpenUsage) => {
            app.usage_center.close();
            app.usage_model_info = None;
            app.usage_step_raw_payloads.clear();
            app.usage_step_raw_loading.clear();
            app.usage_step_raw_modal = None;
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
        Some(domain::Effect::OpenWorkflowManager) => {
            app.workflow_manager =
                crate::app::views::workflow_manager::WorkflowManagerState::default();
            crate::app::views::workflow_manager::load_lifecycle_workflows_task(true)
        }
        Some(domain::Effect::OpenRecent(path)) => Task::perform(async {}, move |_| {
            Message::Project(crate::app::message::project::ProjectMessage::OpenRecentPressed(path))
        }),
    }
}
