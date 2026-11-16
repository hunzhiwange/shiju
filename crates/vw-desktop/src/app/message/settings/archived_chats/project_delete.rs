//! Cross-domain cache reconciliation and gateway effects for archived project deletion.
use super::*;
pub(super) use desktop_settings_sessions::archived_chats::project_delete::{
    apply, delete_checked, handles,
};
pub(super) fn update(
    app: &mut App,
    message: ArchivedChatsMessage,
    current: &Identity,
) -> Task<Message> {
    use desktop_settings_sessions::archived_chats::project_delete::{
        add_reconcile_candidates, prepare,
    };
    let plan = prepare(&mut app.archived_chats, message, current);
    if let Some(ids) = plan.committed {
        sync_deleted_sessions(app, &ids);
    }
    if let Some((generation, row)) = plan.next {
        let candidates = delete_candidates(app, &row.id);
        add_reconcile_candidates(&mut app.archived_chats, candidates);
        return task(crate::app::config::gateway_client(), current.clone(), generation, row);
    }
    if let Some((success, summary)) = plan.summary {
        let kind = if success {
            crate::app::state::ToastKind::Success
        } else {
            crate::app::state::ToastKind::Error
        };
        let toast = app.show_toast(kind, summary);
        return Task::batch([toast, super::update(app, ArchivedChatsMessage::Refresh)]);
    }
    Task::none()
}
fn task(
    client: Result<vw_gateway_client::GatewayClient, String>,
    identity: Identity,
    generation: u64,
    row: Info,
) -> Task<Message> {
    desktop_settings_sessions::archived_chats::project_delete::task(
        client, identity, generation, row,
    )
    .map(super::event)
}
