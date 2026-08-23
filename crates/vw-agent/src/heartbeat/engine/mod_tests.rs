// 验证心跳引擎的调度、状态更新与停止流程。

// 临时心跳文件与暂停时间共同验证循环可停止，单次错误不会终止后续周期。
use super::*;
use crate::app::agent::observability::traits::ObserverMetric;
use std::sync::Once;
use std::sync::atomic::{AtomicUsize, Ordering};

static INFO_TRACING: Once = Once::new();

fn engine_for(workspace_dir: std::path::PathBuf) -> HeartbeatEngine {
    let observer: Arc<dyn Observer> = Arc::new(crate::app::agent::observability::NoopObserver);
    HeartbeatEngine::new(
        HeartbeatConfig { enabled: true, interval_minutes: 1, ..HeartbeatConfig::default() },
        workspace_dir,
        observer,
    )
}

struct InfoSubscriber;

impl tracing::Subscriber for InfoSubscriber {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        *metadata.level() <= tracing::Level::INFO
    }

    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::Id {
        tracing::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::Id, _follows: &tracing::Id) {}

    fn event(&self, _event: &tracing::Event<'_>) {}

    fn enter(&self, _span: &tracing::Id) {}

    fn exit(&self, _span: &tracing::Id) {}

    fn register_callsite(
        &self,
        metadata: &'static tracing::Metadata<'static>,
    ) -> tracing::subscriber::Interest {
        if self.enabled(metadata) {
            tracing::subscriber::Interest::always()
        } else {
            tracing::subscriber::Interest::never()
        }
    }

    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(tracing::metadata::LevelFilter::INFO)
    }
}

fn enable_info_tracing() {
    INFO_TRACING.call_once(|| {
        let _ = tracing::subscriber::set_global_default(InfoSubscriber);
    });
}

#[derive(Default)]
// 记录观察器收集循环事件与错误，不把测试绑定到具体日志后端。
struct RecordingObserver {
    ticks: AtomicUsize,
    errors: AtomicUsize,
}

