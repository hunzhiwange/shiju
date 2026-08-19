use super::*;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::{AutonomyConfig, ReliabilityConfig};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::{ChatMessage, Provider, ProviderRuntimeOptions};
use crate::app::agent::tools::Tool;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::fs;

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(message.to_string())
    }
}

fn autonomy_config(
    auto_approve: &[&str],
    always_ask: &[&str],
    approvers: &[&str],
    mode: NonCliNaturalLanguageApprovalMode,
    mode_by_channel: HashMap<String, NonCliNaturalLanguageApprovalMode>,
) -> AutonomyConfig {
    let mut config = AutonomyConfig::default();
    config.auto_approve = auto_approve.iter().map(|tool| (*tool).to_string()).collect();
    config.always_ask = always_ask.iter().map(|tool| (*tool).to_string()).collect();
    config.non_cli_approval_approvers =
        approvers.iter().map(|approver| (*approver).to_string()).collect();
    config.non_cli_natural_language_approval_mode = mode;
    config.non_cli_natural_language_approval_mode_by_channel = mode_by_channel;
    config
}

fn context(
    workspace_dir: &Path,
    vibewindow_dir: Option<PathBuf>,
    autonomy: AutonomyConfig,
    excluded_tools: &[&str],
) -> ChannelRuntimeContext {
    ChannelRuntimeContext {
        channels_by_name: Arc::new(HashMap::new()),
        provider: Arc::new(StaticProvider),
        default_provider: Arc::new("provider".to_string()),
        memory: Arc::new(NoneMemory::new()),
        tools_registry: Arc::new(Vec::<Box<dyn Tool>>::new()),
        observer: Arc::new(NoopObserver),
        system_prompt: Arc::new("system".to_string()),
        model: Arc::new("model".to_string()),
        temperature: 0.0,
        auto_save_memory: false,
        max_iterations: 1,
        min_relevance_score: 0.0,
        conversation_histories: Arc::new(Mutex::new(HashMap::<String, Vec<ChatMessage>>::new())),
        provider_cache: Arc::new(Mutex::new(HashMap::new())),
        route_overrides: Arc::new(Mutex::new(HashMap::new())),
        api_key: None,
        api_url: None,
        reliability: Arc::new(ReliabilityConfig::default()),
        provider_runtime_options: ProviderRuntimeOptions {
            vibewindow_dir,
            ..ProviderRuntimeOptions::default()
        },
        workspace_dir: Arc::new(PathBuf::from(workspace_dir)),
        message_timeout_secs: 30,
        interrupt_on_new_message: false,
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        hooks: None,
        non_cli_excluded_tools: Arc::new(Mutex::new(
            excluded_tools.iter().map(|tool| (*tool).to_string()).collect(),
        )),
        query_classification: crate::app::agent::config::QueryClassificationConfig::default(),
        model_routes: Vec::new(),
        approval_manager: Arc::new(ApprovalManager::from_config(&autonomy)),
    }
}

fn runtime_config_file(root: &Path) -> PathBuf {
    root.join(".vibewindow.json")
}

async fn write_json_config(path: &Path, config: &Config) {
    fs::write(path, serde_json::to_string_pretty(config).unwrap()).await.unwrap();
}

async fn read_json_value(path: &Path) -> Value {
    let contents = fs::read_to_string(path).await.unwrap();
    serde_json::from_str(&contents).unwrap()
}

fn config_with_approvals(auto_approve: &[&str], always_ask: &[&str]) -> Config {
    let mut config = Config::default();
    config.autonomy.auto_approve = auto_approve.iter().map(|tool| (*tool).to_string()).collect();
    config.autonomy.always_ask = always_ask.iter().map(|tool| (*tool).to_string()).collect();
    config
}

#[test]
fn natural_language_mode_label_covers_all_modes() {
    assert_eq!(
        non_cli_natural_language_mode_label(NonCliNaturalLanguageApprovalMode::Disabled),
        "disabled"
    );
    assert_eq!(
        non_cli_natural_language_mode_label(NonCliNaturalLanguageApprovalMode::RequestConfirm),
        "request_confirm"
    );
    assert_eq!(
        non_cli_natural_language_mode_label(NonCliNaturalLanguageApprovalMode::Direct),
        "direct"
    );
}

