//! Archive lifecycle decisions; shell effects reconcile other domains.
use crate::archived_chats::*;
use iced::Task;
use session_info::Info;
pub enum Effect {
    Deleted(Vec<String>),
    Restored { identity: Identity, generation: u64, info: Info },
    ProjectLoaded(String, Vec<Info>),
    Toast { success: bool, label: &'static str },
}
fn toast(
    effects: &mut Vec<Effect>,
    success: bool,
    label: &'static str,
) -> Task<ArchivedChatsMessage> {
    effects.push(Effect::Toast { success, label });
    Task::none()
}
pub fn update(
    state: &mut crate::state::ArchivedChatsState,
    message: ArchivedChatsMessage,
    current: Identity,
    client: Result<vw_gateway_client::GatewayClient, String>,
    delete_candidates: Vec<String>,
    effects: &mut Vec<Effect>,
) -> Task<ArchivedChatsMessage> {
    match message {
        ArchivedChatsMessage::DeleteConfirmed => {
            if (*state).delete_generation.is_some() || (*state).delete_target.is_none() {
                return Task::none();
            }
            apply(&mut (*state), message, &current);
            let Some(generation) = (*state).delete_generation else {
                return update(
                    state,
                    ArchivedChatsMessage::Refresh,
                    current,
                    client,
                    delete_candidates,
                    effects,
                );
            };
            let target = (*state).delete_target.as_ref().unwrap().row.clone();
            (*state).delete_reconcile_ids = delete_candidates.clone();
            return delete_task(client.clone(), current, generation, target);
        }
        ArchivedChatsMessage::Deleted { identity, generation, id, result } => {
            if !accept_delete(&(*state), &current, &identity, generation, &id) {
                return Task::none();
            }
            let result = result.and_then(|ids| {
                if ids.contains(&id) {
                    Ok(ids)
                } else {
                    Err("Incomplete archived deletion response".into())
                }
            });
            apply(
                &mut (*state),
                ArchivedChatsMessage::Deleted { identity, generation, id, result: result.clone() },
                &current,
            );
            return match result {
                Ok(ids) => {
                    (*state).delete_reconcile_ids.retain(|id| !ids.contains(id));
                    effects.push(Effect::Deleted(ids));
                    let toast = toast(effects, true, "已删除归档聊天。");
                    if (*state).delete_reconcile_ids.is_empty() {
                        toast
                    } else {
                        Task::batch([
                            toast,
                            update(
                                state,
                                ArchivedChatsMessage::Refresh,
                                current,
                                client,
                                delete_candidates,
                                effects,
                            ),
                        ])
                    }
                }
                Err(_) => {
                    let toast = toast(effects, false, "删除归档聊天失败，请重试。");
                    Task::batch([
                        toast,
                        update(
                            state,
                            ArchivedChatsMessage::Refresh,
                            current,
                            client,
                            delete_candidates,
                            effects,
                        ),
                    ])
                }
            };
        }
        ArchivedChatsMessage::Loaded { ref identity, generation, ref result }
            if identity == &current
                && (*state).identity.as_ref() == Some(identity)
                && generation == (*state).request_generation
                && (*state).loading =>
        {
            if let Ok(rows) = result {
                let removed: Vec<_> = state
                    .delete_reconcile_ids
                    .iter()
                    .filter(|id| !rows.iter().any(|row| &row.id == *id))
                    .cloned()
                    .collect();
                (*state).delete_reconcile_ids.clear();
                effects.push(Effect::Deleted(removed));
            }
        }
        ArchivedChatsMessage::Restore(ref id) => {
            if (*state).operating_ids.contains(id) {
                return Task::none();
            }
            let id = id.clone();
            apply(&mut (*state), message, &current);
            let Some(&generation) = (*state).restore_requests.get(&id) else {
                return Task::none();
            };
            let row = (*state).sessions.iter().find(|row| row.id == id).unwrap().clone();
            return restore_task(client.clone(), current, generation, row);
        }
        ArchivedChatsMessage::Restored { identity, generation, id, result } => {
            if !accept_restore(&(*state), &current, &identity, generation, &id) {
                return Task::none();
            }
            let result = result.and_then(|info| {
                if info.id != id || info.time.archived.is_some() {
                    Err("Gateway did not return the restored session".into())
                } else {
                    Ok(info)
                }
            });
            apply(
                &mut (*state),
                ArchivedChatsMessage::Restored {
                    identity: identity.clone(),
                    generation,
                    id,
                    result: result.clone(),
                },
                &current,
            );
            return match result {
                Ok(info) => {
                    effects.push(Effect::Restored {
                        identity,
                        generation: state.request_generation,
                        info,
                    });
                    Task::none()
                }
                Err(_) => toast(effects, false, "取消归档失败，请重试。"),
            };
        }
        ArchivedChatsMessage::ProjectRefreshed { identity, generation, result } => {
            if identity != current
                || (*state).identity.as_ref() != Some(&identity)
                || generation != (*state).request_generation
            {
                return Task::none();
            }
            return match result {
                Ok((path, sessions)) => {
                    effects.push(Effect::ProjectLoaded(path, sessions));
                    Task::none()
                }
                Err(_) => Task::none(),
            };
        }
        _ => {}
    }
    if matches!(message, ArchivedChatsMessage::Refresh) {
        if !(*state).operating_ids.is_empty() {
            return Task::none();
        }
        apply(&mut (*state), message, &current);
        let generation = (*state).request_generation;
        let client = client.clone();
        return load_task(client, current, generation);
    }
    apply(&mut (*state), message, &current);
    Task::none()
}
