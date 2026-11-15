//! 处理应用面板导航，并为模型用量查询选择匹配的提供商。

use super::ViewMessage;
use crate::app::{App, Message, Screen};
use iced::Task;

/// 处理应用面板导航和模型用量刷新消息。
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::OpenApps => {
            let id = "apps".to_string();
            if !app.open_tabs.iter().any(|t| t.id == id) {
                app.open_tabs.push(crate::app::AppTab {
                    id: id.clone(),
                    title: "应用".to_string(),
                    screen: Screen::Apps,
                    project_path: None,
                });
            }
            app.active_tab_id = Some(id);
            app.screen = Screen::Apps;
            Task::none()
        }
        ViewMessage::OpenUsage => {
            app.usage_center.close();
            let id = "usage".to_string();
            let insert_at =
                if app.open_tabs.first().map(|t| t.id.as_str()) == Some("home") { 1 } else { 0 };
            if let Some(pos) = app.open_tabs.iter().position(|t| t.id == id) {
                let tab = app.open_tabs.remove(pos);
                let i = insert_at.min(app.open_tabs.len());
                app.open_tabs.insert(i, tab);
            } else {
                let i = insert_at.min(app.open_tabs.len());
                app.open_tabs.insert(
                    i,
                    crate::app::AppTab {
                        id: id.clone(),
                        title: "用量".to_string(),
                        screen: Screen::Usage,
                        project_path: None,
                    },
                );
            }
            app.active_tab_id = Some(id);
            app.screen = Screen::Usage;
            app.usage_model_info = None;
            app.usage_step_raw_payloads.clear();
            app.usage_step_raw_loading.clear();
            app.usage_step_raw_modal = None;
            let runtime = app.current_session_runtime();
            let model = if runtime.auto_model { None } else { Some(runtime.model) };
            Task::perform(
                desktop_usage::model_info::load(model),
                |info| Message::View(ViewMessage::UsageModelInfoLoaded(info)),
            )
        }
        ViewMessage::OpenKnowledge => {
            let id = "knowledge".to_string();
            if let Some(pos) = app.open_tabs.iter().position(|t| t.id == "apps") {
                app.open_tabs.remove(pos);
            }
            if !app.open_tabs.iter().any(|t| t.id == id) {
                app.open_tabs.push(crate::app::AppTab {
                    id: id.clone(),
                    title: "知识库".to_string(),
                    screen: Screen::Knowledge,
                    project_path: None,
                });
            }
            app.active_tab_id = Some(id);
            app.screen = Screen::Knowledge;
            crate::app::message::knowledge_tool::ensure_loaded(app)
        }
        ViewMessage::OpenWorkflowTool => {
            let id = crate::apps::workflow::WORKFLOW_TOOL_TAB_ID.to_string();
            if let Some(pos) = app.open_tabs.iter().position(|t| t.id == "apps") {
                app.open_tabs.remove(pos);
            }
            if !app.open_tabs.iter().any(|t| t.id == id) {
                app.open_tabs.push(crate::app::AppTab {
                    id: id.clone(),
                    title: "工作流".to_string(),
                    screen: Screen::WorkflowTool,
                    project_path: None,
                });
            }
            app.active_tab_id = Some(id);
            app.screen = Screen::WorkflowTool;
            crate::apps::workflow::ensure_initialized(app)
        }
        ViewMessage::OpenWorkflowManager => {
            let id = "workflow_manager".to_string();
            if let Some(pos) = app.open_tabs.iter().position(|t| t.id == "apps") {
                app.open_tabs.remove(pos);
            }
            app.workflow_manager =
                crate::app::views::workflow_manager::WorkflowManagerState::default();
            let refresh_task =
                crate::app::views::workflow_manager::load_lifecycle_workflows_task(true);
            if !app.open_tabs.iter().any(|t| t.id == id) {
                app.open_tabs.push(crate::app::AppTab {
                    id: id.clone(),
                    title: "任务编排".to_string(),
                    screen: Screen::WorkflowManager,
                    project_path: None,
                });
            }
            app.active_tab_id = Some(id);
            app.screen = Screen::WorkflowManager;
            refresh_task
        }
        ViewMessage::AppsOpenMostRecent => {
            if let Some(path) = app.recent_projects.first().cloned() {
                // Close apps tab immediately
                if let Some(pos) = app.open_tabs.iter().position(|t| t.id == "apps") {
                    app.open_tabs.remove(pos);
                }
                app.active_tab_id = app.open_tabs.last().map(|t| t.id.clone());

                return Task::perform(async {}, move |_| {
                    Message::Project(
                        crate::app::message::project::ProjectMessage::OpenRecentPressed(path),
                    )
                });
            }
            Task::none()
        }
        ViewMessage::AppsSearchChanged(v) => {
            app.apps_search_query = v;
            Task::none()
        }
        _ => Task::none(),
    }
}
