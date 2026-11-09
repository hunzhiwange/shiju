use super::*;

#[test]
fn tui_first_response_title_applies_only_to_the_matching_request_and_unnamed_session() {
    let directory = PathBuf::from("/workspace");
    let runtime = GatewayUiRuntime::new(
        vw_gateway_client::GatewayClient::new(vw_gateway_client::GatewayEndpoint::new(
            "127.0.0.1",
            9,
        ))
        .unwrap(),
        super::super::runtime::gateway::GatewaySessionSeed::new(directory.clone()),
    );
    let state = initial_tui_state(&runtime);
    let mut app = V3GatewayApp::new(runtime, state);
    app.bind_draft("first-session".into());
    let mut runner = ActiveRunner::new();
    runner.title_request_id = Some("first-request".into());

    for (other_directory, session_id, request_id) in [
        (PathBuf::from("/old-directory"), "first-session", "first-request"),
        (directory.clone(), "old-session", "first-request"),
        (directory.clone(), "first-session", "old-request"),
    ] {
        runner
            .host_sender
            .send(HostMessage::SessionTitleLoaded {
                directory: other_directory,
                session_id: session_id.into(),
                request_id: request_id.into(),
                title: "迟到标题".into(),
            })
            .unwrap();
    }
    runner.drain_host_messages(&mut app);
    assert_eq!(app.state.session.title, "");

    app.set_server_title("手工标题".into());
    runner
        .host_sender
        .send(HostMessage::SessionTitleLoaded {
            directory,
            session_id: "first-session".into(),
            request_id: "first-request".into(),
            title: "迟到标题".into(),
        })
        .unwrap();
    runner.drain_host_messages(&mut app);
    assert_eq!(app.state.session.title, "手工标题");
    assert_eq!(app.runtime().title(), Some("手工标题"));
}
