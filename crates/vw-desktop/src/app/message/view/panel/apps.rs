use super::ViewMessage;
use crate::app::views::workflow_manager::{WorkflowManagerMessage, WorkflowManagerTab};
use crate::app::{App, Message, Screen, state::UsageModelInfo};
use iced::Task;

pub(super) fn pick_usage_provider_id(candidates: &[String], model_id: &str) -> Option<String> {
    if candidates.is_empty() {
        return None;
    }
    let first_segment = model_id.split('/').next().unwrap_or_default();
    let first_segment_lower = first_segment.to_ascii_lowercase();
    if !first_segment.is_empty() {
        if let Some(v) = candidates.iter().find(|id| id.as_str() == first_segment) {
            return Some(v.clone());
        }
        if let Some(v) =
            candidates.iter().find(|id| id.to_ascii_lowercase().contains(&first_segment_lower))
        {
            return Some(v.clone());
        }
    }

    let model_lower = model_id.to_ascii_lowercase();
    let prefer = if model_lower.starts_with("gpt-") || model_lower.starts_with('o') {
        Some("openai")
    } else if model_lower.contains("claude") {
        Some("anthropic")
    } else if model_lower.contains("deepseek") {
        Some("deepseek")
    } else {
        None
    };
    if let Some(substr) = prefer
        && let Some(v) = candidates.iter().find(|id| id.to_ascii_lowercase().contains(substr))
    {
        return Some(v.clone());
    }
    if model_lower.contains("deepseek")
        && let Some(v) = candidates.iter().find(|id| id.to_ascii_lowercase().contains("openrouter"))
    {
        return Some(v.clone());
    }
    Some(candidates[0].clone())
}

pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    let open_workflow_manager_stage_actions =
        matches!(message, ViewMessage::OpenWorkflowManagerStageActions);
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
                async move {
                    use crate::app::provider::provider;

                    let parsed = match model {
                        Some(s) => {
                            let s: String = s;
                            if s.contains('/') {
                                let parsed = provider::parse_model(&s);
                                if provider::get_model(&parsed.provider_id, &parsed.model_id)
                                    .await
                                    .is_ok()
                                {
                                    Some(parsed)
                                } else {
                                    let providers = provider::list().await;
                                    let mut candidates = Vec::<String>::new();
                                    for (provider_id, info) in providers {
                                        if info.models.contains_key(&s) {
                                            candidates.push(provider_id);
                                        }
                                    }
                                    pick_usage_provider_id(&candidates, &s).map(|provider_id| {
                                        provider::ParsedModelRef {
                                            provider_id,
                                            model_id: s.clone(),
                                        }
                                    })
                                }
                            } else {
                                let providers = provider::list().await;
                                let mut candidates = Vec::<String>::new();
                                for (provider_id, info) in providers {
                                    if info.models.contains_key(&s) {
                                        candidates.push(provider_id);
                                    }
                                }
                                pick_usage_provider_id(&candidates, &s).map(|provider_id| {
                                    provider::ParsedModelRef { provider_id, model_id: s.clone() }
                                })
                            }
                        }
                        None => provider::default_model().await.ok(),
                    };
                    let Some(parsed) = parsed else {
                        return None;
                    };
                    let m =
                        provider::get_model(&parsed.provider_id, &parsed.model_id).await.ok()?;
                    let provider_name = provider::get_provider(&parsed.provider_id)
                        .await
                        .map(|p| p.name)
                        .unwrap_or_else(|| parsed.provider_id.clone());
                    Some(UsageModelInfo {
                        provider_id: parsed.provider_id,
                        provider_name,
                        model_id: m.id.clone(),
                        model_name: m.name,
                        context_limit: m.limit.context,
                        input_limit: vw_shared::provider::state::effective_input_window(&m.limit),
                        output_limit: m.limit.output,
                        limit_is_default: vw_shared::provider::state::is_default_model_limit(
                            &m.limit,
                        ),
                        cost_input_per_million: m.cost.input,
                        cost_output_per_million: m.cost.output,
                        cost_cache_read_per_million: m.cost.cache.read,
                        cost_cache_write_per_million: m.cost.cache.write,
                    })
                },
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
        ViewMessage::OpenWorkflowManager | ViewMessage::OpenWorkflowManagerStageActions => {
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
            if open_workflow_manager_stage_actions {
                let stage_actions_task = crate::app::views::workflow_manager::update_app(
                    app,
                    WorkflowManagerMessage::SelectTab(WorkflowManagerTab::StageActions),
                );
                Task::batch(vec![refresh_task, stage_actions_task])
            } else {
                refresh_task
            }
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
#[cfg(test)]
#[path = "apps_tests.rs"]
mod apps_tests;
