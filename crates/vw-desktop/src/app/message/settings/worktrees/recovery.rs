//! Desktop mapping for workspace lifecycle effects.
use crate::app::{App, Message};
use iced::Task;
pub use desktop_settings_workspace::messages::RecoveryMessage;
pub(crate) fn invalidate(app: &mut App) {
    desktop_settings_workspace::recovery::invalidate(&mut app.worktree_settings);
}
pub(crate) fn update(app: &mut App, event: RecoveryMessage) -> Task<Message> {
    let current = crate::app::state::worktree_identity(app);
    let client = crate::app::gateway_client();
    let mut effects = Vec::new();
    let task = desktop_settings_workspace::recovery::update(
        &mut app.worktree_settings, event, current,
        app.session_state.session_view_cache.navigation_generation, client, &mut effects,
    );
    super::managed::finish(app, task, effects)
}
