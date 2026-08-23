// 覆盖实例路径、版本控制状态、事件流与释放流程。

use super::*;
use axum::body::{Body, BodyDataStream, to_bytes};
use axum::extract::Query;
use axum::http::{HeaderMap, HeaderValue, Method, Request, StatusCode};
use axum::response::IntoResponse;
use tokio_stream::StreamExt;
use tower::ServiceExt;

#[test]
fn router_builds_with_app_state() {
    let _: axum::Router<()> = router();
    let _: axum::Router<crate::app::agent::gateway::AppState> = router();
}

fn query_for(path: &std::path::Path) -> InstanceQuery {
    InstanceQuery { directory: Some(path.to_string_lossy().to_string()) }
}

fn headers_for(path: &std::path::Path) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-vibewindow-directory",
        HeaderValue::from_str(&path.to_string_lossy()).expect("valid directory header"),
    );
    headers
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    serde_json::from_slice(&bytes).expect("response should be json")
}

#[test]
fn response_structs_serialize_stable_api_shapes() {
    let path = serde_json::to_value(PathInfo {
        home: "/home/test".to_string(),
        state: "/state/test".to_string(),
        config: "/config/test".to_string(),
        worktree: "/worktree/test".to_string(),
        directory: "/directory/test".to_string(),
    })
    .expect("path info should serialize");
    assert_eq!(
        path,
        serde_json::json!({
            "home": "/home/test",
            "state": "/state/test",
            "config": "/config/test",
            "worktree": "/worktree/test",
            "directory": "/directory/test",
        })
    );

    let vcs = serde_json::to_value(VcsInfo { branch: "feature/test".to_string() })
        .expect("vcs info should serialize");
    assert_eq!(vcs, serde_json::json!({"branch": "feature/test"}));
}

async fn next_sse_chunk(stream: &mut BodyDataStream) -> String {
    let bytes = tokio::time::timeout(std::time::Duration::from_secs(1), stream.next())
        .await
        .expect("sse chunk should arrive")
        .expect("sse stream should continue")
        .expect("sse chunk should be readable");
    String::from_utf8(bytes.to_vec()).expect("sse chunk should be utf8")
}

async fn next_sse_chunk_containing(stream: &mut BodyDataStream, needle: &str) -> String {
    for _ in 0..8 {
        let chunk = next_sse_chunk(stream).await;
        if chunk.contains(needle) {
            return chunk;
        }
    }
    panic!("sse chunk containing {needle} should arrive");
}

async fn drain_sse_chunks(stream: &mut BodyDataStream) -> Vec<String> {
    let mut chunks = Vec::new();
    loop {
        match tokio::time::timeout(std::time::Duration::from_millis(25), stream.next()).await {
            Ok(Some(Ok(bytes))) => {
                chunks.push(String::from_utf8(bytes.to_vec()).expect("sse chunk should be utf8"));
            }
            Ok(Some(Err(err))) => panic!("sse chunk should be readable: {err}"),
            Ok(None) | Err(_) => break,
        }
    }
    chunks
}

#[tokio::test]
async fn router_path_get_prefers_query_directory_over_header() {
    let query_dir = tempfile::tempdir().expect("query tempdir");
    let header_dir = tempfile::tempdir().expect("header tempdir");
    let app: axum::Router<()> = router();
    let uri = format!("/path?directory={}", query_dir.path().to_string_lossy());
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .header("x-vibewindow-directory", header_dir.path().to_string_lossy().as_ref())
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["directory"], query_dir.path().to_string_lossy().as_ref());
}

#[tokio::test]
async fn router_vcs_get_uses_directory_header_when_query_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let app: axum::Router<()> = router();
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/vcs")
                .header("x-vibewindow-directory", temp.path().to_string_lossy().as_ref())
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await, serde_json::json!({"branch": ""}));
}

#[tokio::test]
async fn router_instance_dispose_accepts_post_only() {
    let temp = tempfile::tempdir().expect("tempdir");
    let app: axum::Router<()> = router();
    let uri = format!("/instance/dispose?directory={}", temp.path().to_string_lossy());

    let get_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&uri)
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("route should respond");
    assert_eq!(get_response.status(), StatusCode::METHOD_NOT_ALLOWED);

    let post_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("route should respond");
    assert_eq!(post_response.status(), StatusCode::OK);
    assert_eq!(response_json(post_response).await, serde_json::json!(true));
}

