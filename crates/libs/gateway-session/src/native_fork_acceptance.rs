//! Exercise turn forks through the desktop read endpoints and a real model transport.
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use axum::{Json, Router, routing::post};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tower::ServiceExt;

use session_engine::message::{Part, ToolState, WithParts};
use session_info::Info;
use crate::profile_acceptance_host::ProfileAcceptanceHost;
use crate::stream_host::StreamHost;
use vw_api_transport::id::TenantScope;
use vw_api_session::session::{GatewaySessionMessagePage, GatewaySessionUiSummary};
use session_ui_types::{ChatRole, SessionMessageRecord};

const PROMPTS: [&str; 3] = ["fork-retained-user", "fork-discarded-second", "fork-discarded-third"];
const ANSWERS: [&str; 3] =
    ["fork-retained-answer", "fork-discarded-second-answer", "fork-discarded-third-answer"];
const TOOL: &str = "file_read";
const CALL: &str = "fork-retained-call";
const FILE: &str = "tracked.txt";
const RESULT: &str = "fork-retained-result";
const NEW_USER: &str = "fork-continue-user";
const NEW_ANSWER: &str = "fork-continue-answer";
const SUMMARY: &str = "fork-retained-summary";
const LATE_SUMMARY: &str = "fork-discarded-summary";
const PHASE: &str = "SHIJU_FORK_ACCEPTANCE_PHASE";

async fn request(
    app: &Router,
    method: Method,
    route: &str,
    directory: &str,
    payload: Value,
) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(route)
                .header("x-directory", directory)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes =
        tokio::time::timeout(Duration::from_secs(30), to_bytes(response.into_body(), 2_000_000))
            .await
            .expect("gateway response timed out")
            .unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert_eq!(status, StatusCode::OK, "{route} in {directory}: {body}");
    body
}

async fn read_desktop_history(app: &Router, session: &Info) -> Vec<SessionMessageRecord> {
    let route = format!("/session/{}/ui", session.id);
    let read = |suffix: &str| {
        let route = format!("{route}/{suffix}");
        async move { request(app, Method::GET, &route, &session.directory, Value::Null).await }
    };
    let summary: GatewaySessionUiSummary = serde_json::from_str(&read("summary").await).unwrap();
    let page: GatewaySessionMessagePage<SessionMessageRecord> =
        serde_json::from_str(&read("messages").await).unwrap();
    assert_eq!(summary.id, session.id);
    assert_eq!(summary.message_count, page.items.len());
    assert_eq!(page.total_count, page.items.len());
    assert!(page.next_cursor.is_none());
    assert!(page.items.windows(2).all(|pair| pair[0].seq_no < pair[1].seq_no));
    assert!(page.items.iter().all(|record| record.message_id.is_some()));
    page.items
}

async fn send(app: &Router, session: &Info, prompt: &str, answer: &str) -> String {
    let events = request(
        app,
        Method::POST,
        "/chat/stream",
        &session.directory,
        json!({
            "session_id": session.id, "agent": "main", "model": "model-recovery/original",
            "allowed_tools": [TOOL], "options": {"full_access": false},
            "messages": [{"role": "user", "content": prompt}],
        }),
    )
    .await;
    assert!(events.contains("chat.done") && !events.contains("chat.error"), "{events}");
    assert!(events.contains(answer), "{events}");
    events
}

fn git(directory: &Path, args: &[&str]) -> String {
    let output =
        std::process::Command::new("git").current_dir(directory).args(args).output().unwrap();
    assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

fn create_repo(directory: &Path) {
    std::fs::create_dir(directory).unwrap();
    git(directory, &["init", "--quiet"]);
    std::fs::write(directory.join(FILE), RESULT).unwrap();
    git(directory, &["add", FILE]);
    git(
        directory,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "Initial fixture",
        ],
    );
}

