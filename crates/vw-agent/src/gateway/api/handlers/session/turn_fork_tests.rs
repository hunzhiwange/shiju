use super::*;
use axum::{
    Extension,
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use crate::gateway::api::handlers::session::shared::with_tenant_instance;
use crate::session::{agent_binding, message, ui_store};
use message::{Info as MessageInfo, Part, WithParts};
use vw_api_transport::id::TenantScope;

async fn request(method: Method, path: &str, directory: &str, body: Value) -> (StatusCode, Value) {
    let response = super::super::super::router()
        .with_state(state())
        .layer(Extension(ui_store::current_tenant_scope()))
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .header("x-directory", directory)
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

fn acceptance_app() -> axum::Router {
    super::super::super::router().with_state(state())
        .layer(Extension(ui_store::current_tenant_scope()))
}
#[tokio::test]
async fn session_fork_before_turn_keeps_only_complete_earlier_turns() {
    gateway_session::fork_acceptance::session_fork_before_turn_keeps_only_complete_earlier_turns::<super::super::super::message_ops::Host, super::super::super::stream::Host>(acceptance_app).await;
}

#[tokio::test]
async fn session_fork_before_turn_rejects_invalid_boundaries_before_creating_targets() {
    gateway_session::fork_acceptance::session_fork_before_turn_rejects_invalid_boundaries_before_creating_targets::<super::super::super::message_ops::Host, super::super::super::stream::Host>(acceptance_app).await;
}

#[tokio::test]
async fn session_fork_before_turn_rolls_back_staged_history_and_worktree_on_write_failure() {
    gateway_session::fork_acceptance::session_fork_before_turn_rolls_back_staged_history_and_worktree_on_write_failure::<super::super::super::message_ops::Host, super::super::super::stream::Host>(acceptance_app).await;
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
        .replace_history(vec![model_types::ChatMessage::user("source cached future")]);

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
async fn fork_selection_locks_when_own_request_starts() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    crate::session::llm::prompt::ensure_host();
    gateway_session::fork_acceptance::fork_selection_locks_when_own_request_starts::<super::super::super::message_ops::Host, super::super::super::stream::Host>(acceptance_app).await;
}

#[cfg(unix)]
#[tokio::test]
async fn worktree_environment_fork_waits_for_setup_and_rejects_failure() {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    let tenant = TenantScope::new("environment-fork", Uuid::new_v4().to_string()).unwrap();
    ui_store::with_tenant_scope(tenant.clone(), async {
        let repo = repository();
        crate::config::update_global_for_tenant(&tenant, json!({"worktree_settings": {"root": repo.path().join("managed")}})).await.unwrap();
        let source = create_session_in(repo.path(), "Source").await;
        let (_, boundary) = persist_turn(&source.id, "prompt", "answer").await;
        let scope = vw_api_transport::ProjectScope::new(tenant, vw_api_transport::ProjectId::from(source.project_id.as_str())).unwrap();
        let mut info = project::get(&scope).await.unwrap().unwrap();
        info.commands = Some(project::Commands {
            start: Some("test -f tracked.txt && echo setup >> setup.txt".into()),
        });
        project::runtime_store().unwrap().update(&scope, &info).unwrap();
        let (status, body) = fork(&source, json!({"beforeTurnId": boundary, "target": {"type": "worktree", "name": format!("environment-{}", Uuid::new_v4())}})).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let forked: agent_session::session::Info = serde_json::from_value(body).unwrap();
        assert_eq!(
            std::fs::read_to_string(FsPath::new(&forked.directory).join("setup.txt")).unwrap(),
            "setup\n"
        );
        crate::worktree::environment::ensure_ready(&forked.directory).unwrap();
        worktree_cleanup(&source, forked.directory).await;

        info.commands = Some(project::Commands { start: Some("exit 31".into()) });
        project::runtime_store().unwrap().update(&scope, &info).unwrap();
        let failed_directory = std::sync::Arc::new(std::sync::Mutex::new(None));
        let captured = failed_directory.clone();
        let project_id = info.id.clone();
        let unsubscribe = crate::bus::global_subscribe(move |event| {
            if event.payload["type"] == "worktree.failed"
                && event.payload["properties"]["projectId"] == project_id
            {
                *captured.lock().unwrap() = event.directory;
            }
        });
        let (_, before) = request(
            Method::GET,
            &format!("/session?projectID={}", source.project_id),
            &source.directory,
            Value::Null,
        )
        .await;
        let (status, body) = fork(&source, json!({"beforeTurnId": boundary, "target": {"type": "worktree", "name": format!("environment-failed-{}", Uuid::new_v4())}})).await;
        unsubscribe();
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        let (_, after) = request(
            Method::GET,
            &format!("/session?projectID={}", source.project_id),
            &source.directory,
            Value::Null,
        )
        .await;
        assert_eq!(before, after, "failed setup must not publish a fork session");
        let directory = failed_directory.lock().unwrap().take().unwrap();
        assert!(crate::worktree::environment::ensure_ready(&directory).is_err());
        worktree_cleanup(&source, directory).await;
    })
    .await;
}

#[tokio::test]
async fn fork_selection_before_start_survives_reload() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    let _fixture = agent_session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let repo = repository();
    let source = create_session_in(repo.path(), "Selection source").await;
    persist_turn(&source.id, "first", "answer").await;
    let (_, boundary) = persist_turn(&source.id, "second", "answer").await;
    let inherited = agent_binding::AgentBinding::from_definition(
        "acp-claude".into(),
        &crate::agent::get("acp-claude").await.unwrap(),
    );
    agent_binding::save_scoped(&source.project_id, &source.id, &inherited).await.unwrap();
    for target in
        [Value::Null, json!({"type":"worktree","name":format!("selection-{}", Uuid::new_v4())})]
    {
        let (status, body) = fork(&source, json!({"beforeTurnId":boundary,"target":target})).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["fork_execution_started"], false);
        let forked: agent_session::session::Info = serde_json::from_value(body).unwrap();
        for _ in 0..2 {
            let (status, body) = request(
                Method::GET,
                &format!("/session/{}", forked.id),
                &forked.directory,
                Value::Null,
            )
            .await;
            assert_eq!(status, StatusCode::OK, "{body}");
            assert_eq!(body["fork_execution_started"], false);
            assert_eq!(history(&forked).await.len(), 2);
        }
        assert!(forked.is_user_navigable());
        let id = forked.id.clone();
        let workspace = forked.directory.clone();
        with_tenant_instance(ui_store::current_tenant_scope(), workspace.clone(), move || {
            Box::pin(async move {
                for (agent, backend) in
                    [("main", None), ("acp-claude", Some("claude")), ("acp-codex", Some("codex"))]
                {
                    let (binding, _) = agent_binding::prepare(
                        &id,
                        Some(agent),
                        backend,
                        Some("model-recovery/original"),
                        FsPath::new(&workspace),
                    )
                    .await
                    .unwrap();
                    assert_eq!(binding.agent, agent);
                }
                Ok(())
            })
        })
        .await
        .unwrap();
        assert_eq!(
            agent_binding::load_scoped(&source.project_id, &source.id)
                .await
                .unwrap()
                .unwrap()
                .agent,
            "acp-claude"
        );
        if !target.is_null() {
            worktree_cleanup(&source, forked.directory).await;
        }
    }
    assert_eq!(history(&source).await.len(), 4);
}

#[cfg(unix)]
#[path = "fork_acp_tests.rs"]
mod fork_acp_tests;
