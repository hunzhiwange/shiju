use super::*;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use agent_client_protocol::McpServerStdio;
use axum::body::Body;
use model_transport::RelayIdentity;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use tower::ServiceExt;

const SERVER: &str = r#"
while IFS= read -r line; do
    case "$line" in
        *'"id":'*) id=${line#*\"id\":}; id=${id%%[,}]*};;
        *) continue;;
    esac
    case "$line" in
        *'"method":"initialize"'*)
            result='{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"fixture","version":"1"}}';;
        *'"method":"tools/list"'*)
            result='{"tools":[{"name":"echo","inputSchema":{"type":"object"}},{"name":"denied","inputSchema":{"type":"object"}}]}';;
        *'"method":"tools/call"'*)
            if [ -n "${APPROVAL_AUDIT:-}" ]; then
                case "$line" in *'"shijuCallId"'*) exit 3;; esac
                printf 'executed\n' >> "$APPROVAL_AUDIT"
            fi
            result='{"content":[{"type":"text","text":"fixture result"}]}';;
        *) exit 2;;
    esac
    printf '{"jsonrpc":"2.0","id":%s,"result":%s}\n' "$id" "$result"
done
"#;

fn binding(session: &str) -> Arc<RelayBinding> {
    RelayBinding::for_prompt(RelayIdentity {
        tenant: Default::default(),
        project: "project".into(),
        session: session.into(),
        provider: "provider".into(),
        model: "provider/model".into(),
        generation: 0,
    })
    .unwrap()
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
async fn post(
    projection: &Projection,
    token: &str,
    method: &str,
    params: Value,
) -> (StatusCode, Value) {
    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/shiju-mimo-mcp/fixture")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(
            json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}).to_string(),
        ))
        .unwrap();
    let response = projection.router.clone().oneshot(request).await.unwrap();
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, BODY_LIMIT).await.unwrap();
    (parts.status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[tokio::test]
async fn mimo_mcp_projects_filters_calls_and_revokes_the_selected_catalog() {
    let workspace = tempfile::tempdir().unwrap();
    let mut service =
        McpServerStdio::new("fixture", "/bin/bash").args(vec!["-c".into(), SERVER.into()]);
    service.meta = Some(Map::from_iter([(
        "shiju.mimo.toolPolicy".into(),
        json!({"allow":["echo","denied"],"deny":["denied"]}),
    )]));
    let launch = binding("first:mimo-code");
    let projection = prepare(
        &[AcpMcpServer::Stdio(service)],
        workspace.path().to_str().unwrap(),
        launch.clone(),
    )
    .await
    .unwrap();
    assert_eq!(projection.tools, ["fixture_echo"]);
    assert_eq!(
        post(&projection, launch.bearer_token(), "tools/list", json!({})).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        post(&projection, binding("other:mimo-code").bearer_token(), "tools/list", json!({}))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let (status, listed) = post(&projection, projection.token(), "tools/list", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed["result"]["tools"].as_array().unwrap().len(), 1);
    assert_eq!(listed["result"]["tools"][0]["name"], "echo");
    let (_, denied) =
        post(&projection, projection.token(), "tools/call", json!({"name":"denied"})).await;
    assert_eq!(denied["error"]["code"], -32602);
    let (_, called) =
        post(&projection, projection.token(), "tools/call", json!({"name":"echo","arguments":{}}))
            .await;
    assert_eq!(called["result"]["content"][0]["text"], "fixture result");

    launch.revoke();
    assert_eq!(
        post(&projection, projection.token(), "tools/list", json!({})).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[test]
fn vtcode_mcp_approval_is_bound_to_name_and_arguments_and_consumed_once() {
    let grants = super::VtcodeApproval::new(false);
    let tool = "mcp__graph__query";
    let args = serde_json::json!({"search": "createRueSiteApp"});
    assert!(!grants.consume(tool, None, &args));
    assert!(grants.issue(tool, "call", &args, Box::new(|| true)));
    assert!(!grants.consume(tool, None, &serde_json::json!({"search":"other"})));
    assert!(!grants.consume(tool, None, &args));
    assert!(grants.issue(tool, "call", &args, Box::new(|| true)));
    assert!(grants.consume(tool, None, &args));
    assert!(!grants.consume(tool, None, &args));
}

#[test]
fn vtcode_mcp_approval_rejects_revoked_prompt_before_dispatch() {
    let grants = super::VtcodeApproval::new(false);
    let args = serde_json::json!({"path": "/tmp/shiju-approval/note.txt"});
    let tool = "mcp__selected__write";
    let cancellation = tokio_util::sync::CancellationToken::new();
    let prompt = cancellation.clone();
    assert!(grants.issue(tool, "call", &args, Box::new(move || !prompt.is_cancelled())));
    cancellation.cancel();
    assert!(!grants.consume(tool, None, &args));
}

#[tokio::test]
async fn acp_platform_approval_dispatch_binds_call_source_and_arguments() {
    let scratch = tempfile::tempdir().unwrap();
    let audit = scratch.path().join("called");
    let launch = binding("first:vtcode");
    let mut command = Command::new("/bin/bash");
    command.args(["-c", SERVER]).env("APPROVAL_AUDIT", &audit);
    let peer = mcp_client::client::open_isolated_stdio_peer(
        command,
        TIMEOUT,
        launch.cancelled().clone(),
    ).await.unwrap();
    let catalog = mcp_client::catalog::list_tools(&peer).await.unwrap()
        .into_iter().map(|tool| (tool.name.to_string(), tool)).collect();
    let server = Server {
        peer,
        catalog,
        approval_names: BTreeMap::from([("echo".into(), "mcp__selected__echo".into())]),
        permits: Semaphore::new(1),
        _scratch: scratch,
        graph_root: None,
    };
    let approvals = VtcodeApproval::new(true);
    let send = |params: Value| {
        dispatch(&server, Some(&approvals), Request::new(Body::from(
            json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": params})
                .to_string(),
        )))
    };
    let input = json!({"path": "/tmp/shiju-approval/note.txt"});
    let request = json!({"name": "echo", "arguments": input, "_meta": {"shijuCallId": "turn-call"}});
    let tool = "mcp__selected__echo";
    assert!(approvals.issue("mcp__static__echo", "turn-call", &input, Box::new(|| true)));
    assert_eq!(send(request.clone()).await.err(), Some(StatusCode::FORBIDDEN));
    assert!(!audit.exists());

    assert!(approvals.issue(tool, "turn-call", &input, Box::new(|| true)));
    for invalid in [
        json!({"name": "echo", "arguments": input}),
        json!({"name": "echo", "arguments": input, "_meta": {"shijuCallId": "another-call"}}),
        json!({"name": "echo", "arguments": {"path": "/tmp/other"}, "_meta": {"shijuCallId": "turn-call"}}),
    ] {
        assert_eq!(send(invalid).await.err(), Some(StatusCode::FORBIDDEN));
        assert!(!audit.exists());
    }
    assert!(send(request.clone()).await.is_ok());
    assert_eq!(send(request).await.err(), Some(StatusCode::FORBIDDEN));
    assert_eq!(std::fs::read_to_string(audit).unwrap(), "executed\n");
}
