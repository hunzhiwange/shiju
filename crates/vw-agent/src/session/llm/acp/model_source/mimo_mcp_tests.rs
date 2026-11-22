use super::*;
use agent_client_protocol::McpServerStdio;
use axum::body::Body;
use model_transport::RelayIdentity;
use tower::ServiceExt;

const SERVER: &str = r#"
if [ -n "${DISCOVERY_AUDIT:-}" ]; then
    printf 'started\n' >> "$DISCOVERY_AUDIT"
fi
while IFS= read -r line; do
    case "$line" in
        *'"id":'*) id=${line#*\"id\":}; id=${id%%,*}; id=${id%%\}*};;
        *) continue;;
    esac
    case "$line" in
        *'"method":"initialize"'*)
            result='{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"fixture","version":"1"}}';;
        *'"method":"tools/list"'*)
            if [ -n "${DISCOVERY_AUDIT:-}" ]; then
                printf 'listed\n' >> "$DISCOVERY_AUDIT"
            fi
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

async fn post(
    projection: &Projection,
    route: &str,
    token: &str,
    method: &str,
    params: Value,
) -> (StatusCode, Value) {
    let request = axum::http::Request::builder()
        .method("POST")
        .uri(route)
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
    const ROUTE: &str = "/shiju-mimo-mcp/fixture";
    assert_eq!(
        post(&projection, ROUTE, launch.bearer_token(), "tools/list", json!({})).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        post(&projection, ROUTE, binding("other:mimo-code").bearer_token(), "tools/list", json!({}))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let (status, listed) = post(&projection, ROUTE, projection.token(), "tools/list", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed["result"]["tools"].as_array().unwrap().len(), 1);
    assert_eq!(listed["result"]["tools"][0]["name"], "echo");
    let (_, denied) =
        post(&projection, ROUTE, projection.token(), "tools/call", json!({"name":"denied"})).await;
    assert_eq!(denied["error"]["code"], -32602);
    let (_, called) =
        post(&projection, ROUTE, projection.token(), "tools/call", json!({"name":"echo","arguments":{}}))
            .await;
    assert_eq!(called["result"]["content"][0]["text"], "fixture result");

    launch.revoke();
    assert_eq!(
        post(&projection, ROUTE, projection.token(), "tools/list", json!({})).await.0,
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

#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
#[tokio::test]
async fn acp_platform_approval_only_launches_the_reviewed_linux_projection() {
    assert_reviewed_linux_projection(LinuxMcpOwner::VtCode, "vtcode", None).await;
}

#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
#[tokio::test]
async fn acp_platform_approval_iflow_v4_binds_host_selected_source() {
    assert_reviewed_linux_projection(LinuxMcpOwner::IFlow(IFlowMcpSource::Project), "iflow", Some("project")).await;
    assert_reviewed_linux_projection(LinuxMcpOwner::IFlow(IFlowMcpSource::Mounted), "iflow", Some("mounted")).await;
}

#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
#[tokio::test]
async fn acp_platform_approval_glm_v5_binds_both_reviewed_sources() {
    for source in [LinuxMcpSource::Project, LinuxMcpSource::Mounted] {
        assert_reviewed_linux_projection(LinuxMcpOwner::Glm(source), "glm-acp-agent", None).await;
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
#[tokio::test]
async fn acp_platform_approval_glm_v5_rejects_overlong_and_duplicate_catalog() {
    use sha2::{Digest, Sha256};

    let executable = FsPath::new("/bin/bash").canonicalize().unwrap();
    let sha256 = Sha256::digest(std::fs::read(&executable).unwrap()).into();
    let long = "x".repeat(50); // mcp__selected__ + 50 exceeds GLM's 64-byte limit.
    for (body, name, error) in [
        (SERVER.replace(r#""name":"echo""#, &format!(r#""name":"{long}""#)), long.as_str(), "invalid managed GLM MCP tool identity"),
        (SERVER.replace(r#""name":"denied""#, r#""name":"echo""#), "echo", "Invalid or duplicate Linux ACP tool"),
    ] {
        let args = vec!["-c".into(), body];
        let mut policy = acp_config::McpServerManagement::default_for_server("selected");
        policy.enabled = true;
        policy.allowed_tools = vec![name.into()];
        let selected = AcpMcpServer::Stdio(McpServerStdio::new("selected", &executable).args(args.clone()));
        let result = prepare_linux_stdio_with(
            &[selected],
            ReviewedLinuxStdio {
                owner: LinuxMcpOwner::Glm(LinuxMcpSource::Mounted),
                executable: &executable,
                sha256,
                args: &args,
                policy: &policy,
            },
            binding("catalog:glm-acp-agent"),
            |staged, args| {
                let mut command = Command::new(staged);
                command.args(args).env_clear();
                Ok(command)
            },
        ).await;
        let Err(failure) = result else { panic!("invalid catalog was advertised") };
        assert_eq!(failure.to_string(), error);
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
async fn assert_reviewed_linux_projection(owner: LinuxMcpOwner, backend: &str, source: Option<&str>) {
    use sha2::{Digest, Sha256};

    const ROUTE: &str = "/shiju-vtcode-mcp/selected";
    let dir = tempfile::tempdir().unwrap();
    let discovery = dir.path().join("discovery");
    let execution = dir.path().join("execution");
    let executable = FsPath::new("/bin/bash").canonicalize().unwrap();
    let digest: [u8; 32] = Sha256::digest(std::fs::read(&executable).unwrap()).into();
    let args = vec!["-c".into(), SERVER.into()];
    let mut policy = acp_config::McpServerManagement::default_for_server("selected");
    policy.enabled = true;
    policy.allowed_tools = vec!["echo".into()];
    let pin = |sha256| ReviewedLinuxStdio {
        owner,
        executable: &executable, sha256, args: &args, policy: &policy,
    };
    let launch = binding(&format!("first:{backend}"));
    let mut declaration = McpServerStdio::new("selected", &executable).args(args.clone());
    declaration.meta = Some(Map::from_iter([("shiju.iflow.mcp-v4".into(), json!("forged"))]));
    let selected = AcpMcpServer::Stdio(declaration);
    let implicit = AcpMcpServer::Stdio(McpServerStdio::new("project", &executable).args(args.clone()));
    let altered = AcpMcpServer::Stdio(McpServerStdio::new("selected", &executable).args(vec!["-s".into()]));
    let must_not_launch = |_: &FsPath, _: &[String]| -> anyhow::Result<Command> {
        panic!("unapproved discovery reached process preparation")
    };
    for rejected in [vec![], vec![implicit], vec![altered], vec![selected.clone(), selected.clone()]] {
        assert!(prepare_linux_stdio_with(&rejected, pin(digest), launch.clone(), must_not_launch).await.is_err());
    }
    assert!(prepare_linux_stdio_with(
        std::slice::from_ref(&selected), pin([0; 32]), launch.clone(), must_not_launch,
    ).await.is_err());
    assert!(prepare_linux_stdio_with(
        std::slice::from_ref(&selected), pin(digest), binding("first:other"), must_not_launch,
    ).await.is_err());
    let revoked = binding(&format!("revoked:{backend}"));
    revoked.revoke();
    assert!(prepare_linux_stdio_with(
        std::slice::from_ref(&selected), pin(digest), revoked, must_not_launch,
    ).await.is_err());
    assert!(!discovery.exists());

    let projection = prepare_linux_stdio_with(&[selected], pin(digest), launch.clone(), |staged, args| {
        // Only the test replaces the OS sandbox command. Catalog discovery and
        // the mounted peer execute the real stdio protocol and router.
        assert_ne!(staged, executable.as_path());
        let staged_digest: [u8; 32] = Sha256::digest(std::fs::read(staged).unwrap()).into();
        assert_eq!(staged_digest, digest);
        let mut command = Command::new(staged);
        command.args(args).current_dir(staged.parent().unwrap()).env_clear()
            .env("DISCOVERY_AUDIT", &discovery)
            .env("APPROVAL_AUDIT", &execution);
        Ok(command)
    }).await.unwrap();

    assert_eq!(projection.linux_owner, Some(owner));
    let token = projection.token();
    assert!(token != launch.bearer_token());
    assert_eq!(post(&projection, ROUTE, launch.bearer_token(), "tools/list", json!({})).await.0,
        StatusCode::UNAUTHORIZED);
    let foreign = binding(&format!("other:{backend}"));
    assert_eq!(post(&projection, ROUTE, foreign.bearer_token(), "tools/list", json!({})).await.0,
        StatusCode::UNAUTHORIZED);
    let (status, catalog) = post(&projection, ROUTE, token, "tools/list", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let tools = catalog["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "echo");
    assert_eq!(std::fs::read_to_string(&discovery).unwrap(), "started\nlisted\n");

    let input = json!({"path": "note.txt", "values": [null, true, 1, "1", {"enabled": false}]});
    let call = json!({"name": "echo", "arguments": input, "_meta": {"shijuCallId": "owned-call"}});
    assert_eq!(post(&projection, ROUTE, token, "tools/call", call.clone()).await.0, StatusCode::FORBIDDEN);
    assert!(!execution.exists());
    if source.is_some() || matches!(owner, LinuxMcpOwner::Glm(_)) {
        for endpoint in ["https://127.0.0.1:1", "http://192.0.2.1:1", "http://127.0.0.1:1/", "http://127.0.0.1:80"] {
            assert!(projection.approval_config(endpoint).is_err());
        }
    }
    if matches!(owner, LinuxMcpOwner::Glm(_)) {
        assert!(projection.approval_config("http://[::1]:1").is_err());
    }
    let (servers, issue) = projection.approval_config("http://127.0.0.1:1").unwrap();
    let [AcpMcpServer::Http(server)] = servers.as_slice() else { panic!("expected one HTTP projection") };
    assert_eq!(server.url, "http://127.0.0.1:1/shiju-vtcode-mcp/selected");
    assert_eq!(server.meta.as_ref().and_then(|meta| meta.get("shiju.iflow.mcp-v4")).and_then(Value::as_str), source);
    assert!(!issue("mcp__project__echo", "owned-call", &input, Box::new(|| true)));
    assert!(!issue("image_analysis", "owned-call", &input, Box::new(|| true)));
    assert!(issue("mcp__selected__echo", "owned-call", &input, Box::new(|| true)));
    let mut wrong_id = call.clone();
    wrong_id["_meta"]["shijuCallId"] = json!("other-call");
    let mut wrong_args = call.clone();
    wrong_args["arguments"]["values"][2] = json!("1");
    for invalid in [wrong_id, wrong_args] {
        assert_eq!(post(&projection, ROUTE, token, "tools/call", invalid).await.0, StatusCode::FORBIDDEN);
    }
    assert!(!execution.exists());
    assert_eq!(post(&projection, ROUTE, token, "tools/call", call.clone()).await.0, StatusCode::OK);
    assert_eq!(post(&projection, ROUTE, token, "tools/call", call.clone()).await.0, StatusCode::FORBIDDEN);
    launch.revoke();
    assert!(!issue("mcp__selected__echo", "owned-call", &input, Box::new(|| true)));
    assert_eq!(post(&projection, ROUTE, token, "tools/call", call).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(std::fs::read_to_string(execution).unwrap(), "executed\n");
}

#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
#[tokio::test]
#[ignore = "requires non-root GNU Linux with Bubblewrap user namespaces and native dash"]
async fn acp_platform_approval_linux_sandboxed_stdio_dispatch() {
    use sha2::{Digest, Sha256};

    const ROUTE: &str = "/shiju-vtcode-mcp/selected";
    let executable = FsPath::new("/bin/dash").canonicalize().unwrap();
    // The sandbox must not see this host file, even during catalog discovery.
    let host = tempfile::NamedTempFile::new().unwrap();
    let fixture = format!(
        "[ ! -e \"$1\" ] && [ \"$PWD\" = /tmp/work ] || exit 4\n{SERVER}"
    );
    let args = vec!["-c".into(), fixture, "relay-fixture".into(), host.path().to_str().unwrap().into()];
    let mut policy = acp_config::McpServerManagement::default_for_server("selected");
    policy.enabled = true;
    policy.allowed_tools = vec!["echo".into()];
    let launch = binding("sandboxed:vtcode");
    let selection = AcpMcpServer::Stdio(McpServerStdio::new("selected", &executable).args(args.clone()));
    let projection = prepare_linux_stdio(
        &[selection],
        ReviewedLinuxStdio {
            owner: LinuxMcpOwner::VtCode,
            executable: &executable,
            sha256: Sha256::digest(std::fs::read(&executable).unwrap()).into(),
            args: &args,
            policy: &policy,
        },
        launch.clone(),
    ).await.unwrap();
    let token = projection.token();
    let input = json!({});
    let call = json!({"name": "echo", "arguments": input, "_meta": {"shijuCallId": "sandbox-call"}});
    assert_eq!(post(&projection, ROUTE, token, "tools/call", call.clone()).await.0, StatusCode::FORBIDDEN);
    let (_, issue) = projection.approval_config("http://127.0.0.1:1").unwrap();
    assert!(issue("mcp__selected__echo", "sandbox-call", &input, Box::new(|| true)));
    let (status, result) = post(&projection, ROUTE, token, "tools/call", call.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["result"]["content"][0]["text"], "fixture result");
    launch.revoke();
    assert_eq!(post(&projection, ROUTE, token, "tools/call", call).await.0, StatusCode::UNAUTHORIZED);
}

#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
#[path = "mimo_mcp_mount_review_tests.rs"]
mod mount_review;
