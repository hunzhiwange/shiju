use super::*;
use crate::app::agent::approval::{ApprovalManager, ApprovalResponse};
use crate::app::agent::config::{AutonomyConfig, ReliabilityConfig};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::{ChatMessage, Provider, ProviderRuntimeOptions};
use crate::app::agent::tools::{Tool, ToolResult};
use crate::approval::test_support::expire_non_cli_pending_request_for_tests;
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

struct NamedTool(&'static str);

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Tool for NamedTool {
    fn name(&self) -> &str {
        self.0
    }

    fn description(&self) -> &str {
        "unit test tool"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({"type": "object"})
    }

    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: "ok".to_string(), error: None })
    }
}

fn approval_config(auto_approve: &[&str]) -> AutonomyConfig {
    let mut config = AutonomyConfig::default();
    config.auto_approve = auto_approve.iter().map(|tool| (*tool).to_string()).collect();
    config.always_ask = Vec::new();
    config
}

fn boxed_tools(names: &[&'static str]) -> Arc<Vec<Box<dyn Tool>>> {
    Arc::new(names.iter().map(|name| Box::new(NamedTool(*name)) as Box<dyn Tool>).collect())
}

fn context(
    workspace_dir: &Path,
    tool_names: &[&'static str],
    auto_approve: &[&str],
    excluded_tools: &[&str],
) -> ChannelRuntimeContext {
    context_with_runtime_config(workspace_dir, None, tool_names, auto_approve, excluded_tools)
}

fn context_with_runtime_config(
    workspace_dir: &Path,
    vibewindow_dir: Option<PathBuf>,
    tool_names: &[&'static str],
    auto_approve: &[&str],
    excluded_tools: &[&str],
) -> ChannelRuntimeContext {
    ChannelRuntimeContext {
        channels_by_name: Arc::new(HashMap::new()),
        provider: Arc::new(StaticProvider),
        default_provider: Arc::new("default-provider".to_string()),
        memory: Arc::new(NoneMemory::new()),
        tools_registry: boxed_tools(tool_names),
        observer: Arc::new(NoopObserver),
        system_prompt: Arc::new("system".to_string()),
        model: Arc::new("default-model".to_string()),
        temperature: 0.2,
        auto_save_memory: false,
        max_iterations: 4,
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
        approval_manager: Arc::new(ApprovalManager::from_config(&approval_config(auto_approve))),
    }
}

fn runtime_config_file(root: &Path) -> PathBuf {
    root.join(".vibewindow.json")
}

fn config_with_approvals(auto_approve: &[&str], always_ask: &[&str]) -> Config {
    let mut config = Config::default();
    config.autonomy.auto_approve = auto_approve.iter().map(|tool| (*tool).to_string()).collect();
    config.autonomy.always_ask = always_ask.iter().map(|tool| (*tool).to_string()).collect();
    config
}

async fn write_json_config(path: &Path, config: &Config) {
    fs::write(path, serde_json::to_string_pretty(config).unwrap()).await.unwrap();
}

async fn read_json_value(path: &Path) -> Value {
    let contents = fs::read_to_string(path).await.unwrap();
    serde_json::from_str(&contents).unwrap()
}

fn create_pending(ctx: &ChannelRuntimeContext, tool_name: &str) -> String {
    ctx.approval_manager
        .create_non_cli_pending_request(
            tool_name,
            "sender",
            "telegram",
            "chat-1",
            Some("reason".to_string()),
            Value::Null,
            None,
            None,
        )
        .request_id
}

fn expire_pending(ctx: &ChannelRuntimeContext, request_id: &str) {
    expire_non_cli_pending_request_for_tests(&ctx.approval_manager, request_id);
}

#[test]
fn available_tools_preview_sorts_and_limits_to_twelve_tools() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(
        temp.path(),
        &["m", "a", "z", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"],
        &[],
        &[],
    );

    assert_eq!(available_tools_preview(&ctx), "a, b, c, d, e, f, g, h, i, j, k, m");
}

#[test]
fn request_all_tools_once_creates_scoped_pending_request() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path(), &["shell"], &[], &[]);

    let response = handle_request_all_tools_once(&ctx, "sender", "telegram", "chat-1");
    let pending = ctx.approval_manager.list_non_cli_pending_requests(
        Some("sender"),
        Some("telegram"),
        Some("chat-1"),
    );

    assert!(response.contains("One-time all-tools approval request created."));
    assert!(response.contains("/approve-confirm"));
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].tool_name, APPROVAL_ALL_TOOLS_ONCE_TOKEN);
}

