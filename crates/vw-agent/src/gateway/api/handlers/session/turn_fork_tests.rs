use super::*;
use axum::{
    Extension,
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use crate::session::{agent_binding, message};
use message::{Info as MessageInfo, Part, WithParts};
use vw_api_types::id::TenantScope;

async fn request(method: Method, path: &str, directory: &str, body: Value) -> (StatusCode, Value) {
    let response = super::super::super::router()
        .with_state(state())
        .layer(Extension(ui_store::current_tenant_scope()))
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .header("x-vibewindow-directory", directory)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn fork(source: &agent_session::session::Info, body: Value) -> (StatusCode, Value) {
    request(Method::POST, &format!("/session/{}/fork", source.id), &source.directory, body).await
}

async fn history(info: &agent_session::session::Info) -> Vec<WithParts> {
    let (status, body) = request(
        Method::GET,
        &format!("/session/{}/message", info.id),
        &info.directory,
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    serde_json::from_value(body).unwrap()
}

async fn persist_turn(session_id: &str, query: &str, answer: &str) -> (String, String) {
    super::super::super::stream::persist_child_agent_turn(
        session_id, query, answer, "gateway", None,
    )
    .await
    .unwrap()
}

fn git(directory: &FsPath, args: &[&str]) -> String {
    let result =
        std::process::Command::new("git").current_dir(directory).args(args).output().unwrap();
    assert!(result.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&result.stderr));
    String::from_utf8(result.stdout).unwrap()
}

fn repository() -> TempDir {
    let directory = TempDir::new().unwrap();
    git(directory.path(), &["init", "--quiet"]);
    std::fs::write(directory.path().join("tracked.txt"), Uuid::new_v4().to_string()).unwrap();
    git(directory.path(), &["add", "tracked.txt"]);
    git(
        directory.path(),
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "initial",
        ],
    );
    directory
}

async fn worktree_cleanup(source: &agent_session::session::Info, target: String) {
    with_tenant_instance(ui_store::current_tenant_scope(), source.directory.clone(), move || {
        Box::pin(async move {
            crate::worktree::remove_fork_checkout(target)
                .await
                .map_err(|error| ApiError::internal(error.to_string()))
        })
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn session_fork_before_turn_keeps_only_complete_earlier_turns() {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    let repo = repository();
    let source = create_session_in(repo.path(), "Source").await;
    let (first_assistant, first) = persist_turn(&source.id, "same prompt", "first answer").await;
    let (_, second) = persist_turn(&source.id, "same prompt", "second answer").await;
    persist_turn(&source.id, "third prompt", "third answer").await;
    for (body, count) in [(Value::Null, 6), (json!({"messageID":first_assistant}), 2)] {
        let (status, body) = fork(&source, body).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let forked = serde_json::from_value(body).unwrap();
        assert_eq!(history(&forked).await.len(), count);
    }
    let mut earlier_assistant = message::get(&source.id, &first_assistant).await.unwrap().info;
    let early_id = format!("a-{first_assistant}");
    assert!(early_id < first);
    earlier_assistant.set_id(&early_id);
    message::update_message(&earlier_assistant).await.unwrap();
    for (index, state) in [
        json!({"status":"completed","input":{},"output":"retained tool result","title":"read","metadata":{},"time":{"start":1,"end":2}}),
        json!({"status":"running","input":{},"time":{"start":1}}),
    ].into_iter().enumerate() {
        let tool: Part = serde_json::from_value(json!({
            "type":"tool", "id":format!("part-{index}"), "sessionID":source.id,
            "messageID":early_id, "callID":format!("call-{index}"), "tool":"shell", "state":state
        })).unwrap();
        message::update_part(&tool).await.unwrap();
    }
    let binding = agent_binding::AgentBinding::from_definition(
        "agent-fixture".into(),
        &vw_config_types::agent::AgentDefinitionConfig::default(),
    );
    agent_binding::save_scoped(&source.project_id, &source.id, &binding).await.unwrap();
    assert!(
        !agent_binding::resolve_full_access_scoped(
            &source.project_id,
            &source.id,
            Some(false),
            true
        )
        .await
        .unwrap()
    );

    let original = serde_json::to_value(history(&source).await).unwrap();
    let name = format!("fork-test-{}", Uuid::new_v4());
    let mut new_ids = std::collections::HashSet::new();
    for target in [Value::Null, json!({"type":"worktree","name":name})] {
        for (boundary, count) in [(&second, 3), (&first, 0), (&second, 3)] {
            let (status, body) =
                fork(&source, json!({"beforeTurnId":boundary, "target":target})).await;
            assert_eq!(status, StatusCode::OK, "{body}");
            let forked: agent_session::session::Info = serde_json::from_value(body).unwrap();
            assert_ne!(forked.id, source.id);
            assert!(new_ids.insert(forked.id.clone()));
            assert_eq!(forked.project_id, source.project_id);
            assert!(forked.parent_id.is_none());
            assert_eq!(
                agent_binding::load_scoped(&source.project_id, &forked.id)
                    .await
                    .unwrap()
                    .unwrap()
                    .agent,
                "agent-fixture"
            );
            assert!(
                !agent_binding::resolve_full_access_scoped(
                    &source.project_id,
                    &forked.id,
                    None,
                    true
                )
                .await
                .unwrap()
            );
            if target.is_null() {
                assert_eq!(forked.directory, source.directory);
            } else {
                assert_ne!(forked.directory, source.directory);
                assert_eq!(
                    std::fs::read(FsPath::new(&forked.directory).join("tracked.txt")).unwrap(),
                    std::fs::read(repo.path().join("tracked.txt")).unwrap()
                );
                let (status, body) = request(
                    Method::GET,
                    &format!("/session?projectID={}", source.project_id),
                    &source.directory,
                    Value::Null,
                )
                .await;
                assert_eq!(status, StatusCode::OK, "{body}");
                let sessions: Vec<agent_session::session::Info> =
                    serde_json::from_value(body).unwrap();
                assert!(sessions.iter().all(|info| info.project_id == source.project_id));
                assert!(sessions.iter().any(|info| info.id == source.id));
                assert!(sessions.iter().any(|info| info.id == forked.id));
            }
            let messages = history(&forked).await;
            assert_eq!(messages.len(), count);
            if let [latest, earlier, user] = messages.as_slice() {
                assert_ne!(user.info.id(), first);
                assert!(
                    matches!(&user.parts[..], [Part::Text(text)] if text.text == "same prompt")
                );
                for assistant in [latest, earlier] {
                    let MessageInfo::Assistant(info) = &assistant.info else {
                        panic!("assistant should follow user");
                    };
                    assert_eq!(info.parent_id, user.info.id());
                    assert_eq!(info.session_id, forked.id);
                    for part in &assistant.parts {
                        assert_eq!(part.session_id(), forked.id);
                        assert_eq!(part.message_id(), info.id);
                    }
                }
                assert!(
                    matches!(&latest.parts[..], [Part::Text(text)] if text.text == "first answer")
                );
                let [Part::Tool(completed), Part::Tool(interrupted)] = &earlier.parts[..] else {
                    panic!("first turn tools should be retained");
                };
                assert!(
                    matches!(&completed.state, message::ToolState::Completed(state) if state.output == "retained tool result")
                );
                assert!(matches!(&interrupted.state, message::ToolState::Error(_)));
            }
            if !target.is_null() {
                worktree_cleanup(&source, forked.directory).await;
            }
        }
    }
    assert_eq!(serde_json::to_value(history(&source).await).unwrap(), original);
}

#[tokio::test]
async fn session_fork_before_turn_rejects_invalid_boundaries_before_creating_targets() {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    let repo = repository();
    let source = create_session_in(repo.path(), "Source").await;
    let foreign = create_session_in(repo.path(), "Foreign").await;
    let (assistant, user) = persist_turn(&source.id, "first", "answer").await;
    let (_, foreign_user) = persist_turn(&foreign.id, "foreign", "answer").await;
    let (_, deleted) = persist_turn(&source.id, "deleted", "answer").await;
    crate::session::revert::cleanup_from_message(&source.id, &deleted).await.unwrap();
    let missing_response = format!("unpersisted-{user}");
    let mut unfinished = message::get(&source.id, &user).await.unwrap();
    unfinished.info.set_id(&missing_response);
    message::update_message(&unfinished.info).await.unwrap();
    for mut part in unfinished.parts {
        part.set_message_id(&missing_response);
        message::update_part(&part).await.unwrap();
    }

    let before = git(repo.path(), &["worktree", "list", "--porcelain"]);
    let target = json!({"type":"worktree","name":format!("invalid-fork-{}",Uuid::new_v4())});
    for boundary in ["", "missing", &assistant, &foreign_user, &deleted, &missing_response] {
        let (status, _) = fork(&source, json!({"beforeTurnId":boundary,"target":target})).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "boundary {boundary}");
    }
    let (status, _) =
        fork(&source, json!({"beforeTurnId":user,"messageID":user,"target":target})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = fork(&source, json!({"target":target})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let other_tenant = TenantScope::new("fork-test", "other").unwrap();
    let (status, _) = ui_store::with_tenant_scope(
        other_tenant,
        fork(&source, json!({"beforeTurnId":user,"target":target})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(git(repo.path(), &["worktree", "list", "--porcelain"]), before);
    let listed = agent_session::session::list_all().await.unwrap();
    assert_eq!(listed.iter().filter(|info| info.project_id == source.project_id).count(), 2);
}

#[tokio::test]
async fn session_fork_before_turn_rolls_back_staged_history_and_worktree_on_write_failure() {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    let repo = repository();
    let source = create_session_in(repo.path(), "Write Failure Fixture").await;
    persist_turn(&source.id, "first", "answer").await;
    let (_, boundary) = persist_turn(&source.id, "second", "answer").await;
    let db = crate::global_database::GlobalDatabase::new(
        ui_store::session_file_path(&source.id).unwrap(),
    )
    .unwrap();
    let connection = db.connect_sqlite().unwrap();
    connection
        .execute_batch(
            "
        CREATE TRIGGER test_fail_fork_insert BEFORE INSERT ON session_agent_sessions
        WHEN json_extract(NEW.info_json, '$.title') = 'Write Failure Fixture (fork #1)'
        BEGIN SELECT RAISE(ABORT, 'injected fork write failure'); END;
    ",
        )
        .unwrap();

    let name = format!("write-failure-{}", Uuid::new_v4());
    let before = git(repo.path(), &["worktree", "list", "--porcelain"]);
    let branches = git(repo.path(), &["branch", "--list"]);
    let messages = crate::storage::list(&["message"]).await.unwrap();
    let parts = crate::storage::list(&["part"]).await.unwrap();
    let body = json!({"beforeTurnId":boundary,"target":{"type":"worktree","name":name}});
    let (status, failure) = fork(&source, body.clone()).await;
    connection.execute_batch("DROP TRIGGER test_fail_fork_insert").unwrap();

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{failure}");
    assert_eq!(git(repo.path(), &["worktree", "list", "--porcelain"]), before);
    assert_eq!(git(repo.path(), &["branch", "--list"]), branches);
    assert_eq!(crate::storage::list(&["message"]).await.unwrap(), messages);
    assert_eq!(crate::storage::list(&["part"]).await.unwrap(), parts);
    let listed = agent_session::session::list_all().await.unwrap();
    assert_eq!(listed.iter().filter(|info| info.project_id == source.project_id).count(), 1);

    let (status, retried) = fork(&source, body).await;
    assert_eq!(status, StatusCode::OK, "{retried}");
    let forked: agent_session::session::Info = serde_json::from_value(retried).unwrap();
    assert_eq!(history(&forked).await.len(), 2);
    worktree_cleanup(&source, forked.directory).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn fork_history_ui_reads_the_persisted_prefix() {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    let repo = repository();
    let source = create_session_in(repo.path(), "History source").await;
    let (_, first) = persist_turn(&source.id, "retained user", "retained assistant").await;
    let (_, second) = persist_turn(&source.id, "omitted user", "omitted assistant").await;
    persist_turn(&source.id, "future user", "future assistant").await;

    let warm_state = state();
    warm_state.config.lock().workspace_dir = repo.path().to_owned();
    let source_engine =
        crate::gateway::chat::session_query_engine(&warm_state, &source.id).await.unwrap();
    source_engine
        .lock()
        .await
        .replace_history(vec![crate::providers::ChatMessage::user("source cached future")]);

    for (boundary, expected) in [(&second, 2), (&first, 0)] {
        let (status, body) = fork(&source, json!({"beforeTurnId": boundary})).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let forked: agent_session::session::Info = serde_json::from_value(body).unwrap();
        let native = history(&forked).await;
        let (status, ui) = request(
            Method::GET,
            &format!("/session/{}/ui/without-raw", forked.id),
            &forked.directory,
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{ui}");
        let messages = ui["messages"].as_array().expect("persisted UI history");
        assert_eq!(messages.len(), expected);
        let ids = native.iter().rev().map(|msg| msg.info.id()).collect::<Vec<_>>();
        assert_eq!(ui["message_ids"], json!(ids));
        eprintln!("fork UI readback: target={} message_ids={ids:?}", forked.directory);
        if expected > 0 {
            assert_eq!(messages[0]["content"], "retained user");
            assert_eq!(messages[1]["content"], "retained assistant");
        }
        for gateway in [&warm_state, &state()] {
            let engine =
                crate::gateway::chat::session_query_engine(gateway, &forked.id).await.unwrap();
            let engine = engine.lock().await;
            let history = engine.history_snapshot();
            assert!(!history.iter().any(|msg| msg.content.contains("source cached future")));
            assert_eq!(history.iter().any(|msg| msg.content == "retained user"), expected > 0);
            assert_eq!(history.iter().any(|msg| msg.content == "retained assistant"), expected > 0);
        }
    }
    assert_eq!(source_engine.lock().await.history_snapshot()[0].content, "source cached future");
}

#[tokio::test(flavor = "multi_thread")]
async fn fork_history_first_sse_request_replays_only_durable_turns() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();

    struct RestoreHttpMode(Option<std::ffi::OsString>);
    impl Drop for RestoreHttpMode {
        fn drop(&mut self) {
            unsafe {
                if let Some(previous) = self.0.take() {
                    std::env::set_var("VW_RECOVERY_WINDOW_HTTP", previous);
                } else {
                    std::env::remove_var("VW_RECOVERY_WINDOW_HTTP");
                }
            }
        }
    }
    let _http = RestoreHttpMode(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe { std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1") };

    let requests = Arc::new(parking_lot::Mutex::new(Vec::<Value>::new()));
    let captured = Arc::clone(&requests);
    let model = axum::Router::new().route(
        "/v1/chat/completions",
        axum::routing::post(move |Json(body): Json<Value>| {
            let captured = Arc::clone(&captured);
            async move {
                captured.lock().push(body);
                let chunk = json!({"id":"fork-history","object":"chat.completion.chunk","created":0,
                    "model":"original","choices":[{"index":0,"delta":{"content":"fork response"},
                    "finish_reason":"stop"}]});
                (
                    [("content-type", "text/event-stream")],
                    format!("data: {chunk}\n\ndata: [DONE]\n\n"),
                )
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, model).await.unwrap() });
    let _fixture = agent_session::llm::recovery_test_support::RecoveryConfig::new(
        &format!("http://{address}/v1"),
        Some("fixture-key"),
    )
    .await;
    let repo = repository();
    let source = create_session_in(repo.path(), "Fork request source").await;
    let (assistant, first) =
        persist_turn(&source.id, "fork retained user", "fork retained answer").await;
    let (_, second) =
        persist_turn(&source.id, "fork omitted second", "fork omitted second response").await;
    persist_turn(&source.id, "fork omitted third", "fork omitted third response").await;
    let tool: Part = serde_json::from_value(json!({
        "type":"tool", "id":"part-fork-fixture", "sessionID":source.id, "messageID":assistant,
        "callID":"fork-retained-call", "tool":"read",
        "state":{"status":"completed","input":{"path":"tracked.txt"},"output":"fork retained result",
            "title":"read","metadata":{},"time":{"start":1,"end":2}}
    })).unwrap();
    message::update_part(&tool).await.unwrap();

    for (target, boundary) in [
        (Value::Null, &second),
        (Value::Null, &first),
        (json!({"type":"worktree","name":format!("fork-history-{}", Uuid::new_v4())}), &second),
        (json!({"type":"worktree","name":format!("fork-history-{}", Uuid::new_v4())}), &first),
    ] {
        let (status, body) = fork(&source, json!({"beforeTurnId":boundary, "target":target})).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let branch: agent_session::session::Info = serde_json::from_value(body).unwrap();
        let request = vw_api_types::chat::GatewayChatStreamRequest {
            session_id: Some(branch.id.clone().into()),
            model: Some("model-recovery/original".into()),
            options: Some(json!({"full_access":false})),
            messages: vec![
                json!({"role":"user","content":"fork omitted second"}),
                json!({"role":"assistant","content":"fork omitted third"}),
                json!({"role":"user","content":"fork new prompt"}),
            ],
            ..Default::default()
        };
        let mut headers = HeaderMap::new();
        headers.insert("x-vw-stream-request-id", HeaderValue::from_str(&branch.id).unwrap());
        if !target.is_null() {
            let wrong_directory = super::super::super::stream::chat_stream(
                Query(query_for(FsPath::new(&source.directory))),
                headers.clone(),
                Json(request.clone()),
            )
            .await;
            assert!(
                matches!(wrong_directory, Err(error) if error.status == StatusCode::BAD_REQUEST)
            );
        }
        let response = super::super::super::stream::chat_stream(
            Query(query_for(FsPath::new(&branch.directory))),
            headers,
            Json(request),
        )
        .await
        .unwrap();
        let bytes = tokio::time::timeout(
            Duration::from_secs(30),
            axum::body::to_bytes(
                axum::response::IntoResponse::into_response(response).into_body(),
                2_000_000,
            ),
        )
        .await
        .unwrap()
        .unwrap();
        let events = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(events.contains("chat.done"), "{events}");
        let provider_request = requests.lock().pop().expect("real provider request");
        let messages = provider_request["messages"].as_array().unwrap();
        assert!(messages.iter().any(|message| {
            message["role"] == "system"
                && message["content"].as_str().is_some_and(|content| {
                    content.contains(&format!("working directory is `{}`", branch.directory))
                })
        }));
        let relevant = messages
            .iter()
            .filter_map(|message| {
                let role = message["role"].as_str()?;
                if role == "assistant" && message["tool_calls"].is_array() {
                    let call = &message["tool_calls"][0];
                    assert_eq!(call["id"], "fork-retained-call");
                    Some("call")
                } else {
                    let content = message["content"].as_str()?;
                    match (role, content.trim_matches('"')) {
                        ("user", "fork retained user") => Some("user"),
                        ("tool", "fork retained result") => {
                            assert_eq!(message["tool_call_id"], "fork-retained-call");
                            Some("tool")
                        }
                        ("assistant", "fork retained answer") => Some("assistant"),
                        ("user", "fork new prompt") => Some("new user"),
                        _ => {
                            assert!(
                                !content.contains("fork omitted"),
                                "discarded source history in role {role}"
                            );
                            None
                        }
                    }
                }
            })
            .collect::<Vec<_>>();
        let expected: &[&str] = if boundary == &second {
            &["user", "call", "tool", "assistant", "new user"]
        } else {
            &["new user"]
        };
        assert_eq!(relevant, expected);
        eprintln!(
            "fork model request: source={} target={} sequence={relevant:?}",
            source.directory, branch.directory
        );
        if !target.is_null() {
            assert_ne!(branch.directory, source.directory);
            worktree_cleanup(&source, branch.directory).await;
        }
    }
    server.abort();
}
