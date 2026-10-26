// 本文件验证渠道消息执行的会话权限、可靠性快照、取消与超时结果。

use super::*;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::AutonomyConfig;
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::Provider;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("ok".to_string())
    }
}

fn context(workspace_dir: PathBuf) -> ChannelRuntimeContext {
    let provider: Arc<dyn Provider> = Arc::new(StaticProvider);
    ChannelRuntimeContext {
        project_state: Default::default(),
        message_snapshot: None,
        session_full_access: true,
        channels_by_name: Arc::new(HashMap::new()),
        provider: Arc::clone(&provider),
        default_provider: Arc::new("provider".to_string()),
        memory: Arc::new(NoneMemory::new()),
        tools_registry: Arc::new(Vec::new()),
        observer: Arc::new(NoopObserver),
        system_prompt: Arc::new("system".to_string()),
        model: Arc::new("model".to_string()),
        temperature: 0.0,
        auto_save_memory: false,
        max_iterations: 1,
        min_relevance_score: 0.0,
        conversation_histories: Arc::new(Mutex::new(HashMap::new())),
        provider_cache: Arc::new(Mutex::new(HashMap::new())),
        route_overrides: Arc::new(Mutex::new(HashMap::new())),
        api_key: None,
        api_url: None,
        reliability: Arc::new(crate::app::agent::config::ReliabilityConfig::default()),
        provider_runtime_options: crate::app::agent::providers::ProviderRuntimeOptions::default(),
        workspace_dir: Arc::new(workspace_dir),
        message_timeout_secs: 30,
        interrupt_on_new_message: false,
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        hooks: None,
        non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),
        query_classification: crate::app::agent::config::QueryClassificationConfig::default(),
        model_routes: Vec::new(),
        approval_manager: Arc::new(ApprovalManager::from_config(&AutonomyConfig::default())),
    }
}

fn message(sender: &str) -> traits::ChannelMessage {
    traits::ChannelMessage {
        instance_id: "default".to_string(),
        id: "msg".to_string(),
        sender: sender.to_string(),
        reply_target: "reply".to_string(),
        content: "hello".to_string(),
        channel: "test".to_string(),
        timestamp: 1,
        thread_ts: None,
    }
}

fn setup() -> (TempDir, ChannelRuntimeContext, traits::ChannelMessage) {
    let workspace = tempfile::tempdir().expect("workspace temp dir");
    let sender = format!("sender-{}", std::process::id());
    let ctx = context(workspace.path().to_path_buf());
    let msg = message(&sender);
    (workspace, ctx, msg)
}

#[test]
fn channel_session_options_enable_full_access() {
    let route = ChannelRouteSelection {
        provider: "provider".into(),
        model: "model".into(),
        reasoning_effort: None,
        task_mode_enabled: false,
    };
    let options = channel_session_options(&route, true);

    assert_eq!(options.get("full_access").and_then(serde_json::Value::as_bool), Some(true));
}

#[test]
fn channel_request_uses_runtime_reliability_snapshot() {
    let workspace = tempfile::tempdir().expect("workspace temp dir");
    let mut ctx = context(workspace.path().to_path_buf());
    ctx.reliability = Arc::new(crate::app::agent::config::ReliabilityConfig {
        provider_retries: 8,
        provider_backoff_ms: 1_750,
        ..Default::default()
    });

    let reliability = channel_request_reliability(&ctx);
    assert_eq!(reliability.provider_retries, 8);
    assert_eq!(reliability.provider_backoff_ms, 1_750);
}

async fn cleanup_sender_session(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) {
    let _ = clear_sender_session_id(ctx, msg).await;
}