#[test]
fn request_tool_approval_handles_usage_unknown_already_approved_and_pending() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path(), &["shell", "already"], &["already"], &[]);

    assert_eq!(
        handle_request_tool_approval(&ctx, "sender", "telegram", "chat-1", " ".to_string()),
        "Usage: `/approve-request <tool-name>`"
    );

    let unknown =
        handle_request_tool_approval(&ctx, "sender", "telegram", "chat-1", "missing".to_string());
    assert!(unknown.contains("Unknown tool `missing`"));
    assert!(unknown.contains("already, shell"));

    let approved =
        handle_request_tool_approval(&ctx, "sender", "telegram", "chat-1", "already".to_string());
    assert!(approved.contains("already approved"));

    let pending =
        handle_request_tool_approval(&ctx, "sender", "telegram", "chat-1", "shell".to_string());
    assert!(pending.contains("Approval request created."));
    assert!(pending.contains("Tool: `shell`"));
}

#[test]
fn approve_pending_request_handles_usage_not_found_mismatch_and_success() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path(), &["shell"], &[], &[]);
    let request_id = create_pending(&ctx, "shell");

    assert_eq!(
        handle_approve_pending_request(&ctx, "sender", "telegram", "chat-1", " ".to_string()),
        "Usage: `/approve-allow <request-id>`"
    );
    assert!(
        handle_approve_pending_request(
            &ctx,
            "sender",
            "telegram",
            "chat-1",
            "missing".to_string(),
        )
        .contains("was not found")
    );
    assert!(
        handle_approve_pending_request(&ctx, "other", "telegram", "chat-1", request_id.clone(),)
            .contains("can only be approved by the same sender")
    );

    let response =
        handle_approve_pending_request(&ctx, "sender", "telegram", "chat-1", request_id.clone());
    assert!(response.contains("Approved pending request"));
    assert_eq!(
        ctx.approval_manager.take_non_cli_pending_resolution(&request_id),
        Some(ApprovalResponse::Yes)
    );
}

#[test]
fn approve_pending_request_reports_expired_request() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path(), &["shell"], &[], &[]);
    let request_id = create_pending(&ctx, "shell");
    expire_pending(&ctx, &request_id);

    let response =
        handle_approve_pending_request(&ctx, "sender", "telegram", "chat-1", request_id.clone());

    assert_eq!(response, format!("Pending approval request `{request_id}` was not found."));
    assert_eq!(ctx.approval_manager.take_non_cli_pending_resolution(&request_id), None);
}

#[tokio::test]
async fn confirm_tool_approval_grants_all_tools_once_without_persistence() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path(), &["shell"], &[], &[]);
    let request_id = create_pending(&ctx, APPROVAL_ALL_TOOLS_ONCE_TOKEN);

    let response =
        handle_confirm_tool_approval(&ctx, "sender", "telegram", "chat-1", request_id.clone())
            .await;

    assert!(response.contains("Approved one-time all-tools bypass"));
    assert!(response.contains("runtime-only"));
    assert_eq!(ctx.approval_manager.non_cli_allow_all_once_remaining(), 1);
    assert_eq!(
        ctx.approval_manager.take_non_cli_pending_resolution(&request_id),
        Some(ApprovalResponse::Yes)
    );
}

#[tokio::test]
async fn confirm_tool_approval_grants_session_and_reports_excluded_tool() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path(), &["shell"], &[], &["shell"]);
    let request_id = create_pending(&ctx, "shell");

    let response =
        handle_confirm_tool_approval(&ctx, "sender", "telegram", "chat-1", request_id.clone())
            .await;

    assert!(response.contains("Approved supervised execution for `shell`"));
    assert!(response.contains("No runtime config path was found"));
    assert!(response.contains("currently listed in `autonomy.non_cli_excluded_tools`"));
    assert!(ctx.approval_manager.is_non_cli_session_granted("shell"));
    assert!(!ctx.approval_manager.needs_approval("shell"));
}

