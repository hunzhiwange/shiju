use super::*;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
struct ImmediateChannel {
    listens: AtomicUsize,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for ImmediateChannel {
    fn name(&self) -> &str {
        "immediate"
    }

    async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
        Ok(())
    }

    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        self.listens.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

enum ListenOutcome {
    Ok,
    Err(&'static str),
    WaitForClosed,
}

struct ScriptedChannel {
    name: &'static str,
    listens: AtomicUsize,
    outcomes: Mutex<VecDeque<ListenOutcome>>,
    started: tokio::sync::mpsc::UnboundedSender<usize>,
}

impl ScriptedChannel {
    fn new(
        name: &'static str,
        outcomes: impl IntoIterator<Item = ListenOutcome>,
    ) -> (Self, tokio::sync::mpsc::UnboundedReceiver<usize>) {
        let (started, started_rx) = tokio::sync::mpsc::unbounded_channel();
        (
            Self {
                name,
                listens: AtomicUsize::new(0),
                outcomes: Mutex::new(outcomes.into_iter().collect()),
                started,
            },
            started_rx,
        )
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for ScriptedChannel {
    fn name(&self) -> &str {
        self.name
    }

    async fn send(&self, _message: &SendMessage) -> anyhow::Result<()> {
        Ok(())
    }

    async fn listen(
        &self,
        tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        let listen_number = self.listens.fetch_add(1, Ordering::SeqCst) + 1;
        let _ = self.started.send(listen_number);
        let outcome = self
            .outcomes
            .lock()
            .expect("scripted channel outcomes should not be poisoned")
            .pop_front()
            .unwrap_or(ListenOutcome::WaitForClosed);

        match outcome {
            ListenOutcome::Ok => Ok(()),
            ListenOutcome::Err(message) => Err(anyhow::anyhow!(message)),
            ListenOutcome::WaitForClosed => {
                tx.closed().await;
                Ok(())
            }
        }
    }
}

fn listener_health(component: &str) -> crate::app::agent::health::ComponentHealth {
    crate::app::agent::health::snapshot()
        .components
        .get(component)
        .cloned()
        .expect("component health should be recorded")
}

#[tokio::test]
async fn supervised_listener_exits_after_listener_returns_when_receiver_is_closed() {
    let channel = Arc::new(ImmediateChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    drop(rx);

    let handle = spawn_supervised_listener_with_health_interval(
        channel_dyn,
        tx,
        0,
        0,
        Duration::from_millis(0),
    );

    handle.await.expect("listener task should finish");
    assert_eq!(channel.listens.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn supervised_listener_default_entry_exits_when_receiver_is_closed() {
    let channel = Arc::new(ImmediateChannel::default());
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    drop(rx);

    let handle = spawn_supervised_listener(channel_dyn, tx, 0, 0);

    handle.await.expect("listener task should finish");
    assert_eq!(channel.listens.load(Ordering::SeqCst), 1);
}

#[tokio::test(start_paused = true)]
async fn supervised_listener_restarts_after_clean_exit_and_resets_error_on_next_loop() {
    let (channel, mut started_rx) = ScriptedChannel::new(
        "listener-clean-restart",
        [ListenOutcome::Ok, ListenOutcome::WaitForClosed],
    );
    let channel = Arc::new(channel);
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let (tx, rx) = tokio::sync::mpsc::channel(1);

    let handle = spawn_supervised_listener_with_health_interval(
        channel_dyn,
        tx,
        0,
        0,
        Duration::from_millis(5),
    );

    assert_eq!(started_rx.recv().await, Some(1));
    tokio::task::yield_now().await;

    let failed_health = listener_health("channel:listener-clean-restart");
    assert_eq!(failed_health.status, "error");
    assert_eq!(failed_health.last_error.as_deref(), Some("listener exited unexpectedly"));
    assert_eq!(failed_health.restart_count, 1);

    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(started_rx.recv().await, Some(2));
    drop(rx);

    handle.await.expect("listener task should finish");
    assert_eq!(channel.listens.load(Ordering::SeqCst), 2);

    let recovered_health = listener_health("channel:listener-clean-restart");
    assert_eq!(recovered_health.status, "ok");
    assert_eq!(recovered_health.last_error, None);
    assert_eq!(recovered_health.restart_count, 1);
}

#[tokio::test(start_paused = true)]
async fn supervised_listener_records_errors_and_retries_with_minimum_backoff() {
    let (channel, mut started_rx) = ScriptedChannel::new(
        "listener-error-retry",
        [ListenOutcome::Err("boom"), ListenOutcome::WaitForClosed],
    );
    let channel = Arc::new(channel);
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let (tx, rx) = tokio::sync::mpsc::channel(1);

    let handle = spawn_supervised_listener_with_health_interval(
        channel_dyn,
        tx,
        0,
        0,
        Duration::from_millis(5),
    );

    assert_eq!(started_rx.recv().await, Some(1));
    tokio::task::yield_now().await;

    let failed_health = listener_health("channel:listener-error-retry");
    assert_eq!(failed_health.status, "error");
    assert_eq!(failed_health.last_error.as_deref(), Some("boom"));
    assert_eq!(failed_health.restart_count, 1);

    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(started_rx.recv().await, Some(2));
    drop(rx);

    handle.await.expect("listener task should finish");
    assert_eq!(channel.listens.load(Ordering::SeqCst), 2);
}

#[tokio::test(start_paused = true)]
async fn supervised_listener_marks_health_while_listening() {
    let component = "channel:listener-health-tick";
    crate::app::agent::health::mark_component_error(component, "previous failure");
    let (channel, mut started_rx) =
        ScriptedChannel::new("listener-health-tick", [ListenOutcome::WaitForClosed]);
    let channel = Arc::new(channel);
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let (tx, rx) = tokio::sync::mpsc::channel(1);

    let handle = spawn_supervised_listener_with_health_interval(
        channel_dyn,
        tx,
        1,
        2,
        Duration::from_millis(5),
    );

    assert_eq!(started_rx.recv().await, Some(1));
    tokio::task::yield_now().await;

    let health = listener_health(component);
    assert_eq!(health.status, "ok");
    assert_eq!(health.last_error, None);

    drop(rx);
    handle.await.expect("listener task should finish");
    assert_eq!(channel.listens.load(Ordering::SeqCst), 1);
}
// 本文件验证渠道监听器监督、重启退避与健康状态记录。
