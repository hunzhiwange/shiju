//! Domain fixtures use actual reducers and explicit unavailable network boundaries.
use crate::{messages::*, state::*};
#[derive(Default)]
struct Fixture {
    worktree_settings: WorktreeSettingsState,
    project_path: Option<String>,
    project_worktree_enabled: std::collections::HashMap<String, bool>,
    project_id: Option<String>,
    desktop_locale: config_ui::DesktopLocale,
    effects: Vec<crate::lifecycle::Effect>,
}
fn identity(app: &Fixture) -> WorktreeSettingsIdentity {
    WorktreeSettingsIdentity::from_endpoint(
        &gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1),
        app.project_path.clone(),
    )
}
fn apply(
    app: &mut Fixture,
    event: EnvironmentMessage,
) -> Option<(vw_api_project::ProjectDto, vw_api_project::UpdateProjectRequest)> {
    let current = identity(app);
    crate::environment::apply(&mut app.worktree_settings.environment, &current, event)
}
fn update(app: &mut Fixture, event: EnvironmentMessage) -> iced::Task<EnvironmentMessage> {
    let current = identity(app);
    crate::environment::update(
        &mut app.worktree_settings.environment,
        event,
        current,
        Err("fixture unavailable".into()),
        vec![],
        true,
        &mut Vec::new(),
    )
}
fn update_worktrees(
    app: &mut Fixture,
    event: WorktreeSettingsMessage,
) -> iced::Task<WorktreeSettingsMessage> {
    let current = identity(app);
    match event {
        WorktreeSettingsMessage::Managed(event) => crate::managed::update(
            &mut app.worktree_settings,
            event,
            current,
            0,
            Err("fixture unavailable".into()),
            &mut app.effects,
        ),
        _ => iced::Task::none(),
    }
}
mod environment;
mod recovery;

mod projects_view;

mod managed;

mod project_events;