async fn serve_model(requests: Arc<Mutex<Vec<Value>>>) -> String {
    let model = Router::new().route(
        "/v1/chat/completions",
        post(move |Json(body): Json<Value>| {
            let requests = Arc::clone(&requests);
            async move {
                let messages = body["messages"].as_array().unwrap();
                let user = messages.iter().rfind(|msg| msg["role"] == "user").unwrap();
                let answer = match user["content"].as_str().unwrap() {
                    prompt if prompt == PROMPTS[0] => ANSWERS[0],
                    prompt if prompt == PROMPTS[1] => ANSWERS[1],
                    prompt if prompt == PROMPTS[2] => ANSWERS[2],
                    NEW_USER => NEW_ANSWER,
                    other => panic!("unexpected user input: {other}"),
                };
                let use_tool = answer == ANSWERS[0] && !messages.iter().any(|msg| msg["role"] == "tool");
                requests.lock().unwrap().push(body);
                let (delta, reason) = if use_tool {
                    (json!({"tool_calls": [{"index":0, "id":CALL, "type":"function", "function":{
                        "name":TOOL, "arguments":json!({"path":FILE}).to_string()
                    }}]}), "tool_calls")
                } else {
                    (json!({"content": answer}), "stop")
                };
                let chunk = json!({"id":"fork-fixture","object":"chat.completion.chunk","created":0,
                    "model":"original","choices":[{"index":0,"delta":delta,"finish_reason":reason}]});
                ([( "content-type", "text/event-stream")], format!("data: {chunk}\n\ndata: [DONE]\n\n"))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, model).await.unwrap() });
    format!("http://{address}/v1")
}

mod history;
use history::*;
pub use history::fork_acceptance_negative_detects_discarded_model_history;

async fn native_model_history<H: ProfileAcceptanceHost>(info: &Info) -> Vec<Value> {
    let info = info.clone();
    H::with_tenant_instance(TenantScope::default(), info.directory.clone(), move || {
        Box::pin(async move {
            let (_, history) = H::Stream::load_fork_history(&info)
                .await
                .map_err(|err| gateway_types::ApiError::internal(err.to_string()))?;
            Ok(history)
        })
    })
    .await
    .unwrap()
}

async fn seed_checkpoints<H: ProfileAcceptanceHost>(info: &Info, retained: Vec<Value>, full: Vec<Value>) {
    let info = info.clone();
    H::with_tenant_instance(
        TenantScope::default(),
        info.directory.clone(),
        move || {
            Box::pin(async move {
                for (history, summary) in [(&retained, SUMMARY), (&full, LATE_SUMMARY)] {
                    session_engine::compaction_checkpoint::save(&session_engine::compaction_checkpoint::scope_for_directory(&session_store::ui_store::current_tenant_scope(), &agent_project::instance::directory()), &info.id, history, summary, None)
                        .await
                        .map_err(gateway_types::ApiError::internal)?;
                }
                Ok(())
            })
        },
    )
    .await
    .unwrap();
}

async fn fork(
    app: &Router,
    source: &Info,
    boundary: &str,
    prefix: Prefix,
    tool_output: &str,
    worktree: bool,
) -> Branch {
    let target =
        if worktree { json!({"type": "worktree"}) } else { json!({"type": "sameDirectory"}) };
    let body = request(
        app,
        Method::POST,
        &format!("/session/{}/fork", source.id),
        &source.directory,
        json!({"beforeTurnId": boundary, "target": target}),
    )
    .await;
    let info: Info = serde_json::from_str(&body).unwrap();
    assert_ne!(info.id, source.id);
    assert_eq!(info.project_id, source.project_id);
    assert_eq!(info.directory != source.directory, worktree);
    assert_eq!(std::fs::read_to_string(Path::new(&info.directory).join(FILE)).unwrap(), RESULT);
    assert_eq!(
        git(Path::new(&info.directory), &["rev-parse", "--show-toplevel"]).trim(),
        info.directory
    );
    assert_eq!(
        git(Path::new(&info.directory), &["rev-parse", "HEAD"]),
        git(Path::new(&source.directory), &["rev-parse", "HEAD"])
    );
    let initial_history = read_desktop_history(app, &info).await;
    assert_initial_history(&initial_history, prefix, tool_output);
    Branch { info, prefix, initial_history }
}

async fn run_fixture<H: ProfileAcceptanceHost>(root: &Path, test_name: &str) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let endpoint = serve_model(Arc::clone(&requests)).await;
    agent_config::runtime::update(json!({
        "providers": {"model-recovery": {"api": endpoint, "env": ["SHIJU_FORK_FIXTURE_KEY"],
            "models": {"original": {"status": "active"}}}},
        "agents":{"main":{"defaults":{"skills":{"mode":"fixed","value":[]},
            "mcp":{"mode":"fixed","value":[]}}}}
    }))
    .await
    .unwrap();
    H::invalidate_cache().await;

    let directory = root.join("repo");
    create_repo(&directory);
    let app = H::session_router();
    let source: Info = serde_json::from_str(
        &request(
            &app,
            Method::POST,
            "/session",
            directory.to_str().unwrap(),
            json!({"title": "Fork fixture"}),
        )
        .await,
    )
    .unwrap();

    send(&app, &source, PROMPTS[0], ANSWERS[0]).await;
    let retained = native_model_history::<H>(&source).await;
    let native: Vec<WithParts> = serde_json::from_str(
        &request(
            &app,
            Method::GET,
            &format!("/session/{}/message", source.id),
            &source.directory,
            Value::Null,
        )
        .await,
    )
    .unwrap();
    let native_tool_output = native
        .iter()
        .flat_map(|message| &message.parts)
        .find_map(|part| match part {
            Part::Tool(tool) if tool.call_id == CALL => match &tool.state {
                ToolState::Completed(state) => Some(state.output.as_str()),
                _ => None,
            },
            _ => None,
        })
        .expect("persisted file tool result");
    assert!(native_tool_output.contains(RESULT), "file tool output: {native_tool_output}");
    assert!(native_tool_output.contains("<file_link>"));
    let wire_tool_output = {
        let captured = requests.lock().unwrap();
        assert_eq!(captured.len(), 2, "first turn runs exactly one tool round");
        let messages = captured[1]["messages"].as_array().unwrap();
        let tool = messages.iter().find(|msg| msg["role"] == "tool").unwrap();
        assert_eq!(tool["tool_call_id"], CALL);
        let output = tool["content"].as_str().unwrap();
        assert!(output.contains(RESULT), "file tool output: {output}");
        assert!(!output.contains("<file_link>"), "UI-only tool link: {output}");
        output.to_owned()
    };
    for (prompt, answer) in PROMPTS.into_iter().zip(ANSWERS).skip(1) {
        send(&app, &source, prompt, answer).await;
    }

    let original = read_desktop_history(&app, &source).await;
    assert_eq!(original.len(), 6);
    for (index, turn) in original.chunks_exact(2).enumerate() {
        assert_eq!(turn[0].message.role, ChatRole::User);
        assert_eq!(turn[0].message.content, PROMPTS[index]);
        assert_eq!(turn[1].message.role, ChatRole::Assistant);
        assert!(turn[1].message.content.contains(ANSWERS[index]));
    }
    let first = original[0].message_id.as_deref().unwrap();
    let second = original[2].message_id.as_deref().unwrap();
    let mut branches = Vec::new();
    for worktree in [false, true] {
        for (boundary, prefix) in [(second, Prefix::Full), (first, Prefix::Empty)] {
            branches
                .push(fork(&app, &source, boundary, prefix, native_tool_output, worktree).await);
        }
    }
    let full = native_model_history::<H>(&source).await;
    assert!(full.len() > retained.len());
    seed_checkpoints::<H>(&source, retained, full).await;
    for worktree in [false, true] {
        branches.push(
            fork(&app, &source, second, Prefix::Compacted, native_tool_output, worktree).await,
        );
    }
    assert_eq!(read_desktop_history(&app, &source).await, original);
    let ids =
        branches.iter().map(|branch| &branch.info.id).collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), branches.len());
    assert!(branches.iter().all(|branch| Path::new(&branch.info.directory).starts_with(root)));

    std::fs::write(root.join("branches.json"), serde_json::to_vec(&branches).unwrap()).unwrap();
    let before = requests.lock().unwrap().len();
    run_process(root, "restart", test_name).await;
    {
        let captured = requests.lock().unwrap();
        let continuations = &captured[before..];
        assert_eq!(continuations.len(), branches.len(), "one model request per resumed branch");
        for (request, branch) in continuations.iter().zip(&branches) {
            assert_continuation(request, branch, &wire_tool_output);
        }
    }
    assert_eq!(read_desktop_history(&app, &source).await, original);
}