#[tokio::test]
// 预先取消必须在启动提供者前短路，避免产生副作用。
async fn run_message_execution_returns_cancelled_when_token_is_already_cancelled() {
    let (_workspace, ctx, msg) = setup();
    let route = ChannelRouteSelection {
        provider: "provider".to_string(),
        model: "model".to_string(),
        reasoning_effort: None,
        task_mode_enabled: false,
    };
    let cancellation = CancellationToken::new();
    cancellation.cancel();

    crate::project::from_directory(
        &crate::session::ui_store::current_tenant_scope(),
        ctx.workspace_dir.as_ref(),
    )
    .await
    .unwrap();
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let result =
        run_message_execution(&ctx, &msg, &route, &snapshot, Vec::new(), None, 30, &cancellation)
            .await;

    assert!(matches!(result, LlmExecutionResult::Cancelled));
    cleanup_sender_session(&ctx, &msg).await;
}

#[tokio::test]
async fn run_message_execution_reports_channel_budget_as_completed_error() {
    let (_workspace, ctx, msg) = setup();
    let route = ChannelRouteSelection {
        provider: "provider".to_string(),
        model: "model".to_string(),
        reasoning_effort: None,
        task_mode_enabled: false,
    };
    let cancellation = CancellationToken::new();

    crate::project::from_directory(
        &crate::session::ui_store::current_tenant_scope(),
        ctx.workspace_dir.as_ref(),
    )
    .await
    .unwrap();
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let result =
        run_message_execution(&ctx, &msg, &route, &snapshot, Vec::new(), None, 0, &cancellation)
            .await;

    assert!(
        matches!(result, LlmExecutionResult::Completed(Ok(Err(error))) if error.to_string().contains("channel_message_timeout_budget"))
    );
    cleanup_sender_session(&ctx, &msg).await;
}

#[test]
fn channel_command_catalog_effort_is_forwarded_without_permission_changes() {
    let mut route = ChannelRouteSelection {
        provider: "provider".into(),
        model: "model".into(),
        reasoning_effort: None,
        task_mode_enabled: false,
    };
    let defaults = channel_session_options(&route, true);
    route.reasoning_effort = Some("high".into());
    let mut selected = channel_session_options(&route, true);
    assert_eq!(selected["reasoning_effort"], "high");
    selected.as_object_mut().unwrap().remove("reasoning_effort");
    assert_eq!(selected, defaults);
}

#[test]
fn channel_command_cancel_webhook_preserves_supervised_permissions() {
    let route = ChannelRouteSelection {
        provider: "provider".into(),
        model: "model".into(),
        reasoning_effort: None,
        task_mode_enabled: false,
    };
    assert_eq!(channel_session_options(&route, false)["full_access"], false);
}

