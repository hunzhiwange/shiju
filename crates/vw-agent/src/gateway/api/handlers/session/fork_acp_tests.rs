//! Only the external ACP peer is replaced; forks and prompts use production handlers.
use super::*;

const PHASE: &str = "SHIJU_FORK_ACP_PHASE";

async fn process(root: &FsPath, phase: &str) {
    let name = concat!(module_path!(), "::fork_acp_first_prompt_and_restart_preserve_history");
    let output = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name.split_once("::").unwrap().1, "--nocapture"])
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .env("HOME", root)
        .env("TMPDIR", root)
        .env("VW_MODEL_RECOVERY_TEST_ROOT", root)
        .env("VW_RECOVERY_WINDOW_HTTP", "1")
        .env("VW_ACP_MOUNT_RECOVERY_DB_ROOT", root)
        .env("VIBEWINDOW_CONFIG_DIR", root.join("config"))
        .env(PHASE, phase)
        .current_dir(root)
        .kill_on_drop(true)
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "{phase}: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

async fn send(
    branch: &agent_session::session::Info,
    backend: &str,
    strategy: &str,
    prompt: &str,
    force_new: bool,
) -> String {
    let mut options = json!({"acp_model_source":"cli_local", "acp_history_strategy":strategy,
        "acp_close_client_after_request":true, "full_access":false});
    // Desktop reopening may request rebuilding; the server must preserve the fork anchor.
    if force_new {
        options["acp_force_new_session"] = json!(true);
    }
    let response = super::super::super::super::stream::chat_stream(
        Query(query_for(FsPath::new(&branch.directory))),
        HeaderMap::new(),
        Json(vw_api_types::chat::GatewayChatStreamRequest {
            session_id: Some(branch.id.clone().into()),
            agent: Some(format!("acp-{backend}")),
            acp_agent: Some(backend.into()),
            model: Some("model-recovery/original".into()),
            options: Some(options),
            messages: vec![
                json!({"role":"user","content":"forged-client-history"}),
                json!({"role":"assistant","content":"forged-client-answer"}),
                json!({"role":"user","content":prompt}),
            ],
            ..Default::default()
        }),
    )
    .await
    .unwrap();
    let bytes = tokio::time::timeout(
        Duration::from_secs(60),
        axum::body::to_bytes(
            axum::response::IntoResponse::into_response(response).into_body(),
            2_000_000,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn assert_prefix(text: &str) {
    let mut previous = 0;
    for marker in [
        "retained-user-1",
        "retained-call",
        "retained-result",
        "retained-answer-1",
        "retained-user-2",
        "retained-answer-2",
        "retained-user-3",
        "retained-answer-3",
        "current-fork-request",
    ] {
        let index = text.find(marker).unwrap_or_else(|| panic!("missing {marker}: {text}"));
        assert!(index >= previous, "history order: {text}");
        previous = index;
    }
    assert_eq!(text.matches("current-fork-request").count(), 1);
    for marker in ["selected-boundary", "future-source", "forged-client"] {
        assert!(!text.contains(marker), "boundary leaked {marker}: {text}");
    }
}

#[test]
#[should_panic(expected = "missing retained-user-1")]
fn fork_acp_negative_missing_history_is_detected() {
    assert_prefix("current-fork-request");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fork_acp_first_prompt_and_restart_preserve_history() {
    let root = std::env::current_dir().unwrap();
    match std::env::var(PHASE).as_deref() {
        Err(_) => {
            let root = TempDir::new().unwrap();
            process(root.path(), "create").await;
        }
        Ok("create") => {
            ensure_test_home();
            let script = root.join("peer.py");
            std::fs::write(&script, r#"
import json, sys, uuid
for line in sys.stdin:
    req = json.loads(line)
    if 'id' not in req or 'method' not in req: continue
    with open(sys.argv[1], 'a') as out: out.write(json.dumps(req) + '\n')
    method = req['method']
    result = {}
    if method == 'initialize': result = {'protocolVersion':1,'agentCapabilities':{'loadSession':True},'authMethods':[]}
    elif method == 'session/new': result = {'sessionId': str(uuid.uuid4())}
    if method in ('session/new', 'session/load'):
        result['modes'] = {'currentModeId':sys.argv[2], 'availableModes':[{'id':sys.argv[2], 'name':'Restricted fixture'}]}
    if method == 'session/prompt': result = {'stopReason':'end_turn'}
    response = {'jsonrpc':'2.0','id':req['id'],'result':result}
    if method == 'session/resume':
        response.pop('result')
        response['error'] = {'code':-32601,'message':'fixture supports load only'}
    if method == 'session/prompt' and 'reject-first-prompt' in json.dumps(req):
        response.pop('result')
        response['error'] = {'code':-32603,'message':'fixture rejected first prompt'}
    if method == 'session/prompt' and 'cancel-first-prompt' in json.dumps(req):
        response['result'] = {'stopReason':'cancelled'}
    if method == 'session/prompt' and 'drift-first-prompt' in json.dumps(req):
        response.pop('result')
        response['error'] = {'code':-32603,'message':'acp session changed: fixture drift'}
    if method == 'session/prompt' and 'result' in response and response['result']['stopReason'] != 'cancelled':
        print(json.dumps({'jsonrpc':'2.0','method':'session/update','params':{'sessionId':req['params']['sessionId'],'update':{'sessionUpdate':'agent_message_chunk','content':{'type':'text','text':'fixture-answer'}}}}), flush=True)
    print(json.dumps(response), flush=True)
"#).unwrap();
            let peer = |mode| json!({"command":"python3", "args":[script, root.join("calls.jsonl"), mode]});
            crate::config::update(json!({"acp":{"claude":peer("default"),"codex":peer("read-only")},
                "providers":{"model-recovery":{"api":"https://example.test/v1", "models":{"original":{"status":"active"}}}}})).await.unwrap();
            let repo = repository();
            let source = create_session_in(repo.path(), "ACP history source").await;
            let (assistant, _) =
                persist_turn(&source.id, "retained-user-1", "retained-answer-1").await;
            persist_turn(&source.id, "retained-user-2", "retained-answer-2").await;
            persist_turn(&source.id, "retained-user-3", "retained-answer-3").await;
            let (_, boundary) =
                persist_turn(&source.id, "selected-boundary", "selected-boundary-answer").await;
            persist_turn(&source.id, "future-source", "future-source-answer").await;
            let tool: Part = serde_json::from_value(json!({"type":"tool", "id":"part-acp-fixture", "sessionID":source.id,
                "messageID":assistant,"callID":"retained-call","tool":"read","state":{"status":"completed",
                "input":{"path":"tracked.txt"},"output":"retained-result","title":"read","metadata":{},"time":{"start":1,"end":2}}})).unwrap();
            message::update_part(&tool).await.unwrap();
            let original = history(&source).await;
            let mut branches = Vec::new();
            for (source_agent, backend, strategy, worktree) in [
                ("acp-claude", "claude", "discard", false),
                ("main", "claude", "recent", true),
                ("acp-claude", "codex", "summary", false),
                ("main", "claude", "reject", false),
                ("main", "claude", "cancel", false),
                ("main", "claude", "drift", false),
            ] {
                let binding = agent_binding::AgentBinding::from_definition(
                    source_agent.into(),
                    &crate::agent::get(source_agent).await.unwrap(),
                );
                agent_binding::save_scoped(&source.project_id, &source.id, &binding).await.unwrap();
                let target = if worktree { json!({"type":"worktree"}) } else { Value::Null };
                let (status, body) =
                    fork(&source, json!({"beforeTurnId":boundary,"target":target})).await;
                assert_eq!(status, StatusCode::OK, "{body}");
                let branch: agent_session::session::Info = serde_json::from_value(body).unwrap();
                branches.push(json!({"info":branch,"backend":backend,"strategy":strategy}));
            }
            std::fs::write(root.join("branches.json"), serde_json::to_vec(&branches).unwrap())
                .unwrap();
            process(&root, "first").await;
            process(&root, "second").await;
            assert_eq!(
                serde_json::to_value(history(&source).await).unwrap(),
                serde_json::to_value(original).unwrap()
            );
            for item in branches {
                let branch: agent_session::session::Info =
                    serde_json::from_value(item["info"].clone()).unwrap();
                if branch.directory != source.directory {
                    worktree_cleanup(&source, branch.directory).await;
                }
            }
        }
        Ok(phase @ ("first" | "second")) => {
            let branches: Vec<Value> =
                serde_json::from_slice(&std::fs::read(root.join("branches.json")).unwrap())
                    .unwrap();
            let mut remote_ids = serde_json::Map::new();
            for item in branches {
                let branch: agent_session::session::Info =
                    serde_json::from_value(item["info"].clone()).unwrap();
                if matches!(item["strategy"].as_str(), Some("reject" | "cancel" | "drift")) {
                    if phase == "first" {
                        let events = send(
                            &branch,
                            "claude",
                            "discard",
                            &format!("{}-first-prompt", item["strategy"].as_str().unwrap()),
                            false,
                        )
                        .await;
                        assert!(events.contains("chat.error"), "{events}");
                    } else {
                        let before = std::fs::read_to_string(root.join("calls.jsonl")).unwrap();
                        let events =
                            send(&branch, "claude", "discard", "retry-failed-prompt", false).await;
                        assert!(events.contains("delivery is uncertain"), "{events}");
                        assert_eq!(
                            std::fs::read_to_string(root.join("calls.jsonl")).unwrap(),
                            before
                        );
                    }
                    continue;
                }
                let before = std::fs::read_to_string(root.join("calls.jsonl")).unwrap_or_default();
                let prompt =
                    if phase == "first" { "current-fork-request" } else { "second-fork-request" };
                let events = send(
                    &branch,
                    item["backend"].as_str().unwrap(),
                    item["strategy"].as_str().unwrap(),
                    prompt,
                    phase == "second",
                )
                .await;
                assert!(events.contains("chat.done") && !events.contains("chat.error"), "{events}");
                assert!(!events.contains("chat.tool"), "historical tools executed: {events}");
                let calls = std::fs::read_to_string(root.join("calls.jsonl")).unwrap();
                let requests: Vec<Value> = calls[before.len()..]
                    .lines()
                    .map(|line| serde_json::from_str(line).unwrap())
                    .collect();
                let prompts: Vec<_> =
                    requests.iter().filter(|r| r["method"] == "session/prompt").collect();
                assert_eq!(prompts.len(), 1);
                let text = prompts[0]["params"]["prompt"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|b| b["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                if phase == "first" {
                    assert_prefix(&text);
                    remote_ids.insert(branch.id.clone(), prompts[0]["params"]["sessionId"].clone());
                } else {
                    let ids: Value = serde_json::from_slice(
                        &std::fs::read(root.join("remote-ids.json")).unwrap(),
                    )
                    .unwrap();
                    assert_eq!(prompts[0]["params"]["sessionId"], ids[&branch.id]);
                    assert!(!text.contains("retained-user-1"), "repeated history: {text}");
                    assert_eq!(text.matches("second-fork-request").count(), 1);
                    assert!(requests.iter().any(|r| r["method"] == "session/load"));
                    assert!(!requests.iter().any(|r| r["method"] == "session/new"));
                }
            }
            if phase == "first" {
                let unique: std::collections::HashSet<_> =
                    remote_ids.values().map(|id| id.as_str().unwrap()).collect();
                assert_eq!(unique.len(), remote_ids.len());
                std::fs::write(
                    root.join("remote-ids.json"),
                    serde_json::to_vec(&remote_ids).unwrap(),
                )
                .unwrap();
            }
        }
        Ok(other) => panic!("unknown phase {other}"),
    }
}
