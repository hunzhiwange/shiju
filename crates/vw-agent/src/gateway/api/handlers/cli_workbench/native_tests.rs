use axum::{
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::Request,
};
use tower::ServiceExt;

use super::*;

const CAPABILITY: &str = "/cli-workbench/terminal-capability";

#[tokio::test]
async fn cli_workbench_native_capability_requires_a_direct_local_gateway() {
    let tenant = TenantScope::new("native-fixture", "capability").unwrap();
    let plain = router::<()>().layer(Extension(tenant));
    let local = plain.clone().layer(Extension(LocalWorkbenchGateway::default()));
    for (router, peer, header, available) in [
        (local.clone(), "127.0.0.1:1234", None, cfg!(target_os = "macos")),
        (plain, "127.0.0.1:1234", None, false),
        (local.clone(), "192.0.2.2:1234", None, false),
        (local.clone(), "127.0.0.1:1234", Some(("forwarded", "for=127.0.0.1")), false),
        (local, "127.0.0.1:1234", Some(("origin", "https://example.test")), false),
    ] {
        let mut request = Request::builder()
            .uri(CAPABILITY)
            .extension(ConnectInfo(peer.parse::<std::net::SocketAddr>().unwrap()));
        if let Some((name, value)) = header {
            request = request.header(name, value);
        }
        let response = router.oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&body).unwrap(),
            json!({"available": available})
        );
    }
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn cli_workbench_native_drafts_are_scoped_and_manual() {
    let tenant = TenantScope::new("native-fixture", "drafts").unwrap();
    let project = "native-project";
    let scope = project_scope(tenant.clone(), project.into()).ok().unwrap();
    let native = "native-draft-fixture";
    let shell = "native-shell-fixture";

    struct Cleanup(ProjectScope);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let mut workbench = WORKBENCH.lock();
            workbench.sessions.retain(|_, session| session.scope != self.0);
            workbench.deliveries.retain(|(scope, _), _| scope != &self.0);
        }
    }
    let _cleanup = Cleanup(scope.clone());
    for (id, target) in [
        (native, CliWorkbenchTarget::NativeCli { app_id: "codex".into() }),
        (shell, CliWorkbenchTarget::Shell),
    ] {
        let (status, Json(session)) = insert_session(
            Session {
                scope: scope.clone(),
                info: CliWorkbenchSession {
                    id: id.into(),
                    project_id: project.into(),
                    directory: "/fixture".into(),
                    state: ProcessState::Running,
                    capabilities: capabilities(&target),
                    target,
                },
                turn: None,
                terminal: None,
                child_access: None,
            },
            None,
        )
        .ok()
        .unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(session.id, id);
    }
    // The fixture has no PTY. Collaboration can create a draft independently of I/O.
    let core = router::<()>().layer(Extension(LocalWorkbenchGateway::default()));
    let local = core.clone().layer(Extension(tenant));
    let foreign = core.layer(Extension(TenantScope::new("native-fixture", "other").unwrap()));
    let sessions = format!("/projects/{project}/cli-workbench/sessions");
    let native_path = format!("{sessions}/{native}");
    let messages = format!("{native_path}/messages");
    let draft = format!("{native_path}/drafts/n1");
    let content = "line one\nline two";
    let request = |method: &str, path: &str, body: Value| {
        Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .extension(ConnectInfo(std::net::SocketAddr::from(([127, 0, 0, 1], 1234))))
            .body(Body::from(body.to_string()))
            .unwrap()
    };

    for _ in 0..2 {
        let response = local
            .clone()
            .oneshot(request("POST", &messages, json!({"request_id": "n1", "text": content})))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        let message: CliWorkbenchMessage = serde_json::from_slice(&body).unwrap();
        assert_eq!(message.state, MessageState::PendingSubmission);
        assert!(message.manual_submission && message.output.is_empty());
        assert_eq!(message.revision, 1);
    }
    let response = local
        .clone()
        .oneshot(request(
            "POST",
            &format!("{sessions}/{shell}/messages"),
            json!({"request_id": "s1", "text": content}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let list = format!("{sessions}/drafts");
    let response = local.clone().oneshot(request("GET", &list, Value::Null)).await.unwrap();
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    let entries: Vec<vw_api_types::cli_workbench::CliWorkbenchDraft> =
        serde_json::from_slice(&body).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].text, content);

    let response = foreign.oneshot(request("POST", &draft, json!("cancel"))).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    for _ in 0..2 {
        let response =
            local.clone().oneshot(request("POST", &draft, json!("mark_submitted"))).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        let message: CliWorkbenchMessage = serde_json::from_slice(&body).unwrap();
        assert_eq!(message.state, MessageState::Submitted);
        assert!(message.is_settled());
        assert_eq!(message.revision, 2);
    }
    let response = local.clone().oneshot(request("POST", &draft, json!("cancel"))).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let response = local
        .clone()
        .oneshot(request("POST", &messages, json!({"request_id": "n2", "text": content})))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response = local
        .clone()
        .oneshot(request("POST", &format!("{native_path}/drafts/n2"), json!("cancel")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    let cancelled: CliWorkbenchMessage = serde_json::from_slice(&body).unwrap();
    assert_eq!(cancelled.error, Some(ErrorCode::Cancelled));
    assert!(cancelled.is_settled());

    let response = local.oneshot(request("GET", &list, Value::Null)).await.unwrap();
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&body).unwrap(), json!([]));
}
