//! 验证 ACP 会话创建、复用、配置更新与错误恢复。

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use tokio::sync::{Mutex, watch};
use vw_acp::{
    AcpSessionOptions, AuthPolicy, DEFAULT_EVENT_MAX_SEGMENTS, NonInteractivePermissionPolicy,
    PermissionMode,
};

use crate::app::agent::config;
use crate::app::agent::session::message;

use super::*;

static CACHE_LOCK: Mutex<()> = Mutex::const_new(());

fn acp_config(command: &str) -> config::schema::AcpAgentConfig {
    config::schema::AcpAgentConfig {
        command: command.to_string(),
        args: vec!["--stdio".to_string()],
        env: HashMap::new(),
    }
}

fn parsed_options() -> ParsedAcpOptions {
    ParsedAcpOptions {
        mcp_servers: Vec::new(),
        permission_mode: Some(PermissionMode::DenyAll),
        full_access: false,
        non_interactive_permissions: Some(NonInteractivePermissionPolicy::Fail),
        auth_policy: Some(AuthPolicy::Fail),
        session_mode: Some("plan".to_string()),
        session_options: Some(AcpSessionOptions {
            model: Some("gpt-5".to_string()),
            allowed_tools: Some(vec![" shell ".to_string(), "".to_string(), "edit".to_string()]),
            max_turns: Some(7),
        }),
        session_config_options: vec![("reasoning_effort".to_string(), "high".to_string())],
    }
}

fn assistant_message_text(error: Error) -> String {
    match error {
        Error::Api(message::AssistantError::Unknown { message }) => message,
        other => panic!("expected unknown assistant error, got {other:?}"),
    }
}

#[test]
fn clone_non_empty_tools_trims_entries_and_drops_empty_values() {
    assert_eq!(
        clone_non_empty_tools(&[" shell ".to_string(), "".to_string(), " edit".to_string()]),
        Some(vec!["shell".to_string(), "edit".to_string()])
    );
    assert_eq!(clone_non_empty_tools(&[" ".to_string(), "\t".to_string()]), None);
}

#[test]
fn clone_non_empty_tools_preserves_order_and_duplicates() {
    assert_eq!(
        clone_non_empty_tools(&[" edit ".to_string(), "shell".to_string(), "edit".to_string(),]),
        Some(vec!["edit".to_string(), "shell".to_string(), "edit".to_string()])
    );
}

#[test]
fn should_abort_reflects_watch_receiver_state() {
    assert!(!should_abort(None));

    let (tx, rx) = watch::channel(false);
    assert!(!should_abort(Some(&rx)));

    tx.send(true).expect("watch receiver should be open");
    assert!(should_abort(Some(&rx)));
}

#[test]
fn acp_session_name_trims_and_namespaces_non_empty_ids() {
    assert_eq!(acp_session_name("  local-1  ").as_deref(), Some("vw-session:local-1"));
    assert_eq!(acp_session_name(" \n\t "), None);
}