#[tokio::test]
async fn persist_and_remove_return_none_without_runtime_config_path() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        None,
        autonomy_config(&[], &[], &[], NonCliNaturalLanguageApprovalMode::Direct, HashMap::new()),
        &[],
    );

    assert_eq!(persist_non_cli_approval_to_config(&ctx, "shell").await.unwrap(), None);
    assert_eq!(remove_non_cli_approval_from_config(&ctx, "shell").await.unwrap(), None);
}

#[tokio::test]
async fn persist_adds_auto_approve_and_removes_always_ask_entries() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = runtime_config_file(tmp.path());
    write_json_config(&config_path, &config_with_approvals(&["existing"], &["shell", "other"]))
        .await;
    let ctx = context(
        tmp.path(),
        Some(tmp.path().to_path_buf()),
        autonomy_config(&[], &[], &[], NonCliNaturalLanguageApprovalMode::Direct, HashMap::new()),
        &[],
    );

    let persisted_path = persist_non_cli_approval_to_config(&ctx, "shell").await.unwrap();
    assert_eq!(persisted_path, Some(config_path.clone()));

    let persisted = read_json_value(&config_path).await;
    let auto_approve = persisted["autonomy"]["auto_approve"].as_array().unwrap();
    let always_ask = persisted["autonomy"]["always_ask"].as_array().unwrap();
    assert!(auto_approve.iter().any(|entry| entry == "existing"));
    assert!(auto_approve.iter().any(|entry| entry == "shell"));
    assert!(always_ask.iter().any(|entry| entry == "other"));
    assert!(!always_ask.iter().any(|entry| entry == "shell"));

    let before = fs::read_to_string(&config_path).await.unwrap();
    assert_eq!(
        persist_non_cli_approval_to_config(&ctx, "shell").await.unwrap(),
        Some(config_path.clone())
    );
    let after = fs::read_to_string(&config_path).await.unwrap();
    assert_eq!(after, before);
}

#[tokio::test]
async fn persist_removes_always_ask_when_tool_is_already_auto_approved() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = runtime_config_file(tmp.path());
    write_json_config(
        &config_path,
        &config_with_approvals(&["shell", "existing"], &["shell", "other"]),
    )
    .await;
    let ctx = context(
        tmp.path(),
        Some(tmp.path().to_path_buf()),
        autonomy_config(&[], &[], &[], NonCliNaturalLanguageApprovalMode::Direct, HashMap::new()),
        &[],
    );

    let persisted_path = persist_non_cli_approval_to_config(&ctx, "shell").await.unwrap();
    assert_eq!(persisted_path, Some(config_path.clone()));

    let persisted = read_json_value(&config_path).await;
    assert_eq!(persisted["autonomy"]["auto_approve"], serde_json::json!(["shell", "existing"]));
    assert_eq!(persisted["autonomy"]["always_ask"], serde_json::json!(["other"]));
}

#[tokio::test]
async fn persist_supports_leading_whitespace_json_config() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = runtime_config_file(tmp.path());
    let json =
        serde_json::to_string_pretty(&config_with_approvals(&["existing"], &["shell"])).unwrap();
    fs::write(&config_path, format!("\n  {json}")).await.unwrap();
    let ctx = context(
        tmp.path(),
        Some(tmp.path().to_path_buf()),
        autonomy_config(&[], &[], &[], NonCliNaturalLanguageApprovalMode::Direct, HashMap::new()),
        &[],
    );

    let persisted_path = persist_non_cli_approval_to_config(&ctx, "shell").await.unwrap();
    assert_eq!(persisted_path, Some(config_path.clone()));

    let persisted = read_json_value(&config_path).await;
    let auto_approve = persisted["autonomy"]["auto_approve"].as_array().unwrap();
    let always_ask = persisted["autonomy"]["always_ask"].as_array().unwrap();
    assert!(auto_approve.iter().any(|entry| entry == "existing"));
    assert!(auto_approve.iter().any(|entry| entry == "shell"));
    assert!(!always_ask.iter().any(|entry| entry == "shell"));
}

