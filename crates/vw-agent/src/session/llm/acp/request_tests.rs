//! 验证 ACP 请求选项、模型和消息载荷的构造规则。

use std::collections::HashMap;

use serde_json::json;
use tokio::sync::watch;

use crate::app::agent::auth;
use crate::app::agent::provider::provider;
use crate::app::agent::session::{llm::StreamEvent, message};

use super::{do_stream_request_acp, parse_close_client_after_request};

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
    use vw_api_types::acp_model_source::AcpModelSource;

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
        crate::app::agent::session::llm::Error::Api(message::AssistantError::Unknown {
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
        crate::app::agent::session::llm::Error::Api(message::AssistantError::Unknown {
            message,
        }) => assert_eq!(
            message,
            "acp option error: acp_close_client_after_request must be a boolean"
        ),
        other => panic!("expected ACP option error, got {other:?}"),
    }
}

#[tokio::test]
// 中止信号在配置和认证前检查，取消请求不应产生额外读取。
async fn stream_request_reports_abort_before_config_lookup() {
    let (_tx, rx) = watch::channel(true);
    let mut events = Vec::new();

    let result = do_stream_request_acp(
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

    assert!(matches!(result, Err(crate::app::agent::session::llm::Error::Aborted)));
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
    let result = do_stream_request_acp(
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
        Err(super::Error::Api(message::AssistantError::ProviderAuthError { .. }))
    ));
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

#[tokio::test]
async fn acp_model_source_routing_local_reaches_command_lookup_without_client_credentials() {
    let result = do_stream_request_acp(
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
        Err(super::Error::Api(message::AssistantError::Unknown { message })) => {
            assert!(message.contains("acp command not configured"))
        }
        other => panic!("expected command lookup without client auth, got {other:?}"),
    }
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

#[cfg(unix)]
#[tokio::test]
async fn agent_acp_defaults_reject_unsupported_mode() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let temp = tempfile::tempdir().unwrap();
    let script = temp.path().join("peer.py");
    let capture = temp.path().join("requests.jsonl");
    std::fs::write(&script, r#"
import json, sys
for line in sys.stdin:
    r=json.loads(line)
    if 'method' not in r or 'id' not in r: continue
    with open(sys.argv[1],'a') as f: f.write(line)
    result = {'protocolVersion':1,'agentCapabilities':{},'authMethods':[]} if r['method']=='initialize' else {'sessionId':'defaults'} if r['method']=='session/new' else {}
    print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':result}),flush=True)
"#).unwrap();
    let client = vw_acp::AcpClient::new(
        "defaults",
        vw_acp::AcpAgentConfig {
            command: "python3".into(),
            args: vec![script.display().to_string(), capture.display().to_string()],
            env: Default::default(),
        },
    );
    let session = client.create_session(temp.path()).await.unwrap();
    let error =
        super::apply_default_mode(&client, &session.session_id, temp.path(), "unadvertised".into())
            .await
            .unwrap_err();
    assert!(error.to_string().contains("ACP session mode is not advertised: unadvertised"));
    client.close().await.unwrap();
    let requests = std::fs::read_to_string(capture).unwrap();
    assert!(!requests.contains("session/set_mode"));
    assert!(!requests.contains("session/prompt"));
}

#[tokio::test]
async fn acp_skill_mount_outbound_defaults_preserve_rules_without_body() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let temp = tempfile::tempdir().unwrap();
    let skill_dir = temp.path().join("skills/selected");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: selected\ndescription: fixture\n---\nSCOPED_SKILL_CONTENT\n",
    )
    .unwrap();
    let mut cfg = crate::config::Config::default();
    cfg.agents.insert(
        "acp-codex".into(),
        serde_json::from_value(json!({
            "provider":"", "model":"", "system_prompt":"TRUSTED_AGENT_RULE", "defaults":{
                "skills":{"mode":"fixed","value":["selected"]},
                "mcp":{"mode":"fixed","value":[]}
            }
        }))
        .unwrap(),
    );
    let options = json!({"agent":"acp-codex", "mcp_servers":[{"name":"UNTRUSTED"}]});
    let messages = json!([{"role":"user","content":"hello"}]);
    let mut parsed = super::super::ParsedAcpOptions::default();
    let resolved = super::super::config::apply_agent_defaults(
        &cfg,
        &options,
        "codex",
        temp.path(),
        &mut parsed,
        &messages,
    )
    .await
    .unwrap();
    assert!(parsed.mcp_servers.is_empty());
    let prompt = super::super::replay::build_request_prompt(
        &resolved,
        false,
        super::super::AcpReplayStrategy::Discard,
        3,
    );
    assert_eq!(prompt.matches("TRUSTED_AGENT_RULE").count(), 1);
    assert_eq!(prompt.matches("SCOPED_SKILL_CONTENT").count(), 0);
    assert_eq!(parsed.skill_mounts.len(), 1);
    assert_eq!(
        std::path::Path::new(&parsed.skill_mounts[0].path),
        skill_dir.canonicalize().unwrap()
    );
    // A protocol-error peer records the real JSON-RPC request without simulating skill discovery.
    let script = temp.path().join("mount-error-peer.py");
    let capture = temp.path().join("mount-requests.jsonl");
    std::fs::write(&script, r#"
import json, sys
for line in sys.stdin:
    r = json.loads(line)
    if 'method' not in r or 'id' not in r: continue
    with open(sys.argv[1], 'a') as f: f.write(line)
    if r['method'] == 'session/prompt':
        response = {'jsonrpc':'2.0','id':r['id'],'error':{'code':-32602,'message':'intentional protocol error'}}
    else:
        result = {'protocolVersion':1,'agentCapabilities':{},'authMethods':[]} if r['method'] == 'initialize' else {'sessionId':'mount-error'}
        response = {'jsonrpc':'2.0','id':r['id'],'result':result}
    print(json.dumps(response), flush=True)
"#).unwrap();
    let view = super::super::skill_mounts::MountedSkills::prepare(
        &parsed.skill_mounts,
        temp.path(),
        "mount-error",
        &cfg.skills.security,
    )
    .unwrap()
    .unwrap();
    let selection =
        vw_acp::SessionMountSelection { version: 1, skills: parsed.skill_mounts.clone() };
    let client = vw_acp::AcpClient::new(
        "protocol-error-peer",
        vw_acp::AcpAgentConfig {
            command: "python3".into(),
            args: vec![script.display().to_string(), capture.display().to_string()],
            env: Default::default(),
        },
    )
    .with_skill_mounts(Some(selection), Some(view.0));
    let result = client
        .run_prompt(vw_acp::PromptRequest::new(temp.path().to_path_buf(), prompt), &mut |_| {})
        .await;
    assert!(result.is_err());
    client.close().await.unwrap();
    let requests: Vec<serde_json::Value> = std::fs::read_to_string(capture)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let new = requests.iter().find(|request| request["method"] == "session/new").unwrap();
    assert_eq!(
        new["params"]["_meta"][vw_acp::SESSION_SKILL_MOUNTS_META_KEY]["skills"][0]["name"],
        "selected"
    );
    let first =
        requests.iter().find(|request| request["method"] == "session/prompt").unwrap().to_string();
    assert!(first.contains("TRUSTED_AGENT_RULE"));
    assert!(!first.contains("SCOPED_SKILL_CONTENT"));
    let again = super::super::config::apply_agent_defaults(
        &cfg,
        &options,
        "codex",
        temp.path(),
        &mut parsed,
        &resolved,
    )
    .await
    .unwrap();
    assert_eq!(resolved, again);
    crate::config::update(json!({"mcpServers":[{"name":"selected-mcp","command":"fixture-mcp"}]}))
        .await
        .unwrap();
    cfg.agents.get_mut("acp-codex").unwrap().defaults.mcp =
        vw_config_types::agent::AgentDefault::Fixed(vec!["selected-mcp".into()]);
    super::super::config::apply_agent_defaults(
        &cfg,
        &options,
        "codex",
        temp.path(),
        &mut parsed,
        &messages,
    )
    .await
    .unwrap();
    assert_eq!(parsed.mcp_servers.len(), 1);
    assert_eq!(crate::tools::mcp_common::server_name(&parsed.mcp_servers[0]), "selected-mcp");
    crate::config::update_project(
        temp.path(),
        json!({"mcpServerOverrides":[{"name":"selected-mcp","enabled":false}]}),
    )
    .await
    .unwrap();
    let error = super::super::config::apply_agent_defaults(
        &cfg,
        &options,
        "codex",
        temp.path(),
        &mut parsed,
        &messages,
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("MCP capability is disabled"));
    cfg.agents.get_mut("acp-codex").unwrap().defaults.mcp =
        vw_config_types::agent::AgentDefault::Fixed(vec![]);
    std::fs::write(crate::skills::local_skill_disabled_marker_path(&skill_dir), "").unwrap();
    assert!(
        super::super::config::apply_agent_defaults(
            &cfg,
            &options,
            "codex",
            temp.path(),
            &mut parsed,
            &messages
        )
        .await
        .is_err()
    );
    let agent = cfg.agents.get_mut("acp-codex").unwrap();
    agent.defaults.skills = vw_config_types::agent::AgentDefault::Fixed(vec![]);
    agent.defaults.permission = vw_config_types::agent::AgentDefault::Fixed(json!({"*":"allow"}));
    let error = super::super::config::apply_agent_defaults(
        &cfg,
        &options,
        "codex",
        temp.path(),
        &mut parsed,
        &messages,
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("ACP does not support this fixed permission policy"));
    cfg.agents.get_mut("acp-codex").unwrap().defaults.permission =
        vw_config_types::agent::AgentDefault::Fixed(json!({"*":"deny"}));
    super::super::config::apply_agent_defaults(
        &cfg,
        &options,
        "codex",
        temp.path(),
        &mut parsed,
        &messages,
    )
    .await
    .unwrap();
    assert_eq!(parsed.permission_mode, Some(vw_acp::PermissionMode::DenyAll));
    assert!(parsed.skill_mounts.is_empty());
}
