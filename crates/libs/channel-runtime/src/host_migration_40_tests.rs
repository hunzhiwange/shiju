use crate::command_test_support::*;
async fn dispatch(ctx: &ChannelRuntimeContext, channel: &Arc<dyn Channel>, content: &str) -> bool {
    handle_runtime_command_if_needed(ctx, &message(content), Some(channel)).await
}

async fn dispatch_message(
    ctx: &ChannelRuntimeContext,
    channel: &Arc<dyn Channel>,
    msg: &traits::ChannelMessage,
) -> bool {
    handle_runtime_command_if_needed(ctx, msg, Some(channel)).await
}


use std::{future::Future, pin::Pin};
use anyhow::Result;
use channel_support::{scope::ChannelMessageSnapshot, routing::ChannelRouteSelection};
use crate::{host::ScopedFuture, command_host::SessionActivity};
// Approval/help tests must never invoke session storage or task submission.
struct CommandPort;
#[async_trait::async_trait]
#[allow(unused_variables)]
impl crate::command_host::CommandHost for CommandPort {
fn command_tenant(&self) -> vw_api_transport::id::TenantScope { Default::default() }
fn prepare_catalog(&self) {  }
fn command_now_ms(&self) -> u64 { panic!("unexpected external command operation") }
fn session_activity(&self, id: &str) -> SessionActivity { panic!("unexpected external command operation") }
async fn in_project_scope(&self, snapshot: &ChannelMessageSnapshot, operation: ScopedFuture<'_>) { operation.await }
async fn in_session_scope(&self, snapshot: &ChannelMessageSnapshot, operation: ScopedFuture<'_>) { operation.await }
fn get_session<'a>(&'a self, id: &'a str) -> Pin<Box<dyn Future<Output = Result<session_info::Info, session_info::Error>> + Send + 'a>> { panic!("unexpected external command operation") }
async fn get_scoped_session(&self, id: &str) -> Result<session_info::Info, session_info::Error> { panic!("unexpected external command operation") }
#[cfg(not(target_arch = "wasm32"))]
async fn reserve_session(&self, snapshot: &ChannelMessageSnapshot) -> Result<session_info::Info> { panic!("unexpected external command operation") }
#[cfg(not(target_arch = "wasm32"))]
async fn discard_session(&self, snapshot: &ChannelMessageSnapshot, info: session_info::Info) -> Result<()> { panic!("unexpected external command operation") }
async fn set_session_title(&self, id: &str, title: &str) -> Result<()> { panic!("unexpected external command operation") }
async fn set_session_archived(&self, id: &str, archived: Option<u64>) -> Result<()> { panic!("unexpected external command operation") }
async fn fork_session(&self, id: &str, anchor: Option<&str>) -> Result<session_info::Info> { panic!("unexpected external command operation") }
async fn remove_session(&self, id: &str) -> Result<()> { panic!("unexpected external command operation") }
async fn revert_session(&self, id: &str, message: &str, part: Option<&str>) -> Result<session_info::Info> { panic!("unexpected external command operation") }
async fn cleanup_revert(&self, info: &session_info::Info) -> Result<()> { panic!("unexpected external command operation") }
async fn compact_session(&self, id: &str, anchor: &str, auto: bool) -> Result<session_engine::compaction::CompactionOutcome> { panic!("unexpected external command operation") }
async fn load_ui_session(&self, snapshot: &ChannelMessageSnapshot, id: &str) -> Result<Option<session_port::ui_types::ChatSession>> { panic!("unexpected external command operation") }
#[cfg(not(target_arch = "wasm32"))]
async fn load_session_preview(&self, snapshot: &ChannelMessageSnapshot, id: &str) -> Result<Option<String>> { panic!("unexpected external command operation") }
fn create_task(&self, project: &str, priority: u32, model: Option<&str>, prompt: &str) -> std::io::Result<String> { panic!("unexpected external command operation") }
}
async fn handle_runtime_command_if_needed(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage, channel: Option<&Arc<dyn Channel>>) -> bool {
    crate::runtime_command::handler::handle_runtime_command_if_needed(&CommandPort, ctx, msg, channel).await
}

async fn help_reply(
    platform: &str,
    content: &str,
    restricted: bool,
) -> (Vec<SendMessage>, ChannelRuntimeContext) {
    let tmp = tempfile::tempdir().unwrap();
    let mut ctx = context(
        tmp.path(),
        &[],
        autonomy_config(
            &[],
            if restricted { &["lark:private-admin"] } else { &[] },
            NonCliNaturalLanguageApprovalMode::Direct,
        ),
    );
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    ctx.provider = Arc::new(CountingProvider(calls.clone()));
    let channel = Arc::new(RecordingChannel::new());
    let target: Arc<dyn Channel> = channel.clone();
    let mut msg = message_in_channel(platform, content);
    msg.instance_id = "work".into();
    assert!(handle_runtime_command_if_needed(&ctx, &msg, Some(&target)).await);
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    let sent = channel.sent.lock().await.clone();
    (sent, ctx)
}

#[test]
fn runtime_command_parser_keeps_model_switch_payload() {
    assert_eq!(
        parse_runtime_command("telegram", "/model gpt-4.1"),
        Some(ChannelRuntimeCommand::SetModel("gpt-4.1".to_string()))
    );
}

#[tokio::test]
async fn non_command_returns_false_when_task_mode_is_off() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        &[],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Direct),
    );
    let channel = Arc::new(RecordingChannel::new());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    assert!(
        !handle_runtime_command_if_needed(&ctx, &message("ordinary chat"), Some(&channel_dyn))
            .await
    );
    assert!(channel.sent_contents().await.is_empty());
}