#[test]
fn cache_key_is_stable_and_includes_behavioral_inputs() {
    let mut cfg = acp_config("agent");
    cfg.args.push("--fast".to_string());
    cfg.env.insert("B".to_string(), "2".to_string());
    cfg.env.insert("A".to_string(), "1".to_string());

    let key = acp_client_cache_key("custom", &cfg, Path::new("/tmp/project"), &parsed_options());
    assert_eq!(key.len(), 64);
    assert!(key.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(
        key,
        acp_client_cache_key("custom", &cfg, Path::new("/tmp/project"), &parsed_options())
    );
    let mut changed = cfg.clone();
    changed.args.push("--different".into());
    assert_ne!(
        key,
        acp_client_cache_key("custom", &changed, Path::new("/tmp/project"), &parsed_options())
    );
    assert_ne!(
        key,
        acp_client_cache_key("other", &cfg, Path::new("/tmp/project"), &parsed_options())
    );
    assert_ne!(key, acp_client_cache_key("custom", &cfg, Path::new("/other"), &parsed_options()));
}

#[test]
// 环境映射插入顺序不属于会话行为，不能导致缓存键漂移。
fn cache_key_is_stable_across_environment_insertion_order() {
    let mut first = acp_config("agent");
    first.env.insert("B".to_string(), "2".to_string());
    first.env.insert("A".to_string(), "1".to_string());

    let mut second = acp_config("agent");
    second.env.insert("A".to_string(), "1".to_string());
    second.env.insert("B".to_string(), "2".to_string());

    assert_eq!(
        acp_client_cache_key("custom", &first, Path::new("/tmp/project"), &parsed_options()),
        acp_client_cache_key("custom", &second, Path::new("/tmp/project"), &parsed_options())
    );
}

#[test]
fn cache_key_changes_when_session_behavior_changes() {
    let cfg = acp_config("agent");
    let cwd = Path::new("/tmp/project");
    let baseline = parsed_options();
    let baseline_key = acp_client_cache_key("custom", &cfg, cwd, &baseline);

    let mut changed = parsed_options();
    changed.auth_policy = Some(AuthPolicy::Skip);
    assert_ne!(baseline_key, acp_client_cache_key("custom", &cfg, cwd, &changed));

    let mut changed = parsed_options();
    changed.session_options.as_mut().expect("session options").max_turns = Some(8);
    assert_ne!(baseline_key, acp_client_cache_key("custom", &cfg, cwd, &changed));
}

#[test]
fn acp_client_cache_key_separates_full_access_policy() {
    let cfg = acp_config("agent");
    let cwd = Path::new("/tmp/project");
    let standard = parsed_options();
    let mut full_access = standard.clone();
    full_access.full_access = true;

    assert_ne!(
        acp_client_cache_key("custom", &cfg, cwd, &standard),
        acp_client_cache_key("custom", &cfg, cwd, &full_access)
    );
}

#[test]
fn normalize_session_name_rejects_blank_names() {
    assert_eq!(normalize_session_name(Some("  worktree  ")).as_deref(), Some("worktree"));
    assert_eq!(normalize_session_name(Some("   ")), None);
    assert_eq!(normalize_session_name(None), None);
}

#[tokio::test]
async fn cached_client_is_reused_only_for_matching_keys() {
    let _guard = CACHE_LOCK.lock().await;
    ACP_CLIENT_CACHE.lock().clear();

    let cwd = Path::new("/tmp/project");
    let cfg = acp_config("agent");
    let options = parsed_options();

    let first = get_cached_acp_client("custom", &cfg, cwd, &options);
    let second = get_cached_acp_client("custom", &cfg, cwd, &options);
    assert!(Arc::ptr_eq(&first.client, &second.client));

    let mut changed_cfg = cfg.clone();
    changed_cfg.args.push("--other".to_string());
    let changed = get_cached_acp_client("custom", &changed_cfg, cwd, &options);
    assert!(!Arc::ptr_eq(&first.client, &changed.client));

    ACP_CLIENT_CACHE.lock().clear();
}

#[tokio::test]
async fn close_cached_client_removes_matching_unshared_entry() {
    let _guard = CACHE_LOCK.lock().await;
    ACP_CLIENT_CACHE.lock().clear();

    let entry = get_cached_acp_client(
        "custom",
        &acp_config("agent"),
        Path::new("/tmp/project"),
        &parsed_options(),
    );
    assert!(ACP_CLIENT_CACHE.lock().contains_key(&entry.key));

    close_cached_acp_client_if_unshared(&entry).await;

    assert!(!ACP_CLIENT_CACHE.lock().contains_key(&entry.key));
}

#[tokio::test]
// 共享客户端仍被其他请求持有时不能提前关闭。
async fn close_cached_client_keeps_entry_while_another_request_shares_it() {
    let _guard = CACHE_LOCK.lock().await;
    ACP_CLIENT_CACHE.lock().clear();

    let entry = get_cached_acp_client(
        "custom",
        &acp_config("agent"),
        Path::new("/tmp/project"),
        &parsed_options(),
    );
    let shared_client = entry.client.clone();

    close_cached_acp_client_if_unshared(&entry).await;

    let cache = ACP_CLIENT_CACHE.lock();
    let cached = cache.get(&entry.key).expect("shared client should remain cached");
    assert!(Arc::ptr_eq(cached, &shared_client));
    drop(cache);
    ACP_CLIENT_CACHE.lock().clear();
}

#[tokio::test]
async fn close_cached_client_keeps_replacement_for_stale_entry() {
    let _guard = CACHE_LOCK.lock().await;
    ACP_CLIENT_CACHE.lock().clear();

    let cfg = acp_config("agent");
    let options = parsed_options();
    let stale = get_cached_acp_client("custom", &cfg, Path::new("/tmp/project"), &options);
    let replacement = build_cached_acp_client("custom", &cfg, &options);
    ACP_CLIENT_CACHE.lock().insert(stale.key.clone(), replacement.clone());

    close_cached_acp_client_if_unshared(&stale).await;

    let cache = ACP_CLIENT_CACHE.lock();
    let cached = cache.get(&stale.key).expect("replacement should remain cached");
    assert!(Arc::ptr_eq(cached, &replacement));
    drop(cache);
    ACP_CLIENT_CACHE.lock().clear();
}

#[test]
fn session_event_log_for_record_uses_bounded_defaults() {
    let log = session_event_log_for_record("session-1");

    assert_eq!(log.segment_count, DEFAULT_EVENT_MAX_SEGMENTS);
    assert_eq!(log.max_segments, DEFAULT_EVENT_MAX_SEGMENTS);
    assert!(log.max_segment_bytes > 0);
}

#[test]
fn session_state_options_omits_empty_state_and_filters_allowed_tools() {
    assert_eq!(session_state_options(None), None);
    assert_eq!(session_state_options(Some(&AcpSessionOptions::default())), None);

    let state = session_state_options(Some(&AcpSessionOptions {
        model: Some("gpt-5".to_string()),
        allowed_tools: Some(vec![" shell ".to_string(), " ".to_string(), "edit".to_string()]),
        max_turns: Some(4),
    }))
    .expect("non-empty options should produce state");

    assert_eq!(state.model.as_deref(), Some("gpt-5"));
    assert_eq!(state.allowed_tools, Some(vec!["shell".to_string(), "edit".to_string()]));
    assert_eq!(state.max_turns, Some(4));
}

#[test]
fn session_state_options_keeps_model_or_turns_without_tools() {
    let model_only = session_state_options(Some(&AcpSessionOptions {
        model: Some("gpt-5-mini".to_string()),
        allowed_tools: None,
        max_turns: None,
    }))
    .expect("model should be state");
    assert_eq!(model_only.model.as_deref(), Some("gpt-5-mini"));
    assert_eq!(model_only.allowed_tools, None);

    let turns_only = session_state_options(Some(&AcpSessionOptions {
        model: None,
        allowed_tools: Some(vec![" ".to_string()]),
        max_turns: Some(2),
    }))
    .expect("turns should be state even when tools are blank");
    assert_eq!(turns_only.allowed_tools, None);
    assert_eq!(turns_only.max_turns, Some(2));
}

#[test]
// 显式空工具集与未提供工具状态语义不同，序列化时必须保留。
fn session_state_options_keeps_explicit_empty_tool_state() {
    let state = session_state_options(Some(&AcpSessionOptions {
        model: None,
        allowed_tools: Some(Vec::new()),
        max_turns: None,
    }))
    .expect("an explicitly supplied tool option creates session state");

    assert_eq!(state.model, None);
    assert_eq!(state.allowed_tools, None);
    assert_eq!(state.max_turns, None);
}

#[test]
fn missing_session_error_includes_search_boundary_and_create_command() {
    let named =
        assistant_message_text(missing_session_error("/repo", "claude-code", Some("session-a")));
    assert!(named.contains("searched up to /repo"));
    assert!(named.contains("vwacp claude-code sessions new --name session-a"));

    let unnamed = assistant_message_text(missing_session_error("/repo", "claude-code", None));
    assert!(unnamed.contains("vwacp claude-code sessions new"));
    assert!(!unnamed.contains("--name"));
}

#[test]
fn missing_session_error_treats_blank_name_as_unnamed() {
    let message =
        assistant_message_text(missing_session_error("/repo", "claude-code", Some(" \n\t ")));

    assert!(message.contains("vwacp claude-code sessions new"));
    assert!(!message.contains("--name"));
}

#[test]
fn acp_model_source_launch_profile_cache_never_contains_environment_secrets() {
    let cfg = config::schema::AcpAgentConfig {
        command: "agent".into(),
        env: std::collections::HashMap::from([("API_KEY".into(), "launch-secret-9".into())]),
        ..Default::default()
    };
    let first = super::acp_client_cache_key(
        "agent",
        &cfg,
        std::path::Path::new("/tmp"),
        &Default::default(),
    );
    assert!(!first.contains("launch-secret-9"));
    let mut rotated = cfg;
    rotated.env.insert("API_KEY".into(), "rotated-secret-9".into());
    assert_ne!(
        first,
        super::acp_client_cache_key(
            "agent",
            &rotated,
            std::path::Path::new("/tmp"),
            &Default::default()
        )
    );
}

#[tokio::test]
async fn agent_acp_defaults_reach_new_session_and_prompt() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let messages = serde_json::json!([
        {"role":"system","content":"agent rule"},
        {"role":"system","content":"<system-reminder>skill content</system-reminder>"},
        {"role":"user","content":"hello"}
    ]);
    for fresh in [false, true] {
        let prompt = super::super::replay::build_request_prompt(
            &messages,
            fresh,
            super::super::AcpReplayStrategy::Discard,
            3,
        );
        assert_eq!(prompt.matches("agent rule").count(), 1);
        assert_eq!(prompt.matches("skill content").count(), 1);
    }
    let temp = tempfile::tempdir().unwrap();
    let script = temp.path().join("peer.py");
    let capture = temp.path().join("requests.jsonl");
    std::fs::write(
        &script,
        r#"
import json, sys
for line in sys.stdin:
    r=json.loads(line)
    if 'id' not in r or 'method' not in r: continue
    with open(sys.argv[1],'a') as f: f.write(line)
    m=r['method']
    if m=='initialize': result={'protocolVersion':1,'agentCapabilities':{'loadSession':True,'sessionCapabilities':{'resume':{}}},'authMethods':[]}
    elif m=='session/new': result={'sessionId':'defaults'}
    elif m=='session/prompt':
        print(json.dumps({'jsonrpc':'2.0','id':900,'method':'fs/write_text_file','params':{'sessionId':'defaults','path':sys.argv[1]+'.denied','content':'must not write'}}),flush=True)
        permission=json.loads(sys.stdin.readline())
        if 'error' not in permission: raise Exception('permission default did not deny write')
        result={'stopReason':'end_turn'}
    else: result={}
    print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':result}),flush=True)
"#,
    )
    .unwrap();
    let cfg = config::schema::AcpAgentConfig {
        command: "python3".into(),
        args: vec![script.display().to_string(), capture.display().to_string()],
        env: Default::default(),
    };
    let mut options = ParsedAcpOptions {
        permission_mode: Some(PermissionMode::DenyAll),
        mcp_servers: vec![agent_client_protocol::McpServer::Stdio(
            agent_client_protocol::McpServerStdio::new("selected", "fixture-mcp"),
        )],
        ..Default::default()
    };
    let key = acp_client_cache_key("fixture", &cfg, temp.path(), &options);
    let client = build_cached_acp_client("fixture", &cfg, &options);
    options.mcp_servers.clear();
    assert_ne!(key, acp_client_cache_key("fixture", &cfg, temp.path(), &options));
    options.permission_mode = Some(PermissionMode::ApproveReads);
    let reads_key = acp_client_cache_key("fixture", &cfg, temp.path(), &options);
    options.permission_mode = Some(PermissionMode::DenyAll);
    assert_ne!(reads_key, acp_client_cache_key("fixture", &cfg, temp.path(), &options));
    let session = client.client.create_session(temp.path()).await.unwrap();
    client.client.load_session(session.session_id.clone(), temp.path()).await.unwrap();
    client.client.resume_session(session.session_id.clone(), temp.path()).await.unwrap();
    let prompt = super::super::replay::build_request_prompt(
        &messages,
        false,
        super::super::AcpReplayStrategy::Discard,
        3,
    );
    client
        .client
        .run_prompt(
            vw_acp::PromptRequest::new(temp.path().to_path_buf(), prompt)
                .with_session_strategy(vw_acp::SessionStrategy::Load(session.session_id)),
            &mut |_| {},
        )
        .await
        .unwrap();
    client.client.close().await.unwrap();
    assert!(!capture.with_extension("jsonl.denied").exists());
    std::fs::copy(&capture, "/tmp/shiju-unified-acp-6-requests.jsonl").unwrap();
    let requests: Vec<serde_json::Value> = std::fs::read_to_string(capture)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for method in ["session/new", "session/load", "session/resume"] {
        let request = requests.iter().find(|r| r["method"] == method).unwrap();
        assert_eq!(request["params"]["mcpServers"][0]["name"], "selected");
    }
    let prompt = requests.iter().find(|r| r["method"] == "session/prompt").unwrap();
    let text = prompt["params"]["prompt"][0]["text"].as_str().unwrap();
    assert_eq!(text.matches("agent rule").count(), 1);
    assert_eq!(text.matches("skill content").count(), 1);
}
