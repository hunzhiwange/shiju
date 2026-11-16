use crate::app::{App, Message, state::WorkResourceMode, message::ViewMessage};
use crate::app::components::{file_tree, preview_panel};
use desktop_workspace::views::project::layout::work_resources as resources;
pub(super) use resources::ResourceBodyKind;
pub(super) fn body_kind(app: &App) -> ResourceBodyKind { mode(app.navigation_workspace.work_resource_mode) }
fn mode(mode: WorkResourceMode) -> ResourceBodyKind { match mode { WorkResourceMode::Overview => ResourceBodyKind::Overview, WorkResourceMode::WorkspaceFiles => ResourceBodyKind::WorkspaceFiles, WorkResourceMode::Browser => ResourceBodyKind::Browser } }
pub(super) fn overview_model(app: &App) -> resources::OverviewModel {
    let title = resources::active_session_title(app.active_session_id.as_deref(), app.sessions.iter().chain(app.project_sessions.values().flat_map(|sessions| sessions.iter())).map(|s| (s.id.as_str(), s.title.as_str())),app.desktop_locale);
    resources::overview_model(app.navigation_tabs.project_path.as_deref(),title,app.navigation_tabs.project_path.as_ref().and_then(|p| app.project_sessions.get(p)).map(Vec::len),app.sessions.iter().map(|s| s.directory.as_str()),app.git.git_changed_files.len(),app.git.selected_branch.as_deref(),crate::app::components::git_panel::git_repo_path_for_app(app).is_some(),app.desktop_locale)
}
fn event(event: resources::ResourceEvent) -> Message { Message::View(match event {
    resources::ResourceEvent::ModeSelected(value) => ViewMessage::WorkResourceModeSelected(match value {ResourceBodyKind::Overview=>WorkResourceMode::Overview,ResourceBodyKind::WorkspaceFiles=>WorkResourceMode::WorkspaceFiles,ResourceBodyKind::Browser=>WorkResourceMode::Browser}),
    resources::ResourceEvent::SplitRatioChanged(ratio)=>ViewMessage::WorkSplitRatioChanged(ratio),
    resources::ResourceEvent::HalfFullscreenToggled=>ViewMessage::WorkResourceHalfFullscreenToggled,
    resources::ResourceEvent::FullscreenToggled=>ViewMessage::WorkResourceFullscreenToggled,
}) }
pub(super) fn view(app: &App, _radius: f32) -> iced::Element<'_,Message> {
    let body = match body_kind(app) {
        ResourceBodyKind::Overview=>resources::overview(overview_model(app), app.desktop_locale),
        ResourceBodyKind::WorkspaceFiles=>resources::workspace_files(file_tree::view_file_manager(app),preview_panel::view(app)),
        ResourceBodyKind::Browser=>crate::app::message::view::panel::work_browser::view(app),
    };
    resources::view(body_kind(app),app.navigation_workspace.work_resource_fullscreen,app.navigation_workspace.work_resource_half_fullscreen,app.navigation_workspace.work_split_ratio,app.desktop_locale,body,event)
}