#[tokio::test]
async fn parsed_command_is_consumed_without_target_channel() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        &[],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Direct),
    );

    assert!(handle_runtime_command_if_needed(&ctx, &message("/model"), None).await);
}

#[tokio::test]
async fn approval_management_command_denies_sender_not_in_approvers() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        &["shell"],
        autonomy_config(&[], &["telegram:admin"], NonCliNaturalLanguageApprovalMode::Direct),
    );
    let channel = Arc::new(RecordingChannel::new());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    assert!(dispatch(&ctx, &channel_dyn, "/approve shell").await);
    let sent = channel.sent_contents().await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].contains("Approval-management command denied"));
    assert!(sent[0].contains("telegram:admin"));
}

#[tokio::test]
async fn natural_language_approval_denies_unauthorized_sender_before_direct_promotion() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        &["shell"],
        autonomy_config(
            &[],
            &["telegram:zeta", "telegram:admin"],
            NonCliNaturalLanguageApprovalMode::Direct,
        ),
    );
    let channel = Arc::new(RecordingChannel::new());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    assert!(dispatch(&ctx, &channel_dyn, "approve tool shell").await);

    let sent = channel.sent_contents().await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].contains("Approval-management command denied"));
    assert!(sent[0].contains("Allowed approvers: telegram:admin, telegram:zeta"));
    assert!(!ctx.approval_manager.is_non_cli_session_granted("shell"));
    assert!(
        ctx.approval_manager
            .list_non_cli_pending_requests(Some("sender"), Some("telegram"), Some("chat-1"))
            .is_empty()
    );
}

#[tokio::test]
async fn natural_language_approval_commands_follow_runtime_policy_modes() {
    let tmp = tempfile::tempdir().unwrap();

    let disabled = context(
        tmp.path(),
        &["shell"],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Disabled),
    );
    let disabled_channel = Arc::new(RecordingChannel::new());
    let disabled_dyn: Arc<dyn Channel> = disabled_channel.clone();
    assert!(dispatch(&disabled, &disabled_dyn, "approve tool shell").await);
    assert!(
        disabled_channel.sent_contents().await[0]
            .contains("Natural-language approval commands are disabled")
    );

    // Pending requests require supervised autonomy, independently of the default policy.
    let mut request_policy =
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::RequestConfirm);
    request_policy.level = agent_config::runtime_schema::AutonomyLevel::Supervised;
    let request_confirm = context(
        tmp.path(),
        &["shell"],
        request_policy,
    );
    let request_channel = Arc::new(RecordingChannel::new());
    let request_dyn: Arc<dyn Channel> = request_channel.clone();
    assert!(dispatch(&request_confirm, &request_dyn, "approve tool shell").await);
    assert!(request_channel.sent_contents().await[0].contains("Approval request created."));
    assert_eq!(
        request_confirm
            .approval_manager
            .list_non_cli_pending_requests(Some("sender"), Some("telegram"), Some("chat-1"))
            .len(),
        1
    );

    let direct = context(
        tmp.path(),
        &["shell"],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Direct),
    );
    let direct_channel = Arc::new(RecordingChannel::new());
    let direct_dyn: Arc<dyn Channel> = direct_channel.clone();
    assert!(dispatch(&direct, &direct_dyn, "approve tool shell").await);
    assert!(
        direct_channel.sent_contents().await[0]
            .contains("Approved supervised execution for `shell`")
    );
    assert!(direct.approval_manager.is_non_cli_session_granted("shell"));
}

#[tokio::test]
async fn channel_specific_natural_language_policy_override_is_used() {
    let tmp = tempfile::tempdir().unwrap();
    let mut autonomy = autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Disabled);
    autonomy
        .non_cli_natural_language_approval_mode_by_channel
        .insert("telegram".to_string(), NonCliNaturalLanguageApprovalMode::Direct);
    let ctx = context(tmp.path(), &["shell"], autonomy);
    let channel = Arc::new(RecordingChannel::new());
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let msg = message_in_channel("telegram", "approve tool shell");

    assert!(dispatch_message(&ctx, &channel_dyn, &msg).await);
    assert!(channel.sent_contents().await[0].contains("Approved supervised execution for `shell`"));
    assert!(ctx.approval_manager.is_non_cli_session_granted("shell"));
}

