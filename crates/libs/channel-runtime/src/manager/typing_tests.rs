use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
struct TypingChannel {
    starts: AtomicUsize,
    stops: AtomicUsize,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for TypingChannel {
    fn name(&self) -> &str {
        "typing"
    }

    async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
        Ok(())
    }

    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn start_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn stop_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        self.stops.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn spawn_scoped_typing_task_stops_typing_when_cancelled() {
    tokio::time::pause();
    let channel = Arc::new(TypingChannel::default());
    let token = CancellationToken::new();

    let handle = spawn_scoped_typing_task(channel.clone(), "recipient".to_string(), token.clone());
    tokio::task::yield_now().await;
    token.cancel();
    handle.await.expect("typing task should finish");

    assert_eq!(channel.stops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn spawn_scoped_typing_task_refreshes_until_cancelled() {
    tokio::time::pause();
    let channel = Arc::new(TypingChannel::default());
    let token = CancellationToken::new();

    let handle = spawn_scoped_typing_task(channel.clone(), "recipient".to_string(), token.clone());
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(CHANNEL_TYPING_REFRESH_INTERVAL_SECS)).await;
    tokio::task::yield_now().await;
    token.cancel();
    handle.await.expect("typing task should finish");

    assert!(channel.starts.load(Ordering::SeqCst) >= 1);
    assert_eq!(channel.stops.load(Ordering::SeqCst), 1);
}

/// 一个 `start_typing` 与 `stop_typing` 都返回错误的通道，用于覆盖错误日志分支。
#[derive(Default)]
struct FailingTypingChannel {
    starts: AtomicUsize,
    stops: AtomicUsize,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for FailingTypingChannel {
    fn name(&self) -> &str {
        "failing-typing"
    }

    async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
        Ok(())
    }

    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn start_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        anyhow::bail!("start failed")
    }

    async fn stop_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        self.stops.fetch_add(1, Ordering::SeqCst);
        anyhow::bail!("stop failed")
    }
}

#[tokio::test]
async fn spawn_scoped_typing_task_logs_errors_without_failing() {
    tokio::time::pause();
    let channel = Arc::new(FailingTypingChannel::default());
    let token = CancellationToken::new();

    let handle = spawn_scoped_typing_task(channel.clone(), "recipient".to_string(), token.clone());
    tokio::task::yield_now().await;
    // 触发一次 tick，命中 start_typing 的错误日志分支。
    tokio::time::advance(Duration::from_secs(CHANNEL_TYPING_REFRESH_INTERVAL_SECS)).await;
    tokio::task::yield_now().await;
    token.cancel();
    // stop_typing 的错误同样只记录日志，任务正常结束。
    handle.await.expect("typing task should finish even when typing calls fail");

    assert!(channel.starts.load(Ordering::SeqCst) >= 1);
    assert_eq!(channel.stops.load(Ordering::SeqCst), 1);
}
