//! ACP 对端保管历史：活跃会话直接续聊，恢复失败不得换成空会话。

#![cfg(unix)]

use std::collections::HashMap;
use std::fs;
use std::time::Duration;

use serde_json::Value;
use vw_acp::{AcpAgentConfig, AcpClient, PromptRequest, SessionStrategy};

struct Fixture {
    directory: tempfile::TempDir,
    client: AcpClient,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temporary agent directory");
        let script = directory.path().join("agent.py");
        fs::write(&script, AGENT).expect("write agent");
        let client = AcpClient::new(
            "session-reuse-fixture",
            AcpAgentConfig {
                command: "python3".to_string(),
                args: vec![
                    "-u".to_string(),
                    script.display().to_string(),
                    directory.path().join("calls.jsonl").display().to_string(),
                ],
                env: HashMap::new(),
            },
        );
        Self { directory, client }
    }

    fn calls(&self) -> Vec<Value> {
        fs::read_to_string(self.directory.path().join("calls.jsonl"))
            .expect("read calls")
            .lines()
            .map(|line| serde_json::from_str(line).expect("parse call"))
            .collect()
    }

    fn request(&self, session_id: &str, prompt: &str) -> PromptRequest {
        PromptRequest::new(self.directory.path().to_path_buf(), prompt)
            .with_session_strategy(SessionStrategy::ResumeOrLoad(session_id.to_string()))
    }
}

#[tokio::test]
async fn active_session_preserves_remote_history_without_restoring_or_replaying() {
    let fixture = Fixture::new();
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        let session = fixture.client.create_session(fixture.directory.path()).await?;
        let id = session.session_id;
        fixture.client.set_session_mode(id.clone(), fixture.directory.path(), "agent").await?;
        fixture.client.set_session_model(id.clone(), fixture.directory.path(), "model").await?;
        fixture
            .client
            .set_session_config_option(id.clone(), fixture.directory.path(), "effort", "high")
            .await?;
        let first = fixture.client.run_prompt(fixture.request(&id, "marker"), &mut |_| {}).await?;
        let second =
            fixture.client.run_prompt(fixture.request(&id, "continue"), &mut |_| {}).await?;
        Ok::<_, vw_acp::AcpError>((id, first, second))
    })
    .await;
    fixture.client.close().await.expect("close client");
    let (id, first, second) = result.expect("test timeout").expect("same-session turns");
    assert_eq!(first.session_id, id);
    assert_eq!(second.session_id, id);
    assert_eq!(first.deltas.concat(), "marker");
    assert_eq!(second.deltas.concat(), "marker|continue");
    let calls = fixture.calls();
    let methods: Vec<_> = calls.iter().map(|call| call["method"].as_str().unwrap()).collect();
    assert_eq!(
        methods,
        [
            "initialize",
            "session/new",
            "session/set_mode",
            "session/set_model",
            "session/set_config_option",
            "session/prompt",
            "session/prompt",
        ]
    );
    assert_eq!(calls.last().unwrap()["params"]["prompt"][0]["text"], "continue");
}

#[tokio::test]
async fn failed_restore_never_prompts_another_session_or_creates_a_replacement() {
    let fixture = Fixture::new();
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        let first = fixture.client.create_session(fixture.directory.path()).await?;
        let second = fixture.client.create_session(fixture.directory.path()).await?;
        let switching = fixture
            .client
            .run_prompt(fixture.request(&first.session_id, "continue first"), &mut |_| {})
            .await;
        fixture.client.close().await?;
        let reconnecting = fixture
            .client
            .run_prompt(fixture.request(&second.session_id, "continue second"), &mut |_| {})
            .await;
        Ok::<_, vw_acp::AcpError>((switching, reconnecting))
    })
    .await;
    fixture.client.close().await.expect("close client");
    let (switching, reconnecting) = result.expect("test timeout").expect("create sessions");
    for result in [switching, reconnecting] {
        let error =
            result.expect_err("restore must fail without replacing the session").to_string();
        assert!(error.contains("session/resume"), "{error}");
        assert!(error.contains("session/load"), "{error}");
    }
    let calls = fixture.calls();
    assert_eq!(calls.iter().filter(|call| call["method"] == "session/new").count(), 2);
    assert!(!calls.iter().any(|call| call["method"] == "session/prompt"));
    assert_eq!(calls.iter().filter(|call| call["method"] == "session/resume").count(), 2);
    assert_eq!(calls.iter().filter(|call| call["method"] == "session/load").count(), 2);
}

const AGENT: &str = r#"
import json
import sys

sessions = {}
for line in sys.stdin:
    request = json.loads(line)
    with open(sys.argv[1], "a") as log:
        log.write(json.dumps(request) + "\n")
    method = request.get("method")
    params = request.get("params", {})
    response = {"jsonrpc": "2.0", "id": request["id"]}
    if method == "initialize":
        result = {"protocolVersion": 1, "agentCapabilities": {}, "authMethods": []}
    elif method == "session/new":
        session_id = "session-" + str(len(sessions) + 1)
        sessions[session_id] = []
        result = {"sessionId": session_id}
    elif method == "session/prompt":
        session_id = params["sessionId"]
        sessions[session_id].append(params["prompt"][0]["text"])
        print(json.dumps({"jsonrpc": "2.0", "method": "session/update", "params": {
            "sessionId": session_id, "update": {"sessionUpdate": "agent_message_chunk",
            "content": {"type": "text", "text": "|".join(sessions[session_id])}}
        }}), flush=True)
        result = {"stopReason": "end_turn"}
    elif method in ("session/set_mode", "session/set_model"):
        result = {}
    elif method == "session/set_config_option":
        result = {"configOptions": []}
    else:
        response["error"] = {"code": -32601, "message": "Restore unavailable"}
    if "error" not in response:
        response["result"] = result
    print(json.dumps(response), flush=True)
"#;
