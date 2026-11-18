use super::*;
use crate::channels::dispatch_tests::{context, message};

#[cfg(not(target_os = "macos"))]
struct PanicOnceChannel {
    panicked: AtomicBool,
    sends: AtomicU64,
}

#[cfg(not(target_os = "macos"))]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for PanicOnceChannel {
    fn name(&self) -> &str {
        "panic-once"
    }

    async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
        if !self.panicked.swap(true, Ordering::SeqCst) {
            panic!("channel send failed");
        }
        self.sends.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

#[tokio::test]
// 仓库 macOS 链接参数禁用 compact unwind，panic 无法展开，改由下面的 abort 用例验证析构。
#[cfg(not(target_os = "macos"))]
async fn channel_worker_panic_releases_busy_state_and_allows_retry() {
    for name in ["lark", "feishu", "qq", "telegram", "discord", "slack", "dingtalk", "wecom"] {
        let tmp = tempfile::tempdir().unwrap();
        let channel = Arc::new(PanicOnceChannel {
            panicked: AtomicBool::new(false),
            sends: AtomicU64::new(0),
        });
        // 不可用项目会在持有执行锁后发送错误，稳定触发通道发送 panic。
        let ctx =
            Arc::new(context(&tmp.path().join("missing"), name, Some(channel.clone()), false));
        let mut msg = message("first", name, "sender", "room");
        msg.content = "hello".into();
        process_channel_message(ctx.clone(), msg.clone(), CancellationToken::new()).await;
        assert!(channel.panicked.load(Ordering::SeqCst), "{name}");
        assert!(active_channel_task(&ctx, &msg).is_none(), "{name}");
        assert!(channel_chat_gate(&ctx, &msg).try_write().is_ok(), "{name}");
        assert!(stop_channel_task(&ctx, &msg).await.contains("没有正在执行"), "{name}");

        msg.id = "retry".into();
        process_channel_message(ctx.clone(), msg.clone(), CancellationToken::new()).await;
        assert_eq!(channel.sends.load(Ordering::SeqCst), 1, "{name}");
        assert!(active_channel_task(&ctx, &msg).is_none(), "{name}");
    }
}

// Hold real runtime feedback open so caller cancellation can be observed before cleanup.
struct BlockingFeedbackChannel {
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for BlockingFeedbackChannel {
    fn name(&self) -> &str {
        "blocking-feedback"
    }

    async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
        self.entered.notify_one();
        self.release.notified().await;
        Ok(())
    }

    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn channel_worker_abort_releases_busy_state_and_notifies_stop() {
    for name in ["lark", "feishu", "qq", "telegram", "discord", "slack", "dingtalk", "wecom"] {
        let tmp = tempfile::tempdir().unwrap();
        let channel = Arc::new(BlockingFeedbackChannel {
            entered: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
        });
        let ctx =
            Arc::new(context(&tmp.path().join("missing"), name, Some(channel.clone()), false));
        let mut msg = message("first", name, "sender", "room");
        msg.content = "hello".into();
        let cancellation = CancellationToken::new();
        let worker =
            tokio::spawn(process_channel_message(ctx.clone(), msg.clone(), cancellation.clone()));
        tokio::time::timeout(Duration::from_secs(1), channel.entered.notified()).await.unwrap();
        let task = active_channel_task(&ctx, &msg).expect("runtime task must be registered");
        assert!(channel_chat_gate(&ctx, &msg).try_write().is_err(), "{name}");
        worker.abort();
        assert!(worker.await.unwrap_err().is_cancelled());
        assert!(cancellation.is_cancelled(), "{name}");
        // Feedback still owns the execution gate until it actually exits.
        assert!(active_channel_task(&ctx, &msg).is_some(), "{name}");
        assert!(channel_chat_gate(&ctx, &msg).try_write().is_err(), "{name}");
        channel.release.notify_one();
        tokio::time::timeout(Duration::from_secs(1), task.completion.wait()).await.unwrap();
        assert!(active_channel_task(&ctx, &msg).is_none(), "{name}");
        assert!(channel_chat_gate(&ctx, &msg).try_write().is_ok(), "{name}");
        assert!(stop_channel_task(&ctx, &msg).await.contains("没有正在执行"), "{name}");
    }
}
