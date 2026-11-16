//! 首响应契约由真实宿主处理器驱动，不模拟会话创建、提交或标题请求。
use crate::host::ChannelHost;
use channel_support::{runtime_state::ChannelRuntimeContext, session::sender_session_store};
use channel_types::{Channel, ChannelMessage};
use std::{sync::{Arc, Mutex}, collections::HashMap, time::Duration};
use serde_json::Value;

fn lifecycle_id(ctx: &ChannelRuntimeContext) -> String {
    sender_session_store().lock().unwrap().get(&ctx.message_snapshot.as_ref().unwrap().scope)
        .cloned().expect("bound session")
}
async fn lifecycle_dispatch(host: &'static dyn ChannelHost, ctx: &ChannelRuntimeContext, msg: &ChannelMessage, text: &str) -> String {
    let channel = Arc::new(crate::dispatch_test_support::RecordingChannel::new("lark"));
    let target: Arc<dyn Channel> = channel.clone();
    let mut msg = msg.clone();
    msg.content = text.into();
    let mut ctx = ctx.clone();
    ctx.channels_by_name = Arc::new(HashMap::from([(
        channel_support::scope::channel_instance_key(&msg.channel, &msg.instance_id), target
    )]));
    Box::pin(crate::manager::execution_control::process_channel_message(host, Arc::new(ctx), msg.clone(), tokio_util::sync::CancellationToken::new())).await;
    let sent = channel.sent.lock().await;
    assert_eq!(sent.len(), 1, "each command sends a response");
    assert_eq!(sent[0].recipient, msg.reply_target);
    assert_eq!(sent[0].thread_ts, msg.thread_ts);
    sent[0].content.clone()
}

pub async fn channel_first_response_hides_ordinary_and_explicit_drafts(host: &'static dyn ChannelHost, ctx: &ChannelRuntimeContext, msg: &mut ChannelMessage) {
    let snapshot = ctx.message_snapshot.as_ref().unwrap();
    let mut input = msg.clone();
    input.content = "first input before success".into();
    let ordinary = host.session_id(snapshot, &input, None).await.unwrap();
    let info = host.get_session(&ordinary).await.unwrap();
    assert_eq!(info.title, "", "initial input is not a formal title");
    assert!(info.is_first_response_pending());
    assert!(!lifecycle_dispatch(host, &ctx, &msg, "/sessions").await.contains(&ordinary));
    assert!(lifecycle_dispatch(host, &ctx, &msg, "/continue").await.contains("暂无可继续"));
    assert!(lifecycle_dispatch(host, &ctx, &msg, &format!("/resume {ordinary}")).await.contains("失败"));
    assert_eq!(lifecycle_id(&ctx), ordinary);
    assert!(lifecycle_dispatch(host, &ctx, &msg, "/new").await.contains("已创建"));
    let draft = lifecycle_id(&ctx);
    assert_ne!(draft, ordinary);
    assert!(host.get_session(&draft).await.unwrap().title.is_empty());
    assert!(channel_support::session::channel_session_ownership(snapshot, &draft).await.unwrap().route.is_some());
    assert!(!lifecycle_dispatch(host, &ctx, &msg, "/sessions").await.contains(&draft));
}

pub async fn channel_first_response_reuses_failed_ingress_then_publishes_success(host: &'static dyn ChannelHost, ctx: &ChannelRuntimeContext, msg: &mut ChannelMessage) -> String {
    let snapshot = ctx.message_snapshot.as_ref().unwrap();
    msg.id = "failed".into();
    let failure = lifecycle_dispatch(host, &ctx, &msg, "this requires the unavailable model").await;
    assert!(failure.contains("Error:"), "{failure}");
    let id = lifecycle_id(&ctx);
    assert_eq!(host.get_session(&id).await.unwrap().title, "");
    assert!(!lifecycle_dispatch(host, &ctx, &msg, "/sessions").await.contains(&id));

    std::fs::create_dir(snapshot.directory.join("docs")).unwrap();
    std::fs::write(snapshot.directory.join("docs/guide.md"), "# Test guide").unwrap();
    msg.id = "successful".into();
    let prompt = "请读取 docs 文件列表";
    let reply = lifecycle_dispatch(host, &ctx, &msg, prompt).await;
    assert!(reply.contains("docs/guide.md"), "{reply}");
    assert_eq!(lifecycle_id(&ctx), id);
    let listed = lifecycle_dispatch(host, &ctx, &msg, "/sessions").await;
    assert!(listed.contains(&format!("{id}  {prompt}")), "{listed}");
    assert!(lifecycle_dispatch(host, &ctx, &msg, "/continue").await.contains(&id));
    id

}

pub async fn channel_first_response_requests_title_once_after_commit(host: &'static dyn ChannelHost, ctx: &ChannelRuntimeContext, msg: &mut ChannelMessage, requests: &Arc<Mutex<Vec<Value>>>) {
    let snapshot = ctx.message_snapshot.as_ref().unwrap();
    let prompt = "请读取 docs 文件列表";
    assert!(lifecycle_dispatch(host, &ctx, &msg, prompt).await.contains("Error:"));
    assert!(requests.lock().unwrap().is_empty());
    let id = lifecycle_id(&ctx);
    std::fs::create_dir(snapshot.directory.join("docs")).unwrap();
    std::fs::write(snapshot.directory.join("docs/guide.md"), "# Guide").unwrap();
    assert!(lifecycle_dispatch(host, &ctx, &msg, prompt).await.contains("docs/guide.md"));
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if host.get_session(&id).await.unwrap().title == "Fixture title" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }).await.expect("isolated model title should be saved");
    assert!(lifecycle_dispatch(host, &ctx, &msg, "/rename Explicit title").await.contains("已重命名"));
    msg.id = "second-turn".into();
    assert!(lifecycle_dispatch(host, &ctx, &msg, prompt).await.contains("docs/guide.md"));
    assert!(lifecycle_dispatch(host, &ctx, &msg, "/sessions").await.contains("Explicit title"));
    let captured = requests.lock().unwrap();
    assert_eq!(captured.len(), 1);
    assert!(captured[0]["messages"].to_string().contains(prompt));
    assert!(captured[0].get("tools").is_none_or(|tools| tools.as_array().is_some_and(Vec::is_empty)));

}