#[tokio::test]
async fn path_get_reports_global_and_instance_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    let Json(info) = path_get(Query(query_for(temp.path())), HeaderMap::new())
        .await
        .expect("path_get should succeed");

    assert!(!info.home.is_empty());
    assert!(!info.state.is_empty());
    assert!(!info.config.is_empty());
    assert_eq!(info.directory, temp.path().to_string_lossy());
    assert!(!info.worktree.trim().is_empty());
}

#[tokio::test]
async fn path_get_uses_directory_header_when_query_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let Json(info) = path_get(Query(InstanceQuery { directory: None }), headers_for(temp.path()))
        .await
        .expect("path_get should succeed");

    assert_eq!(info.directory, temp.path().to_string_lossy());
}

#[test]
fn branch_for_worktree_returns_empty_for_blank_worktree() {
    assert_eq!(branch_for_worktree("  "), "");
}

#[test]
fn branch_for_worktree_returns_empty_when_directory_is_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let missing = temp.path().join("missing-worktree");

    assert_eq!(branch_for_worktree(&missing.to_string_lossy()), "");
}

#[tokio::test]
async fn vcs_get_returns_empty_branch_for_non_git_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let Json(info) = vcs_get(Query(query_for(temp.path())), HeaderMap::new())
        .await
        .expect("vcs_get should succeed");

    assert_eq!(info.branch, "");
}

#[tokio::test]
async fn vcs_get_reports_current_git_branch() {
    let temp = tempfile::tempdir().expect("tempdir");
    let repo = git2::Repository::init(temp.path()).expect("repo init");
    std::fs::write(temp.path().join("README.md"), "test").expect("readme should be written");
    let mut index = repo.index().expect("index");
    index.add_path(std::path::Path::new("README.md")).expect("add");
    index.write().expect("index write");
    let tree_id = index.write_tree().expect("tree");
    let tree = repo.find_tree(tree_id).expect("tree lookup");
    let sig = git2::Signature::now("Test User", "test@example.invalid").expect("signature");
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).expect("commit");
    repo.branch(
        "unit-test-branch",
        &repo.head().expect("head").peel_to_commit().expect("commit"),
        false,
    )
    .expect("branch");
    repo.set_head("refs/heads/unit-test-branch").expect("checkout branch ref");
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .expect("checkout worktree");

    let Json(info) = vcs_get(Query(query_for(temp.path())), HeaderMap::new())
        .await
        .expect("vcs_get should succeed");

    assert_eq!(info.branch, "unit-test-branch");
}

#[tokio::test]
async fn instance_event_sse_builds_event_stream_response() {
    let temp = tempfile::tempdir().expect("tempdir");
    let response = instance_event_sse(Query(query_for(temp.path())), HeaderMap::new())
        .await
        .expect("sse should be created")
        .into_response();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(
        response.headers().get(axum::http::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()),
        Some("text/event-stream")
    );
}

