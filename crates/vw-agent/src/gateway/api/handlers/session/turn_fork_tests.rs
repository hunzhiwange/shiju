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