#[tokio::test]
async fn confirm_tool_approval_persists_config_when_runtime_path_exists() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = runtime_config_file(temp.path());
    write_json_config(&config_path, &config_with_approvals(&["existing"], &["shell"])).await;
    let ctx = context_with_runtime_config(
        temp.path(),
        Some(temp.path().to_path_buf()),
        &["shell"],
        &[],
        &[],
    );
    let request_id = create_pending(&ctx, "shell");

    let response =
        handle_confirm_tool_approval(&ctx, "sender", "telegram", "chat-1", request_id).await;

    assert!(response.contains("Persisted to"));
    assert!(response.contains(config_path.to_str().unwrap()));
    let persisted = read_json_value(&config_path).await;
    let auto_approve = persisted["autonomy"]["auto_approve"].as_array().unwrap();
    let always_ask = persisted["autonomy"]["always_ask"].as_array().unwrap();
    assert!(auto_approve.iter().any(|entry| entry == "existing"));
    assert!(auto_approve.iter().any(|entry| entry == "shell"));
    assert!(!always_ask.iter().any(|entry| entry == "shell"));
}

#[tokio::test]
async fn confirm_tool_approval_reports_persistence_error_after_in_memory_grant() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context_with_runtime_config(
        temp.path(),
        Some(temp.path().to_path_buf()),
        &["shell"],
        &[],
        &[],
    );
    let request_id = create_pending(&ctx, "shell");

    let response =
        handle_confirm_tool_approval(&ctx, "sender", "telegram", "chat-1", request_id).await;

    assert!(response.contains("Approved supervised execution for `shell` from request"));
    assert!(response.contains("Failed to persist this approval to config"));
    assert!(ctx.approval_manager.is_non_cli_session_granted("shell"));
}

#[tokio::test]
async fn confirm_tool_approval_reports_usage_and_pending_errors() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path(), &["shell"], &[], &[]);
    let request_id = create_pending(&ctx, "shell");

    assert_eq!(
        handle_confirm_tool_approval(&ctx, "sender", "telegram", "chat-1", " ".to_string()).await,
        "Usage: `/approve-confirm <request-id>`"
    );
    assert!(
        handle_confirm_tool_approval(&ctx, "sender", "telegram", "chat-1", "missing".to_string())
            .await
            .contains("was not found")
    );
    assert!(
        handle_confirm_tool_approval(&ctx, "other", "telegram", "chat-1", request_id)
            .await
            .contains("can only be confirmed by the same sender")
    );
}

#[tokio::test]
async fn confirm_tool_approval_reports_expired_request() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path(), &["shell"], &[], &[]);
    let request_id = create_pending(&ctx, "shell");
    expire_pending(&ctx, &request_id);

    let response =
        handle_confirm_tool_approval(&ctx, "sender", "telegram", "chat-1", request_id.clone())
            .await;

    assert_eq!(
        response,
        format!(
            "Pending approval request `{request_id}` was not found. Create one with `/approve-request <tool-name>` or `/approve-all-once`."
        )
    );
    assert_eq!(ctx.approval_manager.take_non_cli_pending_resolution(&request_id), None);
}

#[test]
fn deny_tool_approval_records_no_resolution_and_reports_errors() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path(), &["shell"], &[], &[]);
    let request_id = create_pending(&ctx, "shell");

    assert_eq!(
        handle_deny_tool_approval(&ctx, "sender", "telegram", "chat-1", " ".to_string()),
        "Usage: `/approve-deny <request-id>`"
    );
    assert!(
        handle_deny_tool_approval(&ctx, "sender", "telegram", "chat-1", "missing".to_string())
            .contains("was not found")
    );
    assert!(
        handle_deny_tool_approval(&ctx, "other", "telegram", "chat-1", request_id.clone())
            .contains("can only be denied by the same sender")
    );

    let response =
        handle_deny_tool_approval(&ctx, "sender", "telegram", "chat-1", request_id.clone());
    assert!(response.contains("Denied pending approval request"));
    assert_eq!(
        ctx.approval_manager.take_non_cli_pending_resolution(&request_id),
        Some(ApprovalResponse::No)
    );
}

#[test]
fn deny_tool_approval_reports_expired_request() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path(), &["shell"], &[], &[]);
    let request_id = create_pending(&ctx, "shell");
    expire_pending(&ctx, &request_id);

    let response =
        handle_deny_tool_approval(&ctx, "sender", "telegram", "chat-1", request_id.clone());

    assert_eq!(response, format!("Pending approval request `{request_id}` was not found."));
    assert_eq!(ctx.approval_manager.take_non_cli_pending_resolution(&request_id), None);
}

