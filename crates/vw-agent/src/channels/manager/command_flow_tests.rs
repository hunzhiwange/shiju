use super::*;
use crate::channels::{channel_instance_key, runtime_command::catalog::COMMANDS};

async fn help_flow(platform: &str, instance: &str, target: &str, thread: Option<&str>) {
    let channel = Arc::new(RecordingChannel::new(false));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let mut ctx =
        context(HashMap::from([(channel_instance_key(platform, instance), channel_dyn)]), None);
    let temp = tempfile::tempdir().unwrap();
    ctx.workspace_dir = Arc::new(temp.path().to_path_buf());
    crate::project::from_directory(&crate::session::ui_store::current_tenant_scope(), temp.path())
        .await
        .unwrap();
    let ctx = Arc::new(ctx);
    let mut msg = message("/help", platform);
    msg.instance_id = instance.into();
    msg.reply_target = target.into();
    msg.thread_ts = thread.map(str::to_string);
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    process_channel_message(ctx.clone(), msg.clone(), CancellationToken::new()).await;
    let sent = channel.sent_contents().await;
    assert!(sent.len() > 1, "完整帮助应分段");
    assert!(sent.iter().all(|part| part.len() <= 1800));
    let all = sent.join("\n");
    let doc = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/vibewindow/通道命令.md"
    ))
    .expect("用户命令文档必须存在")
    .replace("\\|", "|");
    for entry in COMMANDS {
        assert!(all.contains(entry.usage), "帮助遗漏 {}", entry.name);
        assert!(doc.contains(entry.usage), "文档遗漏 {}", entry.name);
        for alias in entry.aliases {
            assert!(all.contains(&format!("/{alias}")));
            assert!(doc.contains(&format!("/{alias}")));
        }
    }
    msg.content = "/help project".into();
    process_channel_message(ctx.clone(), msg, CancellationToken::new()).await;
    let sent = channel.sent_contents().await;
    assert!(sent.last().unwrap().contains("/project [稳定ID|default]"));
    // processor 入口必须先创建并登记会话；没有会话即没有模型执行。
    assert!(!sender_session_store().lock().unwrap().contains_key(&snapshot.scope));
    assert!(ctx.conversation_histories.lock().unwrap().is_empty());
    assert!(ctx.route_overrides.lock().unwrap().is_empty());
    assert!(
        channel
            .destinations
            .lock()
            .await
            .iter()
            .all(|destination| destination == &(target.to_string(), thread.map(str::to_string)))
    );
}

#[tokio::test]
async fn channel_command_help_flow_feishu_private() {
    help_flow("feishu", "default", "user", None).await;
}
#[tokio::test]
async fn channel_command_help_flow_feishu_group() {
    help_flow("feishu", "default", "group", Some("topic")).await;
}
#[tokio::test]
async fn channel_command_help_flow_feishu_named_instance() {
    help_flow("feishu", "support", "group", Some("topic")).await;
}
#[tokio::test]
async fn channel_command_help_flow_qq() {
    help_flow("qq", "default", "group", None).await;
}