#[tokio::test]
async fn persist_and_remove_accept_toml_config_payloads() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = runtime_config_file(tmp.path());
    fs::write(
        &config_path,
        r#"
default_temperature = 0.0

[autonomy]
level = "supervised"
workspace_only = true
allowed_commands = []
forbidden_paths = []
max_actions_per_hour = 1
max_cost_per_day_cents = 0
auto_approve = ["existing"]
always_ask = ["shell", "other"]
"#,
    )
    .await
    .unwrap();
    let ctx = context(
        tmp.path(),
        Some(tmp.path().to_path_buf()),
        autonomy_config(&[], &[], &[], NonCliNaturalLanguageApprovalMode::Direct, HashMap::new()),
        &[],
    );

    let persisted_path = persist_non_cli_approval_to_config(&ctx, "shell").await.unwrap();
    assert_eq!(persisted_path, Some(config_path.clone()));

    let persisted = read_json_value(&config_path).await;
    let auto_approve = persisted["autonomy"]["auto_approve"].as_array().unwrap();
    let always_ask = persisted["autonomy"]["always_ask"].as_array().unwrap();
    assert!(auto_approve.iter().any(|entry| entry == "existing"));
    assert!(auto_approve.iter().any(|entry| entry == "shell"));
    assert!(always_ask.iter().any(|entry| entry == "other"));
    assert!(!always_ask.iter().any(|entry| entry == "shell"));

    let removed = remove_non_cli_approval_from_config(&ctx, "shell").await.unwrap();
    assert_eq!(removed, Some((config_path.clone(), true)));

    let persisted = read_json_value(&config_path).await;
    let auto_approve = persisted["autonomy"]["auto_approve"].as_array().unwrap();
    let always_ask = persisted["autonomy"]["always_ask"].as_array().unwrap();
    assert!(auto_approve.iter().any(|entry| entry == "existing"));
    assert!(!auto_approve.iter().any(|entry| entry == "shell"));
    assert!(always_ask.iter().any(|entry| entry == "other"));
}

#[tokio::test]
async fn remove_deletes_auto_approve_only_when_present() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = runtime_config_file(tmp.path());
    write_json_config(&config_path, &config_with_approvals(&["shell", "other"], &[])).await;
    let ctx = context(
        tmp.path(),
        Some(tmp.path().to_path_buf()),
        autonomy_config(&[], &[], &[], NonCliNaturalLanguageApprovalMode::Direct, HashMap::new()),
        &[],
    );

    let removed = remove_non_cli_approval_from_config(&ctx, "shell").await.unwrap();
    assert_eq!(removed, Some((config_path.clone(), true)));

    let persisted = read_json_value(&config_path).await;
    let auto_approve = persisted["autonomy"]["auto_approve"].as_array().unwrap();
    assert!(auto_approve.iter().any(|entry| entry == "other"));
    assert!(!auto_approve.iter().any(|entry| entry == "shell"));

    let before = fs::read_to_string(&config_path).await.unwrap();
    let removed = remove_non_cli_approval_from_config(&ctx, "missing").await.unwrap();
    assert_eq!(removed, Some((config_path.clone(), false)));
    let after = fs::read_to_string(&config_path).await.unwrap();
    assert_eq!(after, before);
}

#[tokio::test]
async fn config_persistence_reports_read_and_parse_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        Some(tmp.path().to_path_buf()),
        autonomy_config(&[], &[], &[], NonCliNaturalLanguageApprovalMode::Direct, HashMap::new()),
        &[],
    );

    let err = persist_non_cli_approval_to_config(&ctx, "shell").await.unwrap_err();
    assert!(err.to_string().contains("Failed to read"));
    let err = remove_non_cli_approval_from_config(&ctx, "shell").await.unwrap_err();
    assert!(err.to_string().contains("Failed to read"));
    let err = describe_non_cli_approvals(&ctx, "sender", "telegram", "chat-1").await.unwrap_err();
    assert!(err.to_string().contains("Failed to read"));

    fs::write(runtime_config_file(tmp.path()), "{not-json").await.unwrap();
    let err = describe_non_cli_approvals(&ctx, "sender", "telegram", "chat-1").await.unwrap_err();
    assert!(err.to_string().contains("Failed to parse"));

    fs::write(runtime_config_file(tmp.path()), "autonomy = [").await.unwrap();
    let err = remove_non_cli_approval_from_config(&ctx, "shell").await.unwrap_err();
    assert!(err.to_string().contains("Failed to parse"));
}

