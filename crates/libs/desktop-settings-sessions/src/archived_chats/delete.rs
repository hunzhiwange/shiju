use super::*;
pub fn valid_delete_target(state: &ArchivedChatsState, current: &Identity) -> bool {
    !state.loading
        && state.operating_ids.is_empty()
        && state.identity.as_ref() == Some(current)
        && state.delete_target.as_ref().is_some_and(|target| {
            target.identity == *current
                && matches!(&state.delete_confirmation,
                    Some(crate::state::ArchivedChatsDeleteConfirmation::Chat(id)) if id == &target.row.id)
                && state.sessions.iter().any(|row| row.id == target.row.id
                    && row.project_id == target.row.project_id && row.directory == target.row.directory
                    && row.time.archived.is_some() && row.is_user_navigable())
        })
}

pub fn accept_delete(
    state: &ArchivedChatsState,
    current: &Identity,
    identity: &Identity,
    generation: u64,
    id: &str,
) -> bool {
    identity == current
        && state.identity.as_ref() == Some(identity)
        && state.delete_generation == Some(generation)
        && state.operating_ids.contains(id)
        && state
            .delete_target
            .as_ref()
            .is_some_and(|target| target.row.id == id && target.identity == *identity)
}

pub fn delete_task(
    client: Result<vw_gateway_client::GatewayClient, String>,
    identity: Identity,
    generation: u64,
    row: Info,
) -> Task<ArchivedChatsMessage> {
    let id = row.id.clone();
    Task::perform(
        async move {
            gateway_session::GatewaySessionClient::new(client?.transport().clone())
                .session_delete_archived(&row.id, Some(&row.directory))
                .await
                .map(|response| response.deleted_ids)
        },
        move |result| {
            ArchivedChatsMessage::Deleted {
                identity: identity.clone(),
                generation,
                id: id.clone(),
                result,
            }
        },
    )
}


pub fn delete_candidates(rows: &[&Info], root: &str) -> Vec<String> {
    let mut ids = vec![root.to_owned()];
    loop {
        let before = ids.len();
        for row in rows {
            if row.parent_id.as_ref().is_some_and(|parent| ids.contains(parent))
                && !ids.contains(&row.id)
            {
                ids.push(row.id.clone());
            }
        }
        if ids.len() == before {
            return ids;
        }
    }
}
