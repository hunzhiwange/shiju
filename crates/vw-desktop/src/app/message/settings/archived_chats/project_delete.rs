use super::*;
pub(super) use desktop_settings_sessions::archived_chats::project_delete::{handles, apply, delete_checked};
pub(super) fn update(
    app: &mut App,
    message: ArchivedChatsMessage,
    current: &Identity,
) -> Task<Message> {
    let mut committed = None;
    if let ArchivedChatsMessage::ProjectDeleted { identity, generation, id, result } = &message {
        let valid = identity == current
            && app.archived_chats.identity.as_ref() == Some(current)
            && app.archived_chats.delete_generation == Some(*generation)
            && app
                .archived_chats
                .project_delete
                .as_ref()
                .and_then(|b| b.targets.get(b.next))
                .is_some_and(|r| &r.id == id);
        if !valid {
            return Task::none();
        }
        if let Ok(Some(ids)) = result
            && ids.contains(id)
        {
            committed = Some(ids.clone());
        }
    }
    let was_running = app.archived_chats.delete_generation.is_some();
    apply(&mut app.archived_chats, &message, current);
    if let Some(ids) = committed {
        sync_deleted_sessions(app, &ids);
    }
    if matches!(message, ArchivedChatsMessage::ProjectDeleteRetry) {
        return Task::none();
    }
    if was_running && !matches!(message, ArchivedChatsMessage::ProjectDeleted { .. }) {
        return Task::none();
    }
    if let Some(generation) = app.archived_chats.delete_generation {
        let batch = app.archived_chats.project_delete.as_ref().unwrap();
        let row = batch.targets[batch.next].clone();
        let candidates = delete_candidates(app, &row.id);
        for id in candidates {
            if !app.archived_chats.delete_reconcile_ids.contains(&id) {
                app.archived_chats.delete_reconcile_ids.push(id);
            }
        }
        return task(crate::app::config::gateway_client(), current.clone(), generation, row);
    }
    if was_running {
        let batch = app.archived_chats.project_delete.as_ref().unwrap();
        let summary = format!(
            "归档聊天删除结果：成功 {}，失败 {}，状态变化跳过 {}。",
            batch.succeeded,
            batch.failed.len(),
            batch.skipped
        );
        let kind = if batch.failed.is_empty() && batch.skipped == 0 {
            crate::app::state::ToastKind::Success
        } else {
            crate::app::state::ToastKind::Error
        };
        let toast = app.show_toast(kind, summary);
        return Task::batch([toast, super::update(app, ArchivedChatsMessage::Refresh)]);
    }
    Task::none()
}

fn task(client: Result<vw_gateway_client::GatewayClient, String>, identity: Identity, generation: u64, row: Info) -> Task<Message> { desktop_settings_sessions::archived_chats::project_delete::task(client, identity, generation, row).map(super::event) }
