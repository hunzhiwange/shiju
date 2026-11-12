use super::*;

#[test]
fn tui_initial_title_uses_input_until_matching_llm_title_arrives() {
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
    app.handle_action(V3Action::Submit {
        text: " \r\n\t\n  首轮中文问题  \r\n补充说明".into(),
        attachments: Vec::new(),
    })
    .unwrap();
    let display = GatewayV3Bridge::session_display_data(app.runtime(), app.state());
    assert_eq!(display.session_title.as_deref(), Some("首轮中文问题"));
    assert_eq!(app.state.session.title, "");
    assert!(app.runtime().title().is_none());
    let rendered = app
        .transcript_lines(100, true)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        rendered.lines().any(|line| line.contains("session:") && line.contains("首轮中文问题"))
    );

    let mut runner = ActiveRunner::new();
    runner.title_request_id = Some("title-request".into());
    runner
        .host_sender
        .send(HostMessage::SessionTitleLoaded {
            directory,
            session_id: app.runtime().session_id().unwrap().into(),
            request_id: "title-request".into(),
            title: "LLM 重写标题".into(),
        })
        .unwrap();
    runner.drain_host_messages(&mut app);
    assert_eq!(
        GatewayV3Bridge::session_display_data(app.runtime(), app.state()).session_title.as_deref(),
        Some("LLM 重写标题")
    );
}

#[test]
fn tui_published_preview_and_resume_list_use_first_non_empty_line() {
    let directory = PathBuf::from("/workspace");
    let runtime = GatewayUiRuntime::new(
        vw_gateway_client::GatewayClient::new(vw_gateway_client::GatewayEndpoint::new(
            "127.0.0.1",
            9,
        ))
        .unwrap(),
        super::super::runtime::gateway::GatewaySessionSeed::new(directory),
    );
    let state = initial_tui_state(&runtime);
    let mut app = V3GatewayApp::new(runtime, state);
    app.bind_draft("published".into());
    let preview = ChatSessionMeta {
        id: "published".into(),
        title: String::new(),
        updated_ms: 1,
        message_count: 2,
        call_count: 0,
        last_content: Some(" \r\n  首轮预览 \r\n补充说明".into()),
    };
    assert_eq!(session_resume_items(std::slice::from_ref(&preview))[0].label, "首轮预览");
    assert!(app.apply_chat_publication(preview));
    assert_eq!(
        GatewayV3Bridge::session_display_data(app.runtime(), app.state()).session_title.as_deref(),
        Some("首轮预览")
    );
    assert_eq!(app.state.session.title, "");
}

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
