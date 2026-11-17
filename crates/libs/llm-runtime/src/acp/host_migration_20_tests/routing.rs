use serde_json::json;
use std::collections::HashMap;
use tokio::sync::watch;
use model_types as provider;
use message_types as message;
use crate::{Error, StreamEvent};
use super::CacheHost;
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

#[tokio::test]
// 中止信号在配置和认证前检查，取消请求不应产生额外读取。
async fn stream_request_reports_abort_before_config_lookup() {
    let (_tx, rx) = watch::channel(true);
    let mut events = Vec::new();

    let result = super::super::request::do_stream_request_acp::<CacheHost>(
        &test_model("github-copilot"),
        &test_provider(None),
        None,
        &json!({}),
        &json!([{"role": "user", "content": "hello"}]),
        "session-1",
        Some(&rx),
        &mut |event| events.push(event),
    )
    .await;

    assert!(matches!(result, Err(Error::Aborted)));
    assert_eq!(events.len(), 1);
    match &events[0] {
        StreamEvent::Error(message::AssistantError::MessageAbortedError { message }) => {
            assert_eq!(message, "aborted");
        }
        other => panic!("expected abort stream event, got {other:?}"),
    }
}

#[tokio::test]
async fn agent_model_inheritance_rejects_missing_credentials_before_launch() {
    let result = super::super::request::do_stream_request_acp::<CacheHost>(
        &test_model("github-copilot"),
        &test_provider(None),
        None,
        &json!({"acp_agent": "__must_not_launch__", "acp_model_source": "session_model"}),
        &json!([{"role": "user", "content": "hello"}]),
        "routing-no-auth",
        None,
        &mut |_| {},
    )
    .await;
    assert!(matches!(
        result,
        Err(Error::Api(message::AssistantError::ProviderAuthError { .. }))
    ));
}


#[tokio::test]
async fn acp_model_source_routing_local_reaches_command_lookup_without_client_credentials() {
    let result = super::super::request::do_stream_request_acp::<CacheHost>(
        &test_model("github-copilot"),
        &test_provider(None),
        None,
        &json!({"acp_agent": "__unconfigured_routing_agent__", "acp_model_source": "cli_local"}),
        &json!([{"role": "user", "content": "hello"}]),
        "routing-local",
        None,
        &mut |_| {},
    )
    .await;
    match result {
        Err(Error::Api(message::AssistantError::Unknown { message })) => {
            assert!(message.contains("acp command not configured"))
        }
        other => panic!("expected command lookup without client auth, got {other:?}"),
    }
}

#[cfg(unix)]
#[tokio::test]
async fn acp_local_blank_anchor_resumes_existing_session() {
    use vw_api_acp_model::acp_model_source::AcpModelSource;

    let Some(root) = std::env::var_os("SHIJU_ACP_LOCAL_RECOVERY_FIXTURE") else {
        let root = tempfile::tempdir().unwrap();
        let name = format!(
            "{}::acp_local_blank_anchor_resumes_existing_session",
            module_path!().split_once("::").unwrap().1
        );
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &name, "--nocapture"])
            .env("SHIJU_ACP_LOCAL_RECOVERY_FIXTURE", root.path())
            .env("VW_MODEL_RECOVERY_TEST_ROOT", root.path())
            .env("VIBEWINDOW_TEST_ROOT", root.path())
            .env("VIBEWINDOW_CONFIG_DIR", root.path())
            .env("HOME", root.path())
            .status()
            .unwrap();
        assert!(status.success());
        assert!(root.path().join("calls").is_file(), "the isolated ACP fixture did not run");
        return;
    };
    let root = std::path::PathBuf::from(root);
    let workspace = root.join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let script = root.join("peer.py");
    let calls = root.join("calls");
    let reject = root.join("reject-resume");
    std::fs::write(
        &script,
        r#"
import json, pathlib, sys
for line in sys.stdin:
    request = json.loads(line)
    if 'id' not in request or 'method' not in request:
        continue
    method = request['method']
    session = request.get('params', {}).get('sessionId', '')
    with open(sys.argv[1], 'a') as calls:
        calls.write(method + ' ' + session + '\n')
    response = {'jsonrpc': '2.0', 'id': request['id']}
    if method == 'initialize':
        response['result'] = {'protocolVersion': 1, 'agentCapabilities': {'loadSession': True, 'sessionCapabilities': {'resume': {}}}, 'authMethods': []}
    elif method == 'session/new':
        response['result'] = {'sessionId': 'original-native-session'}
    elif method in ('session/resume', 'session/load') and pathlib.Path(sys.argv[2]).exists():
        response['error'] = {'code': -32602, 'message': 'fixture refused old session'}
    elif method == 'session/prompt':
        response['result'] = {'stopReason': 'end_turn'}
    else:
        response['result'] = {}
    print(json.dumps(response), flush=True)
"#,
    )
    .unwrap();
    let agent = "local-fixture";
    let mut config = serde_json::to_value(agent_config::runtime::Config::default()).unwrap();
    config["acp"] = json!({agent: {"command":"python3", "args":[&script, &calls, &reject]}});
    let config = serde_json::from_value(config).unwrap();
    super::REQUEST_CONFIG.scope(config, async {
        let session = "local-recovery-chat";
        let mut options = json!({
            "cwd": workspace, "acp_agent": agent, "acp_model_source": "cli_local",
            "acp_close_client_after_request": true
        });
        let model = test_model("unused-provider");
        let provider = test_provider(None);
        let messages = json!([{"role": "user", "content": "continue"}]);
        let request = async |options: serde_json::Value| {
            super::super::request::do_stream_request_acp::<CacheHost>(
                &model,
                &provider,
                None,
                &options,
                &messages,
                session,
                None,
                &mut |_| {},
            )
            .await
        };
        request(options.clone()).await.unwrap();
        let store =
            <CacheHost as super::super::host::Host>::open_selection(session, workspace.to_str().unwrap()).unwrap();
        let mut saved = store.read().unwrap().unwrap();
        assert_eq!(saved.model.source, AcpModelSource::CliLocal);
        assert_eq!(saved.record_id, "original-native-session");
        saved.record_id = " ".into();
        saved.generation = 1;
        store.commit(&saved).unwrap();

        options["acp_force_new_session"] = json!(true);
        std::fs::write(&reject, "").unwrap();
        assert!(request(options.clone()).await.is_err());
        assert_eq!(store.read().unwrap().unwrap().record_id, " ");
        std::fs::remove_file(&reject).unwrap();
        request(options).await.unwrap();

        let restored = store.read().unwrap().unwrap();
        assert_eq!(restored.generation, 0);
        assert_eq!(restored.record_id, "original-native-session");
        let calls = std::fs::read_to_string(calls).unwrap();
        assert_eq!(calls.lines().filter(|line| line.starts_with("session/new ")).count(), 1);
        assert!(calls.lines().any(|line| line == "session/resume original-native-session"));
        assert_eq!(
            calls.lines().filter(|line| *line == "session/prompt original-native-session").count(),
            2
        );
    })
    .await;
}
