use std::collections::HashMap;
use serde_json::json;
use agent_auth as auth;
use model_types as provider;
use message_types as message;
use super::*;
fn test_model(provider_id: &str) -> provider::Model {
    serde_json::from_value(json!({
        "id": "gpt-5-mini",
        "providerID": provider_id,
        "api": {
            "id": "gpt-5-mini",
            "url": "http://localhost",
            "adapter": "openai-compatible"
        },
        "name": "gpt-5-mini",
        "family": null,
        "capabilities": {
            "temperature": true,
            "reasoning": true,
            "attachment": false,
            "toolcall": true,
            "input": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "output": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "interleaved": false
        },
        "cost": {
            "input": 0.0,
            "output": 0.0,
            "cache": {
                "read": 0.0,
                "write": 0.0
            },
            "experimental_over_200k": null
        },
        "limit": {
            "context": 8192,
            "input": null,
            "output": 4096
        },
        "status": "active",
        "options": {},
        "headers": {},
        "release_date": "2026-01-01",
        "variants": {}
    }))
    .expect("test model should deserialize")
}

fn test_provider(key: Option<&str>) -> provider::Info {
    provider::Info {
        id: "github-copilot".to_string(),
        name: "GitHub Copilot".to_string(),
        source: provider::ProviderSource::Api,
        env: Vec::new(),
        key: key.map(ToString::to_string),
        options: HashMap::new(),
        models: HashMap::new(),
    }
}

#[test]
fn close_client_after_request_option_defaults_to_false() {
    assert!(!parse_close_client_after_request(&json!({})).expect("missing option should parse"));
}

#[test]
fn close_client_after_request_option_treats_non_object_as_missing() {
    assert!(!parse_close_client_after_request(&json!(null)).expect("null should use the default"));
    assert!(!parse_close_client_after_request(&json!([])).expect("array should use the default"));
}

#[test]
fn close_client_after_request_option_accepts_boolean_only() {
    assert!(
        parse_close_client_after_request(&json!({"acp_close_client_after_request": true}))
            .expect("boolean option should parse")
    );
    assert!(
        parse_close_client_after_request(&json!({"acp_close_client_after_request": "true"}))
            .is_err()
    );
}

#[test]
fn close_client_after_request_option_accepts_explicit_false() {
    assert!(
        !parse_close_client_after_request(&json!({"acp_close_client_after_request": false}))
            .expect("false should remain disabled")
    );
}

#[test]
fn close_client_after_request_option_reports_non_boolean_error() {
    let error = parse_close_client_after_request(&json!({
        "acp_close_client_after_request": 1
    }))
    .expect_err("numeric option should be rejected");

    match error {
        Error::Api(message::AssistantError::Unknown {
            message,
        }) => assert_eq!(
            message,
            "acp option error: acp_close_client_after_request must be a boolean"
        ),
        other => panic!("expected ACP option error, got {other:?}"),
    }
}

#[test]
fn close_client_after_request_option_rejects_explicit_null() {
    let error = parse_close_client_after_request(&json!({
        "acp_close_client_after_request": null
    }))
    .expect_err("explicit null must not silently use the default");

    match error {
        Error::Api(message::AssistantError::Unknown {
            message,
        }) => assert_eq!(
            message,
            "acp option error: acp_close_client_after_request must be a boolean"
        ),
        other => panic!("expected ACP option error, got {other:?}"),
    }
}

