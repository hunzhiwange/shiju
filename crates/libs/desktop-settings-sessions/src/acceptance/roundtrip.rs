use crate::state::ArchivedChatsState;
use crate::archived_chats::ArchivedChatsMessage as Archive;
pub async fn roundtrip(
    state: &mut ArchivedChatsState,
    client: vw_gateway_client::GatewayClient,
    mut send: impl AsyncFnMut(&mut ArchivedChatsState, Archive),
) -> (ArchivedChatsState, ArchivedChatsState) {
    send(state, Archive::Refresh).await;
    assert!(state.error.is_some(), "injected list failure");
    let error = state.clone();
    send(state, Archive::Refresh).await;
    assert_eq!(state.sessions.len(), 1178);
    let row = state.sessions[0].clone();
    // Persist archive through the real HTTP client before listing and restoring.
    let _: session_info::Info = gateway_session::GatewaySessionClient::new(client.transport().clone())
        .session_update(
            &row.id,
            Some(&row.directory),
            &vw_api_session::GatewaySessionPatchBody {
                title: None,
                time: Some(vw_api_session::GatewaySessionPatchTime { archived: Some(10) }),
            },
        )
        .await
        .unwrap();
    send(state, Archive::Refresh).await;
    send(state, Archive::Restore(row.id.clone())).await;
    assert!(
        state
            .sessions
            .iter()
            .any(|row| row.id == "chat-0000" && row.time.archived.is_some())
    );
    assert!(state.operating_ids.is_empty());
    send(state, Archive::Restore(row.id)).await;
    send(state, Archive::Refresh).await;
    assert!(
        state
            .sessions
            .iter()
            .find(|row| row.id == "chat-0000")
            .unwrap()
            .time
            .archived
            .is_none()
    );
    assert_eq!(
        state.groups().iter().map(|group| group.rows.len()).sum::<usize>(),
        1177
    );
    send(state, Archive::DeleteRequested("chat-0001".into())).await;
    send(state, Archive::DeleteConfirmed).await;
    send(state, Archive::Refresh).await;
    assert!(
        !state
            .sessions
            .iter()
            .any(|row| ["chat-0001", "chat-0002"].contains(&row.id.as_str()))
    );
    send(state, Archive::ProjectDeleteRequested("batch".into())).await;
    send(state, Archive::DeleteConfirmed).await;
    let batch = state.project_delete.as_ref().unwrap();
    assert_eq!((batch.succeeded, batch.failed.len()), (1, 1));
    let partial = state.clone();
    send(state, Archive::ProjectDeleteRetry).await;
    assert_eq!(state.project_delete.as_ref().unwrap().targets.len(), 1);
    send(state, Archive::DeleteConfirmed).await;
    send(state, Archive::Refresh).await;
    assert_eq!(state.sessions.len(), 1174);
    assert!(!state.groups().iter().any(|group| group.project.id == "batch"));
    eprintln!(
        "PASS: 3 HTTP roundtrips: archive/restore/reload, descendant delete/reload, partial batch/retry/reload"
    );
    (error, partial)
}
