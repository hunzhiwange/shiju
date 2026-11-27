use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use vw_api_transport::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, TenantScope};

async fn request(
    tenant: &TenantScope,
    method: &str,
    path: &str,
    body: Value,
) -> (StatusCode, Value) {
    let response = crate::gateway::router::build_router(Vec::new())
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .header(PLATFORM_ID_HEADER, tenant.platform_id().as_ref())
                .header(COMPANY_ID_HEADER, tenant.company_id().as_ref())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 131_072).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn cli_workbench_acp_lists_sessions_for_registered_project() {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let tenant = TenantScope::new("workbench-fixture", "list").unwrap();
    let root = tempfile::tempdir().unwrap();
    let (project, _) = crate::project::from_directory(
        &tenant,
        root.path().canonicalize().unwrap().to_str().unwrap(),
    )
    .await
    .unwrap();
    let path = format!("/v1/projects/{}/cli-workbench/sessions", project.id);
    let (status, body) = request(&tenant, "GET", &path, Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!([]));

    let (status, body) = request(
        &tenant,
        "POST",
        &path,
        json!({"target": {"kind": "acp", "agent": "acp-codex"}}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["code"], "invalid_request");

    for model in ["", "auto", "provider/", "/model", "provider/model "] {
        let (status, body) = request(
            &tenant,
            "POST",
            &path,
            json!({"target": {"kind": "acp", "agent": "acp-codex"},
                "acp": {"model_source": "session_model", "model": model}}),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(body["message"], "model must be a provider/model reference");
    }
    let (status, body) = request(
        &tenant,
        "POST",
        &path,
        json!({"target": {"kind": "acp", "agent": "acp-codex"},
            "acp": {"model_source": "cli_local", "model": "provider/model"}}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["message"], "model requires the session_model source");

    let directory = gateway_project::encode_path_segment(&project.worktree);
    let path = format!("/v1/cli-workbench/projects?directory={directory}");
    let (status, body) = request(&tenant, "GET", &path, Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["preferred_project"], json!(project.id.as_str()));
    let entry = body["projects"].as_array().unwrap().iter()
        .find(|entry| entry["id"] == project.id.as_str()).unwrap();
    assert_eq!(entry["sessions"], json!([]));
    assert!(entry.get("git").is_none());

    let nested = root.path().join("nested directory");
    std::fs::create_dir(&nested).unwrap();
    let directory = gateway_project::encode_path_segment(nested.to_str().unwrap());
    let path = format!("/v1/cli-workbench/projects?directory={directory}");
    let (status, body) = request(&tenant, "GET", &path, Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["preferred_project"], json!(project.id.as_str()));
}

#[tokio::test]
async fn cli_workbench_catalog_returns_the_full_gateway_project_list() {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let tenant = TenantScope::new("workbench-fixture", "catalog").unwrap();
    let root = tempfile::tempdir().unwrap();
    let mut ids = Vec::new();
    for index in 0..12 {
        let directory = root.path().join(format!("project-{index}"));
        std::fs::create_dir(&directory).unwrap();
        let (project, _) = crate::project::from_directory(&tenant, directory.canonicalize().unwrap())
            .await.unwrap();
        ids.push(project.id);
    }
    ids.sort();

    for (route, field) in [
        ("/v1/cli-workbench/projects?catalog_only=true", "projects"),
        ("/v1/cli-workbench/projects", "projects"),
        ("/v1/projects?limit=100", "items"),
    ] {
        let (status, body) = request(&tenant, "GET", route, Value::Null).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let mut returned: Vec<_> = body[field].as_array().unwrap().iter()
            .map(|project| project["id"].as_str().unwrap()).collect();
        returned.sort();
        assert_eq!(returned, ids.iter().map(String::as_str).collect::<Vec<_>>());
    }
}

#[tokio::test]
async fn cli_workbench_resolves_unregistered_directory_once() {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let tenant = TenantScope::new("workbench-fixture", "resolve").unwrap();
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().canonicalize().unwrap();
    let nested = directory.join("nested");
    std::fs::create_dir(&nested).unwrap();
    assert!(agent_project::list(&tenant).await.unwrap().is_empty());

    let route = "/v1/cli-workbench/projects/resolve";
    let body = json!({"directory": directory});
    let ((status, project), concurrent) = tokio::join!(
        request(&tenant, "POST", route, body.clone()),
        request(&tenant, "POST", route, body),
    );
    assert_eq!(status, StatusCode::OK, "{project}");
    assert_eq!(concurrent, (StatusCode::OK, project.clone()));
    let id = project.as_str().unwrap();
    let (status, resolved) = request(&tenant, "POST", route, json!({"directory": nested})).await;
    assert_eq!(status, StatusCode::OK, "{resolved}");
    assert_eq!(resolved, project);
    let missing = directory.join("missing");
    let (status, rejected) = request(&tenant, "POST", route, json!({"directory": missing})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(rejected["code"], "invalid_request");

    let projects = agent_project::list(&tenant).await.unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id.as_str(), id);
    assert_eq!(projects[0].worktree, directory.to_str().unwrap());
    let sessions = format!("/v1/projects/{id}/cli-workbench/sessions");
    assert_eq!(request(&tenant, "GET", &sessions, Value::Null).await, (StatusCode::OK, json!([])));
}

#[cfg(unix)]
const ACP_PEER: &str = r#"
import json, os, sys, urllib.request, uuid

def send(frame):
    print(json.dumps(dict(jsonrpc='2.0', **frame)), flush=True)

def complete(prompt, answer):
    send(dict(method='session/update', params=dict(sessionId=prompt['params']['sessionId'],
        update=dict(sessionUpdate='agent_message_chunk', content=dict(type='text', text=answer)))))
    send(dict(id=prompt['id'], result=dict(stopReason='end_turn')))

previous, pending = None, None
for line in sys.stdin:
    req = json.loads(line)
    method = req.get('method')
    if method is None and req.get('id') == 'permission-2':
        assert req['result']['outcome']['optionId'] == 'allow', req
        complete(pending, 'second result')
        pending = None
        continue
    if method == 'session/cancel':
        if pending:
            send(dict(id=pending['id'], result=dict(stopReason='cancelled')))
            pending = None
        continue
    if 'id' not in req or method is None:
        continue
    result = {}
    if method == 'initialize':
        result = dict(protocolVersion=1, agentCapabilities=dict(loadSession=True), authMethods=[])
    elif method == 'session/new':
        result = dict(sessionId=str(uuid.uuid4()))
    if method in ('session/new', 'session/load'):
        result['modes'] = dict(currentModeId='default',
            availableModes=[dict(id='default', name='Restricted fixture')])
    if method == 'session/prompt':
        marker = req['params']['prompt'][-1]['text']
        with open(sys.argv[1], 'a') as out:
            out.write(marker + '\n')
        if marker == 'first turn':
            context = json.loads(os.environ['SHIJU_WORKBENCH_CONTEXT'])
            assert 'VIBEWINDOW_SERVER_PASSWORD' not in os.environ
            url = f"http://127.0.0.1:{context['port']}/v1/projects/{context['project_id']}/cli-workbench/sessions"
            query = urllib.request.Request(url, headers={'Authorization': 'Bearer ' + context['token']})
            opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
            with opener.open(query, timeout=2) as response:
                assert any(entry['state'] == 'running' for entry in json.load(response))
            previous = req['id']
            complete(req, 'first result')
        elif marker == 'second turn':
            assert previous is not None, 'workbench did not keep the original ACP process'
            # Replay the older RPC response while the second prompt owns the workbench turn.
            send(dict(id=previous, result=dict(stopReason='end_turn')))
            pending = req
            send(dict(id='permission-2', method='session/request_permission',
                params=dict(sessionId=req['params']['sessionId'],
                    toolCall=dict(toolCallId='fixture-call', kind='edit', title='Fixture edit'),
                    options=[dict(optionId='allow', kind='allow_once', name='Allow'),
                             dict(optionId='reject', kind='reject_once', name='Reject')])))
        continue
    send(dict(id=req['id'], result=result))
"#;

#[cfg(unix)]
async fn wait_for_state(
    tenant: &TenantScope,
    path: &str,
    accept: impl Fn(&Value) -> bool,
) -> Value {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut revision = 0;
        loop {
            let events = format!("{path}/events?after_revision={revision}");
            let (status, message) = request(tenant, "GET", &events, Value::Null).await;
            assert_eq!(status, StatusCode::OK, "{message}");
            if accept(&message) {
                return message;
            }
            assert!(
                message["state"] != "completed" && message["state"] != "failed",
                "unexpected terminal state {message}"
            );
            revision = message["revision"].as_u64().unwrap();
        }
    })
    .await
    .expect("workbench subscription timed out")
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_workbench_acp_routes_isolate_projects_and_correlate_approved_turns() {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let tenant = TenantScope::new("workbench-fixture", "turns").unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path().canonicalize().unwrap();
    let foreign = tempfile::tempdir().unwrap();
    let foreign_root = foreign.path().canonicalize().unwrap();
    let (project, _) = crate::project::from_directory(&tenant, &root).await.unwrap();
    let (other, _) = crate::project::from_directory(&tenant, &foreign_root).await.unwrap();
    let script = root.join("peer.py");
    let calls = root.join("prompts.txt");
    std::fs::write(&script, ACP_PEER).unwrap();
    crate::config::update_global_for_tenant(
        &tenant,
        json!({"agents": {"acp-claude": {"defaults": {"full_access": false}}}}),
    )
    .await
    .unwrap();
    crate::config::update_project_for_tenant(
        &tenant,
        &root,
        json!({
            "acp": {"claude": {
                "command": "python3",
                "args": [script, calls],
                "env": {"VIBEWINDOW_SERVER_PASSWORD": "fixture-host-password"}
            }}
        }),
    )
    .await
    .unwrap();

    let sessions = format!("/v1/projects/{}/cli-workbench/sessions", project.id);
    let foreign_sessions = format!("/v1/projects/{}/cli-workbench/sessions", other.id);
    let target = json!({"kind": "acp", "agent": "acp-claude"});
    let acp = json!({"model_source": "cli_local"});
    let (status, _) = request(
        &tenant,
        "POST",
        &sessions,
        json!({"target": target, "directory": foreign_root, "acp": acp}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    for target in [json!({"kind": "shell"}), json!({"kind": "native_cli", "app_id": "claude"})] {
        let (status, body) = request(&tenant, "POST", &sessions, json!({"target": target})).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["code"], "unavailable");
    }
    let (status, session) =
        request(&tenant, "POST", &sessions, json!({"target": target, "acp": acp})).await;
    assert_eq!(status, StatusCode::CREATED, "{session}");
    let id = session["id"].as_str().unwrap();
    let route = format!("{sessions}/{id}");
    let send = format!("{route}/messages");
    let first = json!({"request_id": "first", "text": "first turn"});
    let second = json!({"request_id": "second", "text": "second turn"});
    let (status, _) =
        request(&tenant, "POST", &format!("{foreign_sessions}/{id}/messages"), first.clone()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = request(
        &TenantScope::new("workbench-fixture", "foreign").unwrap(),
        "POST",
        &send,
        first.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, accepted) = request(&tenant, "POST", &send, first.clone()).await;
    assert_eq!(status, StatusCode::OK, "{accepted}");
    assert_eq!(accepted["state"], "accepted");
    assert_eq!(accepted["id"]["target_session_id"], id);
    assert_eq!(accepted["id"]["sender_user_id"], tenant.user_id().as_ref());
    let first_path = format!("{send}/first");
    let first_result =
        wait_for_state(&tenant, &first_path, |value| value["state"] == "completed").await;
    assert_eq!(first_result["turn_id"], accepted["turn_id"]);
    assert_eq!(first_result["output"], "first result");
    let (status, duplicate) = request(&tenant, "POST", &send, first).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(duplicate, first_result);
    let (status, conflict) =
        request(&tenant, "POST", &send, json!({"request_id": "first", "text": "changed"})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(conflict["code"], "request_conflict");

    let (status, accepted) = request(&tenant, "POST", &send, second.clone()).await;
    assert_eq!(status, StatusCode::OK, "{accepted}");
    let second_path = format!("{send}/second");
    let pending =
        wait_for_state(&tenant, &second_path, |value| value["state"] == "waiting_for_approval")
            .await;
    assert_ne!(pending["turn_id"], first_result["turn_id"]);
    assert_eq!(pending["turn_id"], accepted["turn_id"]);
    assert_eq!(pending["output"], "");
    let (status, duplicate) = request(&tenant, "POST", &send, second).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(duplicate, pending);
    let (status, rejected) =
        request(&tenant, "POST", &send, json!({"request_id": "third", "text": "third turn"})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(rejected["code"], "awaiting_approval");
    let (status, listed) = request(&tenant, "GET", &sessions, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed[0]["recent_request_ids"], json!(["first", "second"]));
    let (_, foreign) = request(&tenant, "GET", &foreign_sessions, Value::Null).await;
    assert_eq!(foreign, json!([]));
    let permission = pending["approval_request_id"].as_str().unwrap();
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("directory", root.to_str().unwrap())
        .finish();
    let permission_path = format!("/v1/permission/{permission}/reply?{query}");
    let (status, reply) =
        request(&tenant, "POST", &permission_path, json!({"reply": "once"})).await;
    assert_eq!(status, StatusCode::OK, "{reply}");

    let second_result =
        wait_for_state(&tenant, &second_path, |value| value["state"] == "completed").await;
    assert_eq!(second_result["turn_id"], pending["turn_id"]);
    assert_ne!(second_result["assistant_message_id"], first_result["assistant_message_id"]);
    assert_eq!(second_result["output"], "second result");
    assert_eq!(second_result["approval_history"], json!([permission]));
    assert_eq!(request(&tenant, "GET", &first_path, Value::Null).await.1, first_result);
    let (status, projects) = request(&tenant, "GET", "/v1/cli-workbench/projects", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    let entry = projects["projects"].as_array().unwrap().iter()
        .find(|entry| entry["id"] == project.id.as_str()).unwrap();
    assert_eq!(entry["sessions"][0]["recent_request_ids"], json!(["first", "second"]));
    assert_eq!(std::fs::read_to_string(calls).unwrap(), "first turn\nsecond turn\n");

    let (status, _) = request(&tenant, "POST", &format!("{route}/stop"), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    let (status, closed) =
        request(&tenant, "POST", &send, json!({"request_id": "third", "text": "third turn"})).await;
    assert_eq!(status, StatusCode::GONE);
    assert_eq!(closed["code"], "closed");
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let (_, list) = request(&tenant, "GET", &sessions, Value::Null).await;
            if list[0]["state"] == "exited" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("ACP process did not exit");
}