#[test]
fn list_pending_approvals_reports_empty_and_scoped_rows() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path(), &["shell"], &[], &[]);

    assert_eq!(
        handle_list_pending_approvals(&ctx, "sender", "telegram", "chat-1"),
        "No pending approval requests for your current sender+chat/channel scope."
    );

    let request_id = create_pending(&ctx, "shell");
    let response = handle_list_pending_approvals(&ctx, "sender", "telegram", "chat-1");
    assert!(response.contains("Pending approval requests"));
    assert!(response.contains(&request_id));
    assert!(response.contains("tool=shell"));
    assert!(response.contains("reason=reason"));
}

#[test]
fn list_pending_approvals_reports_blank_reason_as_na() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path(), &["shell"], &[], &[]);
    ctx.approval_manager.create_non_cli_pending_request(
        "shell",
        "sender",
        "telegram",
        "chat-1",
        Some(" \n ".to_string()),
        Value::Null,
        None,
        None,
    );

    let response = handle_list_pending_approvals(&ctx, "sender", "telegram", "chat-1");

    assert!(response.contains("reason=n/a"));
}

#[test]
fn list_pending_approvals_reports_missing_reason_as_na_without_leaking_other_scope() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path(), &["shell"], &[], &[]);
    let visible_request = ctx.approval_manager.create_non_cli_pending_request(
        "shell",
        "sender",
        "telegram",
        "chat-1",
        None,
        Value::Null,
        None,
        None,
    );
    let hidden_request = ctx.approval_manager.create_non_cli_pending_request(
        "shell",
        "other",
        "telegram",
        "chat-1",
        Some("other sender".to_string()),
        Value::Null,
        None,
        None,
    );

    let response = handle_list_pending_approvals(&ctx, "sender", "telegram", "chat-1");

    assert!(response.contains(&visible_request.request_id));
    assert!(response.contains("reason=n/a"));
    assert!(!response.contains(&hidden_request.request_id));
}

#[tokio::test]
async fn approve_tool_handles_usage_unknown_grant_clear_pending_and_exclusion_note() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path(), &["shell"], &[], &["shell"]);
    let request_id = create_pending(&ctx, "shell");

    assert_eq!(handle_approve_tool(&ctx, " ".to_string()).await, "Usage: `/approve <tool-name>`");
    assert!(handle_approve_tool(&ctx, "missing".to_string()).await.contains("Unknown tool"));

    let response = handle_approve_tool(&ctx, " shell ".to_string()).await;
    assert!(response.contains("Approved supervised execution for `shell`"));
    assert!(response.contains("No runtime config path was found"));
    assert!(response.contains("Runtime pending requests cleared: 1"));
    assert!(response.contains("currently listed in `autonomy.non_cli_excluded_tools`"));
    assert!(ctx.approval_manager.is_non_cli_session_granted("shell"));
    assert!(!ctx.approval_manager.has_non_cli_pending_request(&request_id));
}

#[tokio::test]
async fn approve_tool_persists_config_and_reports_no_exclusion_note() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = runtime_config_file(temp.path());
    write_json_config(&config_path, &config_with_approvals(&[], &["shell"])).await;
    let ctx = context_with_runtime_config(
        temp.path(),
        Some(temp.path().to_path_buf()),
        &["shell"],
        &[],
        &[],
    );

    let response = handle_approve_tool(&ctx, "shell".to_string()).await;

    assert!(response.contains("Persisted to"));
    assert!(response.contains("Runtime pending requests cleared: 0"));
    assert!(!response.contains("currently listed in `autonomy.non_cli_excluded_tools`"));
    let persisted = read_json_value(&config_path).await;
    let auto_approve = persisted["autonomy"]["auto_approve"].as_array().unwrap();
    let always_ask = persisted["autonomy"]["always_ask"].as_array().unwrap();
    assert!(auto_approve.iter().any(|entry| entry == "shell"));
    assert!(!always_ask.iter().any(|entry| entry == "shell"));
}

#[tokio::test]
async fn approve_tool_reports_persistence_error_after_runtime_grant() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context_with_runtime_config(
        temp.path(),
        Some(temp.path().to_path_buf()),
        &["shell"],
        &[],
        &[],
    );

    let response = handle_approve_tool(&ctx, "shell".to_string()).await;

    assert!(response.contains("Approved supervised execution for `shell` in-memory"));
    assert!(response.contains("Failed to persist this approval to config"));
    assert!(ctx.approval_manager.is_non_cli_session_granted("shell"));
}