#[tokio::test]
async fn instance_event_sse_sends_connected_event() {
    let temp = tempfile::tempdir().expect("tempdir");
    let response = instance_event_sse(Query(query_for(temp.path())), HeaderMap::new())
        .await
        .expect("sse should be created")
        .into_response();
    let mut stream = response.into_body().into_data_stream();

    let chunk = next_sse_chunk(&mut stream).await;

    assert!(chunk.contains(r#""type":"server.connected""#), "unexpected chunk: {chunk}");
}

#[tokio::test(start_paused = true)]
async fn instance_event_sse_emits_initial_and_periodic_heartbeat_events() {
    let temp = tempfile::tempdir().expect("tempdir");
    let response = instance_event_sse(Query(query_for(temp.path())), HeaderMap::new())
        .await
        .expect("sse should be created")
        .into_response();
    let mut stream = response.into_body().into_data_stream();

    let connected = next_sse_chunk(&mut stream).await;
    assert!(connected.contains(r#""type":"server.connected""#));

    let initial_heartbeat = next_sse_chunk(&mut stream).await;
    assert!(initial_heartbeat.contains(r#""type":"server.heartbeat""#));

    tokio::time::advance(std::time::Duration::from_secs(30)).await;
    next_sse_chunk_containing(&mut stream, r#""type":"server.heartbeat""#).await;
}

#[tokio::test]
async fn instance_event_sse_filters_events_by_instance_directory() {
    const TEST_EVENT: bus::Definition = bus::Definition { r#type: "instance.test.event" };

    let temp = tempfile::tempdir().expect("tempdir");
    let other = tempfile::tempdir().expect("other tempdir");
    let response = instance_event_sse(Query(query_for(temp.path())), HeaderMap::new())
        .await
        .expect("sse should be created")
        .into_response();
    let mut stream = response.into_body().into_data_stream();
    let _ = drain_sse_chunks(&mut stream).await;

    crate::app::agent::bus::publish(
        TEST_EVENT,
        serde_json::json!({"id": "skip"}),
        Some(other.path().to_string_lossy().to_string()),
    )
    .expect("event should publish");
    let skipped = drain_sse_chunks(&mut stream).await;
    assert!(skipped.iter().all(|chunk| !chunk.contains("skip")));

    crate::app::agent::bus::publish(
        TEST_EVENT,
        serde_json::json!({"id": "match"}),
        Some(temp.path().to_string_lossy().to_string()),
    )
    .expect("event should publish");
    let chunk = next_sse_chunk(&mut stream).await;
    assert!(chunk.contains(r#""type":"instance.test.event""#), "unexpected chunk: {chunk}");
    assert!(chunk.contains(r#""id":"match""#), "unexpected chunk: {chunk}");
}

#[tokio::test]
async fn instance_event_sse_forwards_events_without_directory_scope() {
    const TEST_EVENT: bus::Definition = bus::Definition { r#type: "instance.global.test.event" };

    let temp = tempfile::tempdir().expect("tempdir");
    let response = instance_event_sse(Query(query_for(temp.path())), HeaderMap::new())
        .await
        .expect("sse should be created")
        .into_response();
    let mut stream = response.into_body().into_data_stream();
    let _ = drain_sse_chunks(&mut stream).await;

    crate::app::agent::bus::publish(TEST_EVENT, serde_json::json!({"scope": "global"}), None)
        .expect("global event should publish");

    let chunk = next_sse_chunk(&mut stream).await;
    assert!(chunk.contains(r#""type":"instance.global.test.event""#));
    assert!(chunk.contains(r#""scope":"global""#));
}

#[tokio::test(start_paused = true)]
async fn instance_event_sse_stops_when_receiver_is_dropped() {
    let temp = tempfile::tempdir().expect("tempdir");
    let sse = instance_event_sse(Query(query_for(temp.path())), HeaderMap::new())
        .await
        .expect("sse should be created");

    drop(sse);
    tokio::task::yield_now().await;
    tokio::time::advance(std::time::Duration::from_secs(30)).await;
}

#[tokio::test]
async fn instance_dispose_returns_true_and_clears_cached_instance() {
    let temp = tempfile::tempdir().expect("tempdir");

    let Json(disposed) = instance_dispose(Query(query_for(temp.path())), HeaderMap::new())
        .await
        .expect("dispose should succeed");

    assert!(disposed);
}

#[tokio::test]
async fn instance_dispose_clears_directory_approval_manager() {
    let temp = tempfile::tempdir().expect("tempdir");
    let directory = temp.path().to_string_lossy().to_string();
    approval_state::clear_approval_manager_for_directory(&directory);

    let before = crate::app::agent::project::instance::provide(temp.path(), None, || {
        Box::pin(async {
            Ok::<_, crate::app::agent::project::Error>(
                approval_state::approval_manager_for_current_instance().await,
            )
        })
    })
    .await
    .expect("instance context should be provided")
    .expect("approval manager should load");

    let Json(disposed) = instance_dispose(Query(query_for(temp.path())), HeaderMap::new())
        .await
        .expect("dispose should succeed");
    assert!(disposed);

    let after = crate::app::agent::project::instance::provide(temp.path(), None, || {
        Box::pin(async {
            Ok::<_, crate::app::agent::project::Error>(
                approval_state::approval_manager_for_current_instance().await,
            )
        })
    })
    .await
    .expect("instance context should be provided")
    .expect("approval manager should reload");

    assert!(!std::sync::Arc::ptr_eq(&before, &after));
    approval_state::clear_approval_manager_for_directory(&directory);
}

#[tokio::test]
async fn instance_dispose_publishes_disposed_event_for_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let directory = temp.path().to_string_lossy().to_string();
    let expected_directory = directory.clone();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let unsub = crate::app::agent::bus::global_subscribe(move |evt| {
        if evt.payload.get("type").and_then(serde_json::Value::as_str)
            == Some("server.instance.disposed")
            && evt.directory.as_deref() == Some(expected_directory.as_str())
        {
            let _ = tx.send(evt);
        }
    });

    let Json(disposed) = instance_dispose(Query(query_for(temp.path())), HeaderMap::new())
        .await
        .expect("dispose should succeed");
    assert!(disposed);

    let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("dispose event should arrive")
        .expect("dispose event should be sent");
    unsub();

    assert_eq!(event.directory.as_deref(), Some(directory.as_str()));
    assert_eq!(event.payload["properties"]["directory"], directory);
}
