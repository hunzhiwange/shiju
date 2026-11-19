use super::*;
use agent_client_protocol::McpServerStdio;
use axum::body::Body;
use model_transport::RelayIdentity;
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