#[tokio::test]
async fn unapprove_tool_handles_usage_and_revokes_runtime_session_without_config_path() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path(), &["shell"], &[], &[]);
    ctx.approval_manager.grant_non_cli_session("shell");
    ctx.approval_manager.apply_persistent_runtime_grant("shell");
    create_pending(&ctx, "shell");

    assert_eq!(
        handle_unapprove_tool(&ctx, " ".to_string()).await,
        "Usage: `/unapprove <tool-name>`"
    );

    let response = handle_unapprove_tool(&ctx, "shell".to_string()).await;
    assert!(response.contains("Runtime config path was not found"));
    assert!(response.contains("Runtime session grant removed for `shell`: yes"));
    assert!(!ctx.approval_manager.is_non_cli_session_granted("shell"));
    assert!(!ctx.approval_manager.has_non_cli_pending_request("shell"));
}

#[tokio::test]
async fn unapprove_tool_removes_persisted_config_and_runtime_state() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = runtime_config_file(temp.path());
    write_json_config(&config_path, &config_with_approvals(&["shell", "other"], &[])).await;
    let ctx = context_with_runtime_config(
        temp.path(),
        Some(temp.path().to_path_buf()),
        &["shell"],
        &[],
        &[],
    );
    ctx.approval_manager.grant_non_cli_session("shell");
    ctx.approval_manager.apply_persistent_runtime_grant("shell");
    let request_id = create_pending(&ctx, "shell");

    let response = handle_unapprove_tool(&ctx, "shell".to_string()).await;

    assert!(response.contains("Persistent approval removed for `shell`: yes"));
    assert!(response.contains("Runtime effective auto_approve removed: yes"));
    assert!(response.contains("Runtime pending requests cleared: 1"));
    assert!(response.contains("Runtime session grant removed: yes"));
    assert!(!ctx.approval_manager.has_non_cli_pending_request(&request_id));
    let persisted = read_json_value(&config_path).await;
    let auto_approve = persisted["autonomy"]["auto_approve"].as_array().unwrap();
    assert!(auto_approve.iter().any(|entry| entry == "other"));
    assert!(!auto_approve.iter().any(|entry| entry == "shell"));
}

#[tokio::test]
async fn unapprove_tool_reports_absent_persistent_entry_and_persistence_error() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = runtime_config_file(temp.path());
    write_json_config(&config_path, &config_with_approvals(&["other"], &[])).await;
    let ctx = context_with_runtime_config(
        temp.path(),
        Some(temp.path().to_path_buf()),
        &["shell"],
        &[],
        &[],
    );

    let response = handle_unapprove_tool(&ctx, "shell".to_string()).await;
    assert!(response.contains("Persistent approval removed for `shell`: no (not present)"));
    assert!(response.contains("Runtime effective auto_approve removed: no (not present)"));
    assert!(response.contains("Runtime session grant removed: no"));

    fs::write(&config_path, "{not-json").await.unwrap();
    let response = handle_unapprove_tool(&ctx, "shell".to_string()).await;
    assert!(response.contains("Removed runtime session grant for `shell`: no"));
    assert!(response.contains("Failed to persist removal to config"));
}

#[tokio::test]
async fn list_approvals_reports_runtime_state_without_config_path() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context(temp.path(), &["shell"], &["file_read"], &["shell"]);
    create_pending(&ctx, "shell");
    ctx.approval_manager.grant_non_cli_allow_all_once();

    let response = handle_list_approvals(&ctx, "sender", "telegram", "chat-1").await;

    assert!(response.contains("Supervised non-CLI tool approvals:"));
    assert!(response.contains("Runtime auto_approve (effective): file_read"));
    assert!(response.contains("Runtime one-time all-tools bypass tokens: 1"));
    assert!(response.contains("Runtime non_cli_excluded_tools: shell"));
    assert!(response.contains("Pending approvals"));
    assert!(response.contains("Persisted config approvals: unavailable"));
}

#[tokio::test]
async fn list_approvals_reports_config_read_error() {
    let temp = tempfile::tempdir().unwrap();
    let ctx = context_with_runtime_config(
        temp.path(),
        Some(temp.path().to_path_buf()),
        &["shell"],
        &[],
        &[],
    );

    let response = handle_list_approvals(&ctx, "sender", "telegram", "chat-1").await;

    assert!(response.contains("Failed to read approval state"));
    assert!(response.contains("Failed to read"));
}
