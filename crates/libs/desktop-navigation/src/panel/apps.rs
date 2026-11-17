//! Application-tab navigation and the cross-domain actions required after selection.
use crate::{components::top_bar::types::AppTab, ui_types::Screen};
pub enum Message {
    OpenApps,
    OpenUsage,
    OpenKnowledge,
    OpenWorkflowTool,
    OpenWorkflowManager,
    OpenThirdPartyAgents,
    AppsOpenMostRecent,
    AppsSearchChanged(String),
}
pub enum Effect {
    OpenUsage,
    OpenKnowledge,
    OpenWorkflowTool,
    OpenWorkflowManager,
    OpenThirdPartyAgents,
    OpenRecent(String),
}
pub struct Context<'a> {
    pub open_tabs: &'a mut Vec<AppTab>,
    pub active_tab_id: &'a mut Option<String>,
    pub screen: &'a mut Screen,
    pub apps_search_query: &'a mut String,
}
pub fn update(
    app: Context<'_>,
    workflow_tab_id: &str,
    recent_project: Option<&str>,
    message: Message,
) -> Option<Effect> {
    match message {
        Message::OpenApps => {
            let id = "apps".to_string();
            if !(*app.open_tabs).iter().any(|t| t.id == id) {
                (*app.open_tabs).push(AppTab {
                    id: id.clone(),
                    title: "应用".to_string(),
                    screen: Screen::Apps,
                    project_path: None,
                });
            }
            (*app.active_tab_id) = Some(id);
            (*app.screen) = Screen::Apps;
            None
        }
        Message::OpenUsage => {
            let id = "usage".to_string();
            let insert_at =
                if (*app.open_tabs).first().map(|t| t.id.as_str()) == Some("home") { 1 } else { 0 };
            if let Some(pos) = (*app.open_tabs).iter().position(|t| t.id == id) {
                let tab = (*app.open_tabs).remove(pos);
                let i = insert_at.min((*app.open_tabs).len());
                (*app.open_tabs).insert(i, tab);
            } else {
                let i = insert_at.min((*app.open_tabs).len());
                (*app.open_tabs).insert(
                    i,
                    AppTab {
                        id: id.clone(),
                        title: "用量".to_string(),
                        screen: Screen::Usage,
                        project_path: None,
                    },
                );
            }
            (*app.active_tab_id) = Some(id);
            (*app.screen) = Screen::Usage;
            Some(Effect::OpenUsage)
        }
        Message::OpenKnowledge => {
            let id = "knowledge".to_string();
            if let Some(pos) = (*app.open_tabs).iter().position(|t| t.id == "apps") {
                (*app.open_tabs).remove(pos);
            }
            if !(*app.open_tabs).iter().any(|t| t.id == id) {
                (*app.open_tabs).push(AppTab {
                    id: id.clone(),
                    title: "知识库".to_string(),
                    screen: Screen::Knowledge,
                    project_path: None,
                });
            }
            (*app.active_tab_id) = Some(id);
            (*app.screen) = Screen::Knowledge;
            Some(Effect::OpenKnowledge)
        }
        Message::OpenWorkflowTool => {
            let id = workflow_tab_id.to_string();
            if let Some(pos) = (*app.open_tabs).iter().position(|t| t.id == "apps") {
                (*app.open_tabs).remove(pos);
            }
            if !(*app.open_tabs).iter().any(|t| t.id == id) {
                (*app.open_tabs).push(AppTab {
                    id: id.clone(),
                    title: "工作流".to_string(),
                    screen: Screen::WorkflowTool,
                    project_path: None,
                });
            }
            (*app.active_tab_id) = Some(id);
            (*app.screen) = Screen::WorkflowTool;
            Some(Effect::OpenWorkflowTool)
        }
        Message::OpenThirdPartyAgents => {
            let id = "third_party_agents".to_string();
            app.open_tabs.retain(|tab| tab.id != "apps");
            if !app.open_tabs.iter().any(|tab| tab.id == id) {
                app.open_tabs.push(AppTab { id: id.clone(), title: "第三方智能体".into(),
                    screen: Screen::ThirdPartyAgents, project_path: None });
            }
            *app.active_tab_id = Some(id);
            *app.screen = Screen::ThirdPartyAgents;
            Some(Effect::OpenThirdPartyAgents)
        }
        Message::OpenWorkflowManager => {
            let id = "workflow_manager".to_string();
            if let Some(pos) = (*app.open_tabs).iter().position(|t| t.id == "apps") {
                (*app.open_tabs).remove(pos);
            }
            if !(*app.open_tabs).iter().any(|t| t.id == id) {
                (*app.open_tabs).push(AppTab {
                    id: id.clone(),
                    title: "任务编排".to_string(),
                    screen: Screen::WorkflowManager,
                    project_path: None,
                });
            }
            (*app.active_tab_id) = Some(id);
            (*app.screen) = Screen::WorkflowManager;
            Some(Effect::OpenWorkflowManager)
        }
        Message::AppsOpenMostRecent => {
            if let Some(path) = recent_project.map(str::to_owned) {
                // Close apps tab immediately
                if let Some(pos) = (*app.open_tabs).iter().position(|t| t.id == "apps") {
                    (*app.open_tabs).remove(pos);
                }
                (*app.active_tab_id) = (*app.open_tabs).last().map(|t| t.id.clone());

                return Some(Effect::OpenRecent(path));
            }
            None
        }
        Message::AppsSearchChanged(v) => {
            (*app.apps_search_query) = v;
            None
        }
    }
}

#[cfg(test)]
#[path = "apps_tests.rs"]
mod tests;