#[tokio::test]
async fn shell_outer_timeout_explicit_budget_is_reported() {
    use crate::tools::shell::processes::{ShellProcessScope, ShellProcessState, ShellProcesses};
    let (_workspace, ctx, msg) = setup();
    crate::project::from_directory(
        &crate::session::ui_store::current_tenant_scope(),
        ctx.workspace_dir.as_ref(),
    )
    .await
    .unwrap();
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let session = resolve_or_create_sender_session_id(&snapshot, &msg).await.unwrap();
    let scope = ShellProcessScope::new(
        serde_json::to_string(&snapshot.scope.chat.tenant).unwrap(),
        &snapshot.directory,
        session,
    )
    .unwrap();
    let host = ShellProcesses::shared().unwrap();
    let started = host
        .start(scope.clone(), || {
            let mut command = tokio::process::Command::new("sh");
            command.args(["-c", "printf budget-ready; exec sleep 60"]);
            Ok(command)
        })
        .await
        .unwrap();
    let ready = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let value = host
                .wait(&scope, &started.process_id, 0, Duration::from_millis(100))
                .await
                .unwrap();
            if value.output.contains("budget-ready") {
                break value;
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(ready.state, ShellProcessState::Running);
    let route = ChannelRouteSelection {
        provider: "provider".into(),
        model: "model".into(),
        reasoning_effort: None,
        task_mode_enabled: false,
    };
    let result = run_message_execution(
        &ctx,
        &msg,
        &route,
        &snapshot,
        Vec::new(),
        None,
        0,
        &CancellationToken::new(),
    )
    .await;
    let after = host.snapshot(&scope, &started.process_id).unwrap();
    // Clean up even when the regression assertion fails.
    host.cancel_scope(&scope).unwrap();
    cleanup_sender_session(&ctx, &msg).await;
    assert_eq!(after.state, ShellProcessState::Cancelled, "channel budget must reap its processes");
    assert!(after.output.contains("budget-ready"), "tail output remains readable after reaping");
    let LlmExecutionResult::Completed(Ok(Err(error))) = result else {
        panic!("expected a channel budget diagnostic distinct from model idle and user stop");
    };
    let report = error.to_string();
    assert!(report.contains("channel_message_timeout_budget"), "{report}");
    assert!(report.contains("budget_secs=0"), "{report}");
    assert!(report.contains("elapsed_ms="), "{report}");
    assert!(report.contains("reaped"), "{report}");
    eprintln!(
        "{report}; process={} state={:?} exit_code={:?}; retained tail verified",
        after.process_id, after.state, after.exit_code
    );
}

#[tokio::test]
async fn shell_outer_timeout_user_stop_preserves_other_sessions() {
    use crate::tools::shell::processes::{ShellProcessScope, ShellProcessState, ShellProcesses};
    let (_workspace, ctx, msg) = setup();
    crate::project::from_directory(
        &crate::session::ui_store::current_tenant_scope(),
        ctx.workspace_dir.as_ref(),
    )
    .await
    .unwrap();
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let tenant = serde_json::to_string(&snapshot.scope.chat.tenant).unwrap();
    let scope =
        ShellProcessScope::new(tenant.clone(), &snapshot.directory, "channel-stop".into()).unwrap();
    let foreign =
        ShellProcessScope::new(tenant, &snapshot.directory, "other-channel-session".into())
            .unwrap();
    let host = ShellProcesses::shared().unwrap();
    let mut ids = Vec::new();
    for owner in [&scope, &foreign] {
        let started = host
            .start(owner.clone(), || {
                let mut command = tokio::process::Command::new("sh");
                command.args(["-c", "printf stop-ready; exec sleep 60"]);
                Ok(command)
            })
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let value = host
                    .wait(owner, &started.process_id, 0, Duration::from_millis(100))
                    .await
                    .unwrap();
                if value.output.contains("stop-ready") {
                    break;
                }
            }
        })
        .await
        .unwrap();
        ids.push(started.process_id);
    }
    let abort = CancellationToken::new();
    stop_channel_execution(
        std::future::ready(Ok(String::new())),
        &abort,
        &snapshot,
        "channel-stop",
    )
    .await
    .unwrap();
    let own_state = host.snapshot(&scope, &ids[0]).unwrap().state;
    let foreign_state = host.snapshot(&foreign, &ids[1]).unwrap().state;
    host.cancel_scope(&foreign).unwrap();
    assert!(abort.is_cancelled());
    assert_eq!(own_state, ShellProcessState::Cancelled);
    assert_eq!(foreign_state, ShellProcessState::Running);
    eprintln!(
        "channel user stop reaped {}; other session {} stayed Running and was cleaned up separately",
        ids[0], ids[1]
    );
}

#[tokio::test(start_paused = true)]
async fn shell_outer_timeout_blocked_delivery_has_bounded_drain() {
    let (_workspace, ctx, msg) = setup();
    crate::project::from_directory(
        &crate::session::ui_store::current_tenant_scope(),
        ctx.workspace_dir.as_ref(),
    )
    .await
    .unwrap();
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let abort = CancellationToken::new();
    let started = tokio::time::Instant::now();
    stop_channel_execution(std::future::pending(), &abort, &snapshot, "blocked-delivery")
        .await
        .unwrap();
    assert!(abort.is_cancelled());
    assert_eq!(started.elapsed(), Duration::from_secs(5));
}