async fn run_process(root: &Path, phase: &str, test: &str) {
    let output = tokio::time::timeout(
        Duration::from_secs(180),
        tokio::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .env("HOME", root)
            .env("TMPDIR", root)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("VW_MODEL_RECOVERY_TEST_ROOT", root)
            .env("VIBEWINDOW_TEST_ROOT", root)
            // The test session store otherwise creates a different SQLite file per PID.
            .env("VW_ACP_MOUNT_RECOVERY_DB_ROOT", root)
            .env("VIBEWINDOW_CONFIG_DIR", root.join("config"))
            .env("VW_RECOVERY_WINDOW_HTTP", "1")
            .env("SHIJU_FORK_FIXTURE_KEY", "loopback-fixture-key")
            .env(PHASE, phase)
            .current_dir(root)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .unwrap_or_else(|_| panic!("{phase} process timed out"))
    .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "{phase} process: {stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

pub async fn fork_acceptance_restarts_and_resumes_both_targets<H: ProfileAcceptanceHost>(test_name: &str) {
    match std::env::var(PHASE).as_deref() {
        Ok("create") => run_fixture::<H>(&std::env::current_dir().unwrap(), test_name).await,
        Ok("restart") => {
            let branches: Vec<Branch> =
                serde_json::from_slice(&std::fs::read("branches.json").unwrap()).unwrap();
            let app = H::session_router();
            for branch in branches {
                assert_eq!(read_desktop_history(&app, &branch.info).await, branch.initial_history);
                let events = send(&app, &branch.info, NEW_USER, NEW_ANSWER).await;
                assert!(
                    !events.contains("chat.tool") && !events.contains(CALL),
                    "retained tool must not run again: {events}"
                );
            }
        }
        Err(_) => {
            let root = tempfile::tempdir().unwrap();
            run_process(&root.path().canonicalize().unwrap(), "create", test_name).await;
        }
        Ok(phase) => panic!("unknown acceptance phase: {phase}"),
    }
}