impl Observer for RecordingObserver {
    fn record_event(&self, event: &ObserverEvent) {
        match event {
            ObserverEvent::HeartbeatTick => {
                self.ticks.fetch_add(1, Ordering::Relaxed);
            }
            ObserverEvent::Error { component, .. } if component == "heartbeat" => {
                self.errors.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    fn record_metric(&self, _metric: &ObserverMetric) {}

    fn name(&self) -> &str {
        "recording"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[test]
fn parse_tasks_trims_line_edges_but_keeps_task_body_spacing() {
    let content = "  - first task  \n- second  task\n* ignored\n-\n - ";

    let tasks = HeartbeatEngine::parse_tasks(content);

    assert_eq!(tasks, vec!["first task", "second  task"]);
}

#[test]
fn parse_tasks_supports_unicode_and_preserves_markdown_after_bullet() {
    let content = "- 日本語\n- **bold** text\n- [link](https://example.test)";

    let tasks = HeartbeatEngine::parse_tasks(content);

    assert_eq!(tasks[0], "日本語");
    assert_eq!(tasks[1], "**bold** text");
    assert_eq!(tasks[2], "[link](https://example.test)");
}

#[tokio::test]
async fn collect_tasks_returns_empty_when_file_is_absent() {
    let temp = tempfile::tempdir().unwrap();
    let engine = engine_for(temp.path().to_path_buf());

    let tasks = engine.collect_tasks().await.unwrap();

    assert!(tasks.is_empty());
}

#[tokio::test]
async fn collect_tasks_reads_heartbeat_file_and_ignores_non_tasks() {
    let temp = tempfile::tempdir().unwrap();
    tokio::fs::write(temp.path().join("HEARTBEAT.md"), "# title\n- A\ntext\n  - B\n")
        .await
        .unwrap();
    let engine = engine_for(temp.path().to_path_buf());

    let tasks = engine.collect_tasks().await.unwrap();

    assert_eq!(tasks, vec!["A", "B"]);
}

#[tokio::test]
async fn collect_tasks_propagates_read_errors() {
    let temp = tempfile::tempdir().unwrap();
    tokio::fs::create_dir(temp.path().join("HEARTBEAT.md")).await.unwrap();
    let engine = engine_for(temp.path().to_path_buf());

    let error = engine.collect_tasks().await.unwrap_err();

    assert!(
        error.to_string().contains("Is a directory") || error.to_string().contains("directory")
    );
}

#[tokio::test]
async fn tick_counts_current_tasks() {
    let temp = tempfile::tempdir().unwrap();
    tokio::fs::write(temp.path().join("HEARTBEAT.md"), "- A\n- B\nnot a task").await.unwrap();
    let engine = engine_for(temp.path().to_path_buf());

    assert_eq!(engine.tick().await.unwrap(), 2);
}

#[tokio::test]
async fn ensure_heartbeat_file_creates_default_and_does_not_overwrite_existing_content() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("HEARTBEAT.md");

    HeartbeatEngine::ensure_heartbeat_file(temp.path()).await.unwrap();
    let default_content = tokio::fs::read_to_string(&path).await.unwrap();
    assert!(default_content.contains("Periodic Tasks"));

    tokio::fs::write(&path, "- custom").await.unwrap();
    HeartbeatEngine::ensure_heartbeat_file(temp.path()).await.unwrap();
    assert_eq!(tokio::fs::read_to_string(&path).await.unwrap(), "- custom");
}

#[tokio::test]
async fn run_returns_immediately_when_disabled() {
    let observer: Arc<dyn Observer> = Arc::new(crate::app::agent::observability::NoopObserver);
    let engine = HeartbeatEngine::new(
        HeartbeatConfig { enabled: false, interval_minutes: 1, ..HeartbeatConfig::default() },
        std::path::PathBuf::from("/tmp"),
        observer,
    );

    engine.run().await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn run_enabled_records_tick_and_processes_tasks_until_aborted() {
    enable_info_tracing();
    let temp = tempfile::tempdir().unwrap();
    tokio::fs::write(temp.path().join("HEARTBEAT.md"), "- A\n- B").await.unwrap();
    let observer = Arc::new(RecordingObserver::default());
    let engine = HeartbeatEngine::new(
        HeartbeatConfig { enabled: true, interval_minutes: 1, ..HeartbeatConfig::default() },
        temp.path().to_path_buf(),
        observer.clone(),
    );

    let task = tokio::spawn(async move { engine.run().await });

    for _ in 0..10 {
        if observer.ticks.load(Ordering::Relaxed) >= 2 {
            break;
        }
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(5 * 60)).await;
    }

    assert!(observer.ticks.load(Ordering::Relaxed) >= 1);
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }

    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
}

#[tokio::test(start_paused = true)]
async fn run_enabled_continues_without_error_when_no_tasks_exist() {
    let temp = tempfile::tempdir().unwrap();
    let observer = Arc::new(RecordingObserver::default());
    let engine = HeartbeatEngine::new(
        HeartbeatConfig { enabled: true, interval_minutes: 1, ..HeartbeatConfig::default() },
        temp.path().to_path_buf(),
        observer.clone(),
    );

    let task = tokio::spawn(async move { engine.run().await });

    for _ in 0..10 {
        if observer.ticks.load(Ordering::Relaxed) >= 2 {
            break;
        }
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(5 * 60)).await;
    }

    assert!(observer.ticks.load(Ordering::Relaxed) >= 1);
    assert_eq!(observer.errors.load(Ordering::Relaxed), 0);
    assert!(!task.is_finished());

    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
}

#[tokio::test(start_paused = true)]
async fn run_enabled_records_error_events_without_stopping_loop() {
    let temp = tempfile::tempdir().unwrap();
    tokio::fs::create_dir(temp.path().join("HEARTBEAT.md")).await.unwrap();
    let observer = Arc::new(RecordingObserver::default());
    let engine = HeartbeatEngine::new(
        HeartbeatConfig { enabled: true, interval_minutes: 1, ..HeartbeatConfig::default() },
        temp.path().to_path_buf(),
        observer.clone(),
    );

    let task = tokio::spawn(async move { engine.run().await });

    for _ in 0..10 {
        if observer.errors.load(Ordering::Relaxed) >= 1 {
            break;
        }
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(5 * 60)).await;
    }

    assert!(observer.ticks.load(Ordering::Relaxed) >= 1);
    assert!(observer.errors.load(Ordering::Relaxed) >= 1);

    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
}

#[tokio::test]
async fn ensure_heartbeat_file_propagates_write_errors() {
    let temp = tempfile::tempdir().unwrap();
    let file_parent = temp.path().join("not-a-directory");
    tokio::fs::write(&file_parent, "plain file").await.unwrap();

    let error = HeartbeatEngine::ensure_heartbeat_file(&file_parent).await.unwrap_err();

    assert!(
        error.to_string().contains("Not a directory")
            || error.to_string().contains("not a directory")
    );
}
