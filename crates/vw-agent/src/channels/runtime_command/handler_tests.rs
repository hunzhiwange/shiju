use channel_runtime::command_test_support::*;
use super::handle_runtime_command_if_needed;
use channel_support::runtime_state::ChannelRuntimeContext;
use channel_types::{Channel, self as traits};
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

async fn dispatch(ctx: &ChannelRuntimeContext, channel: &Arc<dyn Channel>, content: &str) -> bool {
    crate::project::from_directory(
        &crate::session::ui_store::current_tenant_scope(),
        ctx.workspace_dir.as_ref(),
    )
    .await
    .unwrap();
    handle_runtime_command_if_needed(ctx, &message(content), Some(channel)).await
}

#[tokio::test(flavor = "multi_thread")]
async fn slash_command_dispatch_covers_session_model_and_approval_variants() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(
        tmp.path(),
        &["shell"],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Direct),
    );
    let channel = Arc::new(RecordingChannel::new());
    let channel_dyn: Arc<dyn Channel> = channel.clone();

    for content in [
        "/models",
        "/models definitely-missing-provider",
        "/model",
        "/model `gpt-next`",
        "/new",
        "/task",
        "/approve-all-once",
        "/approve-request shell",
    ] {
        assert!(dispatch(&ctx, &channel_dyn, content).await, "{content}");
    }

    let allow_id = create_pending(&ctx, "shell");
    assert!(dispatch(&ctx, &channel_dyn, &format!("/approve-allow {allow_id}")).await);
    let confirm_id = create_pending(&ctx, "shell");
    assert!(dispatch(&ctx, &channel_dyn, &format!("/approve-confirm {confirm_id}")).await);
    let deny_id = create_pending(&ctx, "shell");
    assert!(dispatch(&ctx, &channel_dyn, &format!("/approve-deny {deny_id}")).await);

    for content in ["/approve-pending", "/approve shell", "/unapprove shell", "/approvals"] {
        assert!(dispatch(&ctx, &channel_dyn, content).await, "{content}");
    }

    let sent = channel.sent_contents().await;
    assert!(sent.iter().any(|text| text.contains("无可用模型目录")));
    assert!(sent.iter().any(|text| text.contains("Unknown provider")));
    assert!(sent.iter().any(|text| text.contains("Current model")));
    assert!(sent.iter().any(|text| text.contains("模型未在当前 provider 目录中声明")));
    assert!(sent.iter().any(|text| text.contains("会话已重置")));
    assert!(sent.iter().any(|text| text.contains("我进入了任务模式")));
    assert!(sent.iter().any(|text| text.contains("One-time all-tools approval request created.")));
    assert!(sent.iter().any(|text| text.contains("Approval request created.")));
    assert!(sent.iter().any(|text| text.contains("Approved pending request")));
    assert!(sent.iter().any(|text| text.contains("Approved supervised execution for `shell`")));
    assert!(sent.iter().any(|text| text.contains("Denied pending approval request")));
    assert!(sent.iter().any(|text| text.contains("Pending approval requests")));
    assert!(sent.iter().any(|text| text.contains("Runtime session grant removed for `shell`")));
    assert!(sent.iter().any(|text| text.contains("Supervised non-CLI tool approvals:")));
}

#[tokio::test]
async fn channel_command_help_pipeline_routes_named_instance_without_project_or_model() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ctx = context(
        tmp.path(),
        &[],
        autonomy_config(&[], &[], NonCliNaturalLanguageApprovalMode::Direct),
    );
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    ctx.provider = Arc::new(CountingProvider(calls.clone()));
    let selected = Arc::new(RecordingChannel::new());
    let other = Arc::new(RecordingChannel::new());
    ctx.channels_by_name = Arc::new(HashMap::from([
        (super::super::channel_instance_key("lark", "work"), selected.clone() as Arc<dyn Channel>),
        (super::super::channel_instance_key("lark", "default"), other.clone() as Arc<dyn Channel>),
    ]));
    let mut msg = message_in_channel("lark", "/help");
    msg.instance_id = "work".into();
    // Help remains available when a configured project binding is invalid.
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("lark:work".into(), "missing-project".into());
    super::super::process_channel_message(
        Arc::new(ctx),
        msg,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;
    let sent = selected.sent_contents().await.join("\n");
    assert!(sent.contains("全部已接通命令"));
    assert!(sent.contains("/approve-confirm <request-id>"));
    assert!(other.sent_contents().await.is_empty());
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}