#[tokio::test]
async fn describe_approvals_reports_empty_runtime_state_without_config_path() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        None,
        autonomy_config(&[], &[], &[], NonCliNaturalLanguageApprovalMode::Direct, HashMap::new()),
        &[],
    );

    let response = describe_non_cli_approvals(&ctx, "sender", "telegram", "chat-1").await.unwrap();

    assert!(response.contains("Supervised non-CLI tool approvals:"));
    assert!(response.contains("Runtime auto_approve (effective): (none)"));
    assert!(response.contains("Runtime always_ask (effective): (none)"));
    assert!(response.contains("Runtime session grants: (none)"));
    assert!(response.contains("Runtime one-time all-tools bypass tokens: 0"));
    assert!(response.contains("Runtime non_cli_approval_approvers: (any channel-allowed sender)"));
    assert!(response.contains("Runtime non_cli_natural_language_approval_mode: direct"));
    assert!(response.contains("Runtime non_cli_natural_language_approval_mode_by_channel: (none)"));
    assert!(response.contains("Pending approvals (sender+chat/channel scoped): (none)"));
    assert!(response.contains("Runtime non_cli_excluded_tools: (none)"));
    assert!(response.contains("Persisted config approvals: unavailable"));
}

#[tokio::test]
async fn describe_approvals_reports_empty_persisted_state_with_config_path() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = runtime_config_file(tmp.path());
    write_json_config(&config_path, &config_with_approvals(&[], &[])).await;
    let ctx = context(
        tmp.path(),
        Some(tmp.path().to_path_buf()),
        autonomy_config(&[], &[], &[], NonCliNaturalLanguageApprovalMode::Direct, HashMap::new()),
        &[],
    );

    let response = describe_non_cli_approvals(&ctx, "sender", "telegram", "chat-1").await.unwrap();

    assert!(response.contains("Persisted autonomy.auto_approve: (none)"));
    assert!(response.contains("Persisted autonomy.always_ask: (none)"));
    assert!(response.contains(&format!("Config path: {}", config_path.display())));
}

#[tokio::test]
async fn describe_approvals_reads_toml_persisted_state() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = runtime_config_file(tmp.path());
    fs::write(
        &config_path,
        r#"
default_temperature = 0.0

[autonomy]
level = "supervised"
workspace_only = true
allowed_commands = []
forbidden_paths = []
max_actions_per_hour = 0
max_cost_per_day_cents = 0
auto_approve = ["toml_b", "toml_a"]
always_ask = ["ask_b", "ask_a"]
"#,
    )
    .await
    .unwrap();
    let ctx = context(
        tmp.path(),
        Some(tmp.path().to_path_buf()),
        autonomy_config(&[], &[], &[], NonCliNaturalLanguageApprovalMode::Direct, HashMap::new()),
        &[],
    );

    let response = describe_non_cli_approvals(&ctx, "sender", "telegram", "chat-1").await.unwrap();

    assert!(response.contains("Persisted autonomy.auto_approve: toml_a, toml_b"));
    assert!(response.contains("Persisted autonomy.always_ask: ask_a, ask_b"));
    assert!(response.contains(&format!("Config path: {}", config_path.display())));
}

