//! 工作侧栏的桌面输入与跨域消息适配。
use iced::{Element, Point};
use crate::app::{App, Message};
#[cfg(test)]
use crate::app::message::ProjectMessage;
use super::super::components::running_draft_sessions;
#[cfg(test)]
use super::super::components::build_session_menu;
pub(crate) use desktop_workspace::views::project::layout::work_sidebar as domain;
pub(crate) use domain::WorkSidebarModel;
#[cfg(test)]
pub(super) use domain::compare_sessions;
fn map(event: domain::events::Message) -> Message {
 use domain::events as e;
 match event {
 e::Message::Batch(events) => Message::Batch(events.into_iter().map(map).collect()),
 e::Message::Session(event) => super::super::components::sessions_panel::map(event),
e::Message::Project(event) => Message::Project(match event {
e::ProjectMessage::CurrentProjectMenuToggled(v0) => crate::app::message::ProjectMessage::CurrentProjectMenuToggled(v0),
e::ProjectMessage::OpenProjectSessionPressed(v0, v1) => crate::app::message::ProjectMessage::OpenProjectSessionPressed(v0, v1),
e::ProjectMessage::ProjectEditOpened(v0) => crate::app::message::ProjectMessage::ProjectEditOpened(v0),
e::ProjectMessage::ProjectLoadSessions(v0) => crate::app::message::ProjectMessage::ProjectLoadSessions(v0),
e::ProjectMessage::ProjectToolsMenuClosed => crate::app::message::ProjectMessage::ProjectToolsMenuClosed,
e::ProjectMessage::ProjectToolsMenuToggled(v0) => crate::app::message::ProjectMessage::ProjectToolsMenuToggled(v0),
e::ProjectMessage::RecentHovered(v0) => crate::app::message::ProjectMessage::RecentHovered(v0),
e::ProjectMessage::RecentRemovePressed(v0) => crate::app::message::ProjectMessage::RecentRemovePressed(v0),
e::ProjectMessage::RecentRevealPressed(v0) => crate::app::message::ProjectMessage::RecentRevealPressed(v0),
e::ProjectMessage::SessionMenuClose => crate::app::message::ProjectMessage::SessionMenuClose,
e::ProjectMessage::SessionRightClicked(v0, v1, v2) => crate::app::message::ProjectMessage::SessionRightClicked(v0, v1, v2),
e::ProjectMessage::SwitchWorkSpacePressed(v0) => crate::app::message::ProjectMessage::SwitchWorkSpacePressed(v0),
}),
e::Message::View(event) => Message::View(match event {
e::ViewMessage::OpenApps => crate::app::message::ViewMessage::OpenApps,
e::ViewMessage::OpenKnowledge => crate::app::message::ViewMessage::OpenKnowledge,
e::ViewMessage::OpenSystemSettingsTab(v0) => crate::app::message::ViewMessage::OpenSystemSettingsTab(match v0 {e::SystemTab::Agents => crate::app::components::system_settings::SystemTab::Agents,e::SystemTab::Cron => crate::app::components::system_settings::SystemTab::Cron,e::SystemTab::Projects => crate::app::components::system_settings::SystemTab::Projects,}),
e::ViewMessage::OpenWorkflowTool => crate::app::message::ViewMessage::OpenWorkflowTool,
e::ViewMessage::ProjectFileNewProject => crate::app::message::ViewMessage::ProjectFileNewProject,
e::ViewMessage::ProjectFileNewSession => crate::app::message::ViewMessage::ProjectFileNewSession,
e::ViewMessage::ToggleWebLinksMenu => crate::app::message::ViewMessage::ToggleWebLinksMenu,
e::ViewMessage::WorkProjectHome => crate::app::message::ViewMessage::WorkProjectHome,
e::ViewMessage::WorkSidebarFilterClosed => crate::app::message::ViewMessage::WorkSidebarFilterClosed,
e::ViewMessage::WorkSidebarFilterReset => crate::app::message::ViewMessage::WorkSidebarFilterReset,
e::ViewMessage::WorkSidebarFilterScopeSelected(v0) => crate::app::message::ViewMessage::WorkSidebarFilterScopeSelected(v0),
e::ViewMessage::WorkSidebarFilterStatusSelected(v0) => crate::app::message::ViewMessage::WorkSidebarFilterStatusSelected(v0),
e::ViewMessage::WorkSidebarFilterTimeSelected(v0) => crate::app::message::ViewMessage::WorkSidebarFilterTimeSelected(v0),
e::ViewMessage::WorkSidebarFilterToggled => crate::app::message::ViewMessage::WorkSidebarFilterToggled,
e::ViewMessage::WorkSidebarLoadMorePressed => crate::app::message::ViewMessage::WorkSidebarLoadMorePressed,
e::ViewMessage::WorkSidebarMoreClosed => crate::app::message::ViewMessage::WorkSidebarMoreClosed,
e::ViewMessage::WorkSidebarMoreOpened => crate::app::message::ViewMessage::WorkSidebarMoreOpened,
e::ViewMessage::WorkSidebarProjectNavHovered(v0) => crate::app::message::ViewMessage::WorkSidebarProjectNavHovered(v0),
e::ViewMessage::WorkSidebarSpaceTasksLoadMorePressed(v0) => crate::app::message::ViewMessage::WorkSidebarSpaceTasksLoadMorePressed(v0),
e::ViewMessage::WorkSidebarTabSelected(v0) => crate::app::message::ViewMessage::WorkSidebarTabSelected(v0),
e::ViewMessage::WorkSidebarToggled => crate::app::message::ViewMessage::WorkSidebarToggled,
e::ViewMessage::WorkSpaceToggled(v0) => crate::app::message::ViewMessage::WorkSpaceToggled(v0),
}),
#[cfg(not(target_arch = "wasm32"))]
e::Message::SkillHub(event) => Message::SkillHub(match event {
e::SkillHubMessage::Open => crate::app::message::SkillHubMessage::Open,
e::SkillHubMessage::OpenShijuSkills => crate::app::message::SkillHubMessage::OpenShijuSkills,
e::SkillHubMessage::SelectSection(v0) => crate::app::message::SkillHubMessage::SelectSection(match v0 {e::ExpertHubSection::Experts => crate::app::skill_hub::ExpertHubSection::Experts,e::ExpertHubSection::Knowledge => crate::app::skill_hub::ExpertHubSection::Knowledge,}),
}),
#[cfg(not(target_arch = "wasm32"))]
e::Message::ShijuApps(event) => Message::ShijuApps(match event {
e::ShijuAppsMessage::Open => crate::app::message::ShijuAppsMessage::Open,
}),
e::Message::Search(event) => Message::Search(match event {
e::SearchMessage::Open => crate::app::message::SearchMessage::Open,
}),
e::Message::TaskBoard(event) => Message::TaskBoard(match event {
e::TaskBoardMessage::ToggleBoard => crate::app::message::TaskBoardMessage::ToggleBoard,
}),
}
}
pub(super) fn cached_model(app: &App) -> std::sync::Arc<WorkSidebarModel> {
 let mut cache = app.session_ui_projection.sidebar.borrow_mut();
 cache.get_or_insert_with(|| std::sync::Arc::new(model(app))).clone()
}
pub(super) fn model(app: &App) -> WorkSidebarModel {
 let drafts = running_draft_sessions(app).collect::<Vec<_>>();
 let activity = |id: &str| {
 let runtime = app.session_state.session_runtime_states.get(id);
 (runtime.is_some_and(|r| r.request_lifecycle.is_requesting), runtime.is_some_and(|r| !r.request_lifecycle.queue.is_empty()), super::super::components::session_row::status_for_session(app, id))
 };
 domain::model(&domain::ModelInput {
recent_projects: &app.workspace_state.recent_projects,
recent_projects_edits: &app.workspace_state.recent_projects_edits,
project_path: &app.navigation_tabs.project_path,
project_sessions: &app.workspace_state.project_sessions,
work_expanded_spaces: &app.navigation_workspace.work_expanded_spaces,
work_sidebar_space_task_visible: &app.navigation_sidebar.work_sidebar_space_task_visible,
hovered_recent_project: &app.workspace_state.hovered_recent_project,
project_tools_menu_path: &app.workspace_state.project_tools_menu_path,
work_sidebar_filter_scope: app.navigation_sidebar.work_sidebar_filter_scope,
work_sidebar_filter_status: app.navigation_sidebar.work_sidebar_filter_status,
work_sidebar_filter_time: app.navigation_sidebar.work_sidebar_filter_time,
work_sidebar_visible_tasks: app.navigation_sidebar.work_sidebar_visible_tasks,
work_sidebar_visible_spaces: app.navigation_sidebar.work_sidebar_visible_spaces,
pinned: &app.session_ui_projection.pinned, drafts: &drafts, activity: &activity,
})
}
fn input(app: &App) -> domain::ViewInput<'_> { domain::ViewInput {
recent_projects: &app.workspace_state.recent_projects,
project_sessions_loading: &app.workspace_state.project_sessions_loading,
desktop_locale: app.general_settings.desktop_locale,
work_sidebar_collapsed: app.navigation_sidebar.work_sidebar_collapsed,
work_sidebar_more_open: app.navigation_sidebar.work_sidebar_more_open,
work_sidebar_filter_open: app.navigation_sidebar.work_sidebar_filter_open,
work_sidebar_project_nav_hovered: app.navigation_sidebar.work_sidebar_project_nav_hovered,
work_sidebar_filter_scope: app.navigation_sidebar.work_sidebar_filter_scope,
work_sidebar_filter_status: app.navigation_sidebar.work_sidebar_filter_status,
work_sidebar_filter_time: app.navigation_sidebar.work_sidebar_filter_time,
work_sidebar_tab: app.navigation_sidebar.work_sidebar_tab,
work_sidebar_width: app.layout_drag.work_sidebar_width,
new_session_creating: app.workspace_state.new_session_creating,
}}
pub(super) fn view(app: &App) -> Element<'_, Message> {
 let model = cached_model(app);
 let card = domain::current_project_card(&input(app), app.navigation_tabs.project_path.as_deref(), &app.workspace_state.recent_projects_edits, app.navigation_mode.work_current_project_menu.as_deref());
 let render = |task: &domain::WorkTaskItem| {
 let mut row = super::super::components::session_row::row_input(app, &task.id, task.details.title(), task.details.updated);
 row.pinned = app.session_ui_projection.pinned.contains(&(task.details.project_id().to_owned(), task.id.clone()));
 let menu = (app.workspace_state.session_menu_id.as_ref() == Some(&task.id)).then(|| {
 let path = app.known_session_directory(&task.id).or_else(|| app.navigation_tabs.project_path.clone()).unwrap_or_default();
 let compacting = app.session_state.sessions.iter().chain(app.workspace_state.project_sessions.values().flatten()).find(|s| s.id == task.id).is_some_and(|s| s.time.compacting.is_some());
 (path, app.session_is_requesting(&task.id) || compacting, app.workspace_state.session_menu_anchor.unwrap_or(Point::ORIGIN))
 });
 domain::TaskPresentation {row, menu}
 };
 domain::view(&input(app), &model, card, &render).map(map)
}
#[cfg(test)]
pub(super) fn refresh_all_message(app: &App) -> Message { map(domain::refresh_all_message(&input(app))) }
#[cfg(test)]
pub(super) fn project_navigation_message() -> Message { map(domain::project_navigation_message()) }
#[cfg(test)]
pub(super) fn project_create_message() -> Message { map(domain::project_create_message()) }
#[cfg(test)]
pub(super) fn refresh_all_control(app: &App) -> Element<'static, Message> { domain::refresh_all_control(&input(app)).map(map) }
#[cfg(test)]
pub(super) fn task_pool_control(app: &App) -> Element<'static, Message> { domain::task_pool_control(&input(app)).map(map) }
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
pub(super) fn expert_hub_button(i18n: crate::app::i18n::I18n) -> Element<'static, Message> { domain::expert_hub_button(i18n).map(map) }
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
pub(super) fn shiju_apps_button(i18n: crate::app::i18n::I18n) -> Element<'static, Message> { domain::shiju_apps_button(i18n).map(map) }
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
pub(super) fn shiju_skills_button(i18n: crate::app::i18n::I18n) -> Element<'static, Message> { domain::shiju_skills_button(i18n).map(map) }
#[cfg(test)]
pub(super) fn task_session_context_menu(
    app: &App,
    id: String,
    content: Element<'static, Message>,
) -> Element<'static, Message> {
    let menu = (app.workspace_state.session_menu_id.as_ref() == Some(&id)).then(|| {
        let path = app.known_session_directory(&id).or_else(|| app.navigation_tabs.project_path.clone()).unwrap_or_default();
        let compacting = app.session_state.sessions.iter().chain(app.workspace_state.project_sessions.values().flatten()).find(|session| session.id == id).is_some_and(|session| session.time.compacting.is_some());
        (build_session_menu(crate::app::i18n::I18n::new(app.general_settings.desktop_locale), id.clone(), path, app.session_is_requesting(&id) || compacting), app.workspace_state.session_menu_anchor.unwrap_or(Point::ORIGIN))
    });
    domain::session_context_menu(content, move |position| Message::Project(ProjectMessage::SessionRightClicked(id.clone(), position.x, position.y)), menu, Message::Project(ProjectMessage::SessionMenuClose))
}
