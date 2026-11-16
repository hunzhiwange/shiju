//! 验证 ACP 请求选项、模型和消息载荷的构造规则。

use std::collections::HashMap;

use serde_json::json;

use crate::app::agent::provider::provider;
use crate::app::agent::session::message;

use super::do_stream_request_acp;

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
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (project, _) = crate::project::from_directory(&tenant, &workspace).await.unwrap();
    crate::config::update_project_for_tenant(
        &tenant,
        &workspace,
        json!({"acp": {agent: {"command": "python3", "args": [&script, &calls, &reject]}}}),
    )
    .await
    .unwrap();
    let context = crate::project::instance::Context {
        directory: workspace.to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    crate::project::instance::provide_context(context, async {
        let session = "local-recovery-chat";
        let mut options = json!({
            "cwd": workspace, "acp_agent": agent, "acp_model_source": "cli_local",
            "acp_close_client_after_request": true
        });
        let model = test_model("unused-provider");
        let provider = test_provider(None);
        let messages = json!([{"role": "user", "content": "continue"}]);
        let request = async |options: serde_json::Value| {
            do_stream_request_acp(
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
            super::super::lifecycle::Store::open(session, workspace.to_str().unwrap()).unwrap();
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


#[cfg(unix)]
#[tokio::test]
async fn acp_model_source_launch_profile_production_request_rejects_unverified_driver() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("unexpected-launch.py");
    let launched = directory.path().join("launched");
    std::fs::write(
        &script,
        "import pathlib, sys\npathlib.Path(sys.argv[1]).write_text('unexpected launch')\n",
    )
    .unwrap();
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (project, _) = crate::project::from_directory(&tenant, directory.path()).await.unwrap();
    crate::config::update_project_for_tenant(&tenant, directory.path(), json!({
        "providers": {"github-copilot": {"api": "http://127.0.0.1:1/v1", "models": {"gpt-5-mini": {"status": "active"}}}},
        "acp": {"unverified-fixture": {"command": "python3", "args": [&script, &launched]}}
    })).await.unwrap();
    let context = crate::project::instance::Context {
        directory: directory.path().to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    // A synthetic driver stays unverified as reviewed CLI support expands.
    let result = crate::project::instance::provide_context(
        context,
        do_stream_request_acp(
            &test_model("github-copilot"),
            &test_provider(Some("provider-secret-never-distributed")),
            None,
            &json!({"acp_agent": "unverified-fixture", "acp_model_source": "session_model", "cwd": directory.path()}),
            &json!([{"role": "user", "content": "hello"}]),
            "task9-production",
            None,
            &mut |_| {},
        ),
    )
    .await;
    match result {
        Err(super::Error::Api(message::AssistantError::Unknown { message })) => {
            assert!(message.contains("ACP session_model driver is not verified"), "{message}");
            assert!(!message.contains("provider-secret-never-distributed"));
        }
        other => panic!("unexpected production result: {other:?}"),
    }
    assert!(!launched.exists(), "an unverified driver must be rejected before launch");
}