#[tokio::test]
async fn describe_approvals_reports_runtime_pending_excluded_and_persisted_state() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = runtime_config_file(tmp.path());
    write_json_config(
        &config_path,
        &config_with_approvals(&["persist_b", "persist_a"], &["ask_b"]),
    )
    .await;
    let mut mode_by_channel = HashMap::new();
    mode_by_channel.insert("telegram".to_string(), NonCliNaturalLanguageApprovalMode::Direct);
    mode_by_channel.insert("discord".to_string(), NonCliNaturalLanguageApprovalMode::Disabled);
    let ctx = context(
        tmp.path(),
        Some(tmp.path().to_path_buf()),
        autonomy_config(
            &["runtime_b"],
            &["runtime_a"],
            &["telegram:sender", "admin"],
            NonCliNaturalLanguageApprovalMode::RequestConfirm,
            mode_by_channel,
        ),
        &["shell", "browser"],
    );
    ctx.approval_manager.grant_non_cli_session("session_tool");
    ctx.approval_manager.grant_non_cli_allow_all_once();
    ctx.approval_manager.create_non_cli_pending_request(
        "shell",
        "sender",
        "telegram",
        "chat-1",
        Some("   ".to_string()),
        Value::Null,
        None,
        None,
    );
    ctx.approval_manager.create_non_cli_pending_request(
        "browser",
        "sender",
        "telegram",
        "chat-1",
        Some("because".to_string()),
        Value::Null,
        None,
        None,
    );

    let response = describe_non_cli_approvals(&ctx, "sender", "telegram", "chat-1").await.unwrap();

    assert!(response.contains("Runtime auto_approve (effective): runtime_b"));
    assert!(response.contains("Runtime always_ask (effective): runtime_a"));
    assert!(response.contains("Runtime session grants: session_tool"));
    assert!(response.contains("Runtime one-time all-tools bypass tokens: 1"));
    assert!(response.contains("Runtime non_cli_approval_approvers: admin, telegram:sender"));
    assert!(response.contains("Runtime non_cli_natural_language_approval_mode: request_confirm"));
    assert!(response.contains(
        "Runtime non_cli_natural_language_approval_mode (current channel `telegram`): direct"
    ));
    assert!(response.contains(
        "Runtime non_cli_natural_language_approval_mode_by_channel: discord=disabled, telegram=direct"
    ));
    assert!(response.contains("Pending approvals (sender+chat/channel scoped):"));
    assert!(response.contains("tool=shell"));
    assert!(response.contains("tool=browser"));
    assert!(response.contains("reason=n/a"));
    assert!(response.contains("reason=because"));
    assert!(response.contains("Runtime non_cli_excluded_tools: browser, shell"));
    assert!(response.contains("Persisted autonomy.auto_approve: persist_a, persist_b"));
    assert!(response.contains("Persisted autonomy.always_ask: ask_b"));
    assert!(response.contains(&format!("Config path: {}", config_path.display())));
}

#[tokio::test]
async fn describe_approvals_filters_pending_requests_to_current_scope() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = runtime_config_file(tmp.path());
    write_json_config(&config_path, &config_with_approvals(&[], &[])).await;
    let ctx = context(
        tmp.path(),
        Some(tmp.path().to_path_buf()),
        autonomy_config(&[], &[], &[], NonCliNaturalLanguageApprovalMode::Direct, HashMap::new()),
        &[],
    );
    ctx.approval_manager.create_non_cli_pending_request(
        APPROVAL_ALL_TOOLS_ONCE_TOKEN,
        "sender",
        "telegram",
        "chat-1",
        Some("allow all".to_string()),
        Value::Null,
        None,
        None,
    );
    ctx.approval_manager.create_non_cli_pending_request(
        "shell",
        "other-sender",
        "telegram",
        "chat-1",
        Some("wrong sender".to_string()),
        Value::Null,
        None,
        None,
    );
    ctx.approval_manager.create_non_cli_pending_request(
        "browser",
        "sender",
        "telegram",
        "chat-2",
        Some("wrong chat".to_string()),
        Value::Null,
        None,
        None,
    );

    let response = describe_non_cli_approvals(&ctx, "sender", "telegram", "chat-1").await.unwrap();

    assert!(response.contains("tool=all tools/commands (one-time bypass token)"));
    assert!(response.contains("reason=allow all"));
    assert!(!response.contains("tool=shell"));
    assert!(!response.contains("tool=browser"));
}