#[test]
fn agent_model_inheritance_validates_credentials_and_model_snapshot() {
    let mut model = test_model("github-copilot");
    let info = test_provider(Some("fixture-provider-key"));
    assert!(super::validate_session_model(&model, &info, None).is_ok());
    assert!(
        super::validate_session_model(&model, &test_provider(Some(auth::OAUTH_DUMMY_KEY)), None)
            .is_err()
    );
    let oauth = auth::Info::Oauth(auth::OauthInfo {
        access: String::new(),
        refresh: "fixture-refresh".into(),
        expires: 0,
        account_id: None,
        enterprise_url: None,
    });
    assert!(super::validate_session_model(&model, &test_provider(None), Some(&oauth)).is_ok());
    model.api.adapter = "acp".into();
    assert!(super::validate_session_model(&model, &info, None).is_err());
    model.api.adapter = "openai-compatible".into();
    model.status = "disabled".into();
    assert!(super::validate_session_model(&model, &info, None).is_err());
    model.status = "active".into();
    model.provider_id = "different-provider".into();
    assert!(super::validate_session_model(&model, &info, None).is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn employee_acp_host_sends_only_current_text_and_restores_original_id() {
    use crate::acp::{
        host::{EmployeeAcpKind, EmployeeAcpLaunch},
        host_migration_20_tests::{CacheHost, EMPLOYEE_LAUNCH, REQUEST_CONFIG},
    };
    use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

    const ROOT: &str = "SHIJU_EMPLOYEE_ACP_HOST_FIXTURE";
    let Some(root) = std::env::var_os(ROOT) else {
        let root = tempfile::tempdir().unwrap();
        let name = format!(
            "{}::employee_acp_host_sends_only_current_text_and_restores_original_id",
            module_path!().split_once("::").unwrap().1
        );
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &name, "--nocapture"])
            .env(ROOT, root.path())
            .env("VIBEWINDOW_TEST_ROOT", root.path())
            .env("VIBEWINDOW_CONFIG_DIR", root.path())
            .env("HOME", root.path())
            .env(EMPLOYEE_ACP_CREDENTIAL_ENV, "must-not-inherit")
            .status()
            .unwrap();
        assert!(status.success());
        assert!(root.path().join("requests").is_file());
        return;
    };
    let root = PathBuf::from(root).canonicalize().unwrap();
    let executable = root.join("shiju");
    fs::write(
        &executable,
        r#"#!/bin/sh
exec /usr/bin/env python3 -c '
import json, os, pathlib, sys
os.chdir(pathlib.Path(sys.argv.pop(1)).parent)
employee = sys.argv[1:2] == ["employee-acp"]
if employee:
    assert sys.argv[2:] == ["--employee-id", "worker-1", "--employee-kind", "private", "--state-dir", os.getcwd(), "--account-scope", "fixture-account"]
    assert os.environ.get("SHIJU_EMPLOYEE_ACP_CREDENTIAL") == "fixture-turn-credential"
else:
    assert "SHIJU_EMPLOYEE_ACP_CREDENTIAL" not in os.environ
pending = None
for line in sys.stdin:
    request = json.loads(line)
    method = request.get("method")
    if method != "session/cancel" and ("id" not in request or method is None):
        continue
    with open("requests", "a") as capture:
        capture.write(line)
    if method == "session/cancel":
        assert request["params"]["sessionId"] == "employee-fixture"
        print(json.dumps({"jsonrpc": "2.0", "id": pending, "result": {"stopReason": "cancelled"}}), flush=True)
        pending = None
        continue
    if method == "session/prompt" and request["params"]["prompt"][0]["text"] == "wait for stop":
        pending = request["id"]
        continue
    response = {"jsonrpc": "2.0", "id": request["id"]}
    if method == "initialize":
        result = {"protocolVersion": 1, "agentCapabilities": {"loadSession": True}, "authMethods": []}
    elif method == "session/new":
        result = {"sessionId": "employee-fixture" if employee else "generic-fixture"}
    elif method == "session/load" and pathlib.Path("reject-load").exists():
        response["error"] = {"code": -32602, "message": "fixture could not restore"}
        print(json.dumps(response), flush=True)
        continue
    elif method in ("session/load", "session/prompt"):
        text = "history replay" if method == "session/load" else "reply"
        print(json.dumps({"jsonrpc": "2.0", "method": "session/update", "params": {"sessionId": request["params"]["sessionId"], "update": {"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": text}}}}), flush=True)
        result = {"stopReason": "end_turn"} if method == "session/prompt" else {}
    else:
        result = {}
    response["result"] = result
    print(json.dumps(response), flush=True)
' "$0" "$@"
"#,
    ).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let launch = EmployeeAcpLaunch {
        employee_id: "worker-1".into(),
        kind: EmployeeAcpKind::Private,
        account_scope: "fixture-account".into(),
        state_dir: root.clone(),
        credential: "fixture-turn-credential".into(),
        executable: Some(executable.clone()),
    };
    let mut config = agent_config::schema::Config::default();
    for backend in ["employee-acp", "local-fixture"] {
        config.acp.insert(
            backend.into(),
            agent_config::schema::AcpAgentConfig {
                command: if backend == "employee-acp" {
                    "must-not-execute".into()
                } else {
                    executable.display().to_string()
                },
                args: vec![],
                env: [(EMPLOYEE_ACP_CREDENTIAL_ENV.to_owned(), "must-not-inherit".into())].into(),
            },
        );
    }
    let model = test_model("unused-provider");
    let provider = test_provider(None);
    let messages = json!([
        {"role":"system","content":"local system context"},
        {"role":"user","content":"old text"},
        {"role":"assistant","content":"old answer"},
        {"role":"user","content":"<system-reminder>workspace rules</system-reminder>"},
        {"role":"user","content":"  current text\n"},
        {"role":"user","content":"local hook", "_vw_source":{"kind":"context"}}
    ]);
    let replies = std::sync::Mutex::new(Vec::new());
    let request = async |backend: &str,
                         session: &str,
                         messages: &Value,
                         source: &str,
                         abort: Option<&tokio::sync::watch::Receiver<bool>>| {
        let mut options = json!({
            "cwd": root, "acp_agent": backend, "acp_model_source": source,
            "acp_close_client_after_request": true,
        });
        if backend == "employee-acp" {
            options["full_access"] = json!(true);
            options["acp_force_new_session"] = json!(true);
            options["acp_history_strategy"] = json!("full");
            options["acp_allowed_tools"] = json!(["shell"]);
            options["acp_session_model"] = json!("unused");
        }
        do_stream_request_acp::<CacheHost>(
            &model,
            &provider,
            None,
            &options,
            messages,
            session,
            abort,
            &mut |event| {
                if let StreamEvent::Delta(text) = event {
                    replies.lock().unwrap().push(text);
                }
            },
        )
        .await
    };
    REQUEST_CONFIG.scope(config, async {
        // The backend name and a configured command give no launch authority.
        let error = request("employee-acp", "employee-chat", &messages, "session_model", None)
            .await.unwrap_err();
        assert!(error.to_string().contains("authorized session binding"));
        EMPLOYEE_LAUNCH.scope(launch.clone(), async {
            let invalid = json!([{"role":"user","content":[{"type":"image_url","image_url":{"url":"file:///private"}}]}]);
            assert!(request("employee-acp", "employee-chat", &invalid, "session_model", None).await.is_err());
            assert!(!root.join("requests").exists());
            request("employee-acp", "employee-chat", &messages, "session_model", None).await.unwrap();
            acp_storage::close_session("employee-fixture").await.unwrap();
            request("employee-acp", "employee-chat", &messages, "session_model", None).await.unwrap();
            fs::write(root.join("reject-load"), "").unwrap();
            assert!(request("employee-acp", "employee-chat", &messages, "session_model", None).await.is_err());
        }).await;
        let capture = fs::read_to_string(root.join("requests")).unwrap();
        let frames: Vec<Value> = capture.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
        let methods = |method: &'static str| frames.iter().filter(move |frame| frame["method"] == method);
        assert_eq!(methods("session/new").count(), 1);
        assert_eq!(methods("session/load").count(), 2);
        assert!(methods("session/load").all(|frame| frame["params"]["sessionId"] == "employee-fixture"));
        assert_eq!(methods("session/prompt").count(), 2);
        assert!(methods("session/prompt").all(|frame| frame["params"]["prompt"] == json!([
            {"type":"text","text":"  current text\n"}
        ])));
        assert_eq!(methods("session/new").next().unwrap()["params"]["mcpServers"], json!([]));
        let capabilities = &methods("initialize").next().unwrap()["params"]["clientCapabilities"];
        assert_ne!(capabilities["terminal"], true);
        assert_ne!(capabilities["fs"]["readTextFile"], true);
        assert_ne!(capabilities["fs"]["writeTextFile"], true);
        assert!(!capture.contains("fixture-turn-credential"));
        assert_eq!(*replies.lock().unwrap(), ["reply", "reply"]);
        let record = acp_storage::resolve_session_record("employee-fixture").await.unwrap();
        assert!(record.agent_config.is_none());
        assert!(!serde_json::to_string(&record).unwrap().contains("fixture-turn-credential"));

        fs::remove_file(root.join("reject-load")).unwrap();
        let (cancel, abort) = tokio::sync::watch::channel(false);
        let pending = json!([{"role":"user","content":"wait for stop"}]);
        EMPLOYEE_LAUNCH.scope(launch, async {
            let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(15), async {
                tokio::join!(
                    request("employee-acp", "employee-chat", &pending, "session_model", Some(&abort)),
                    async {
                        while !fs::read_to_string(root.join("requests")).unwrap().contains("wait for stop") {
                            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        }
                        cancel.send(true).unwrap();
                    },
                )
            }).await.unwrap();
            assert!(matches!(result, Err(Error::Aborted)));
        }).await;
        assert!(fs::read_to_string(root.join("requests")).unwrap().contains("session/cancel"));

        let mut generic_messages = messages.clone();
        generic_messages.as_array_mut().unwrap().pop();
        request("local-fixture", "generic-chat", &generic_messages, "cli_local", None).await.unwrap();
        let capture = fs::read_to_string(root.join("requests")).unwrap();
        let frame: Value = capture.lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .rfind(|frame| frame["method"] == "session/prompt").unwrap();
        let prompt = frame["params"]["prompt"][0]["text"].as_str().unwrap();
        assert!(prompt.contains("workspace rules"));
        assert!(prompt.contains("current text"));
    }).await;
}