#[tokio::test]
async fn direct_mode_keeps_natural_language_unapprove_as_revoke() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        &["shell"],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Direct),
    );
    ctx.approval_manager.grant_non_cli_session("shell");
    let channel = Arc::new(RecordingChannel::new());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    assert!(dispatch(&ctx, &channel_dyn, "unapprove shell").await);
    let sent = channel.sent_contents().await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].contains("Runtime session grant removed for `shell`: yes."));
    assert!(!ctx.approval_manager.is_non_cli_session_granted("shell"));
}

#[tokio::test]
async fn task_mode_plain_message_is_consumed_without_reply_channel() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        &[],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Direct),
    );
    let msg = message("create a task");
    let sender_key = channel_support::scope::scoped_history_key(&ctx, &msg, Default::default());
    // Exercise dispatch in an already enabled mode without resolving a project or storage.
    crate::runtime_command::task_mode::set_sender_task_mode(&ctx, &sender_key, true);

    assert!(handle_runtime_command_if_needed(&ctx, &msg, None).await);
}

#[tokio::test]
async fn send_failure_still_consumes_runtime_command() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        &[],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Direct),
    );
    let channel = Arc::new(RecordingChannel::failing());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    assert!(dispatch(&ctx, &channel_dyn, "/approvals").await);
}

#[tokio::test]
async fn channel_command_help_lark_complete_outbound() {
    let (sent, ctx) = help_reply("lark", "/help", false).await;
    let text = sent.iter().map(|m| m.content.as_str()).collect::<Vec<_>>().join("\n");
    for name in [
        "/projects",
        "/project",
        "/new",
        "/task",
        "/help",
        "/status",
        "/settings",
        "/models",
        "/model",
        "/effort",
        "/approve-all-once",
        "/approve-request",
        "/approve-confirm",
        "/approve-allow",
        "/approve-deny",
        "/approve-pending",
        "/approve",
        "/unapprove",
        "/approvals",
    ] {
        assert!(text.contains(name), "missing {name}");
    }
    assert!(ctx.route_overrides.lock().unwrap().is_empty());
    assert!(ctx.conversation_histories.lock().unwrap().is_empty());
}

#[tokio::test]
async fn channel_command_help_detail_and_unknown() {
    let (sent, _) = help_reply("lark", "/help approve-confirm", false).await;
    assert!(sent[0].content.contains("/approve-confirm <request-id>"));
    assert!(sent[0].content.contains("示例"));
    let (sent, _) = help_reply("lark", "/help absent", false).await;
    assert!(sent[0].content.contains("未知"));
    assert!(sent[0].content.contains("/help"));
}

#[tokio::test]
async fn channel_command_help_permission_does_not_expose_approvers() {
    let (sent, _) = help_reply("lark", "/help", true).await;
    let text = sent.iter().map(|m| m.content.as_str()).collect::<Vec<_>>().join("\n");
    assert!(text.contains("无权执行"));
    assert!(!text.contains("private-admin"));
}

#[tokio::test]
async fn channel_command_help_named_instance_preserves_addressing() {
    let (sent, _) = help_reply("lark", "/help", false).await;
    assert!(!sent.is_empty());
    for msg in sent {
        assert_eq!(msg.recipient, "chat-1");
        assert_eq!(msg.thread_ts.as_deref(), Some("thread-1"));
    }
}

#[tokio::test]
async fn channel_command_help_long_output_has_complete_entries() {
    let (sent, _) = help_reply("lark", "/help", false).await;
    assert!(sent.len() > 1);
    let text = sent.iter().map(|m| m.content.as_str()).collect::<Vec<_>>().join("\n\n");
    for msg in &sent {
        assert!(msg.content.len() <= 4000);
        assert!(msg.content.contains("示例"));
    }
    assert_eq!(text.matches("/approve-confirm <request-id>").count(), 1);
}

#[tokio::test]
async fn channel_command_help_text_channels_share_catalog() {
    for platform in ["lark", "qq", "dingtalk", "slack"] {
        let (sent, _) = help_reply(platform, "/help", false).await;
        let text = sent.iter().map(|m| m.content.as_str()).collect::<Vec<_>>().join("\n");
        for meta in crate::runtime_command::catalog::COMMANDS {
            assert_eq!(
                text.lines().filter(|line| *line == meta.usage).count(),
                1,
                "{platform}: {}",
                meta.name
            );
        }
    }
}

// 本文件确认渠道运行时命令测试模块已正确挂载。

#[test]
// 哨兵防止运行时命令子测试在模块拆分后被遗漏。
fn test_module_is_linked() {
    let module = module_path!();
    assert!(module.contains("tests"));
}
