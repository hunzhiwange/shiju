#[test]
fn llm_runner_tests_module_is_wired() {
    let marker = String::from("llm_runner_tests");
    assert_eq!(marker.as_str(), "llm_runner_tests");
}

use crate::app::agent::session::llm;
use crate::app::agent::tools::ToolSpec;
use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::Duration;


pub(crate) struct RecordedLlmRequest {
    pub(crate) tool_specs: Vec<ToolSpec>,
    pub(crate) system: Vec<String>,
    pub(crate) model: Option<String>,
    pub(crate) directory: String,
    pub(crate) tenant: vw_api_transport::TenantScope,
    pub(crate) messages: Vec<serde_json::Value>,
    pub(crate) options: serde_json::Value,
}
type RecordedMessages = std::collections::HashMap<String, Vec<RecordedLlmRequest>>;
static TEST_LLM_MESSAGES: std::sync::LazyLock<std::sync::Mutex<RecordedMessages>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Default::default()));

pub(super) fn record_test_llm_messages(
    session: String,
    messages: Vec<serde_json::Value>,
    options: serde_json::Value,
    system: Vec<String>,
    model: Option<String>,
    tool_specs: Vec<ToolSpec>,
) {
    TEST_LLM_MESSAGES.lock().unwrap().entry(session).or_default().push(RecordedLlmRequest {
        tool_specs,
        system,
        model,
        directory: crate::project::instance::directory(),
        tenant: crate::session::ui_store::current_tenant_scope(),
        messages,
        options,
    });
}

pub(crate) fn take_test_llm_messages(session: &str) -> Vec<RecordedLlmRequest> {
    TEST_LLM_MESSAGES.lock().unwrap().remove(session).unwrap_or_default()
}

static TEST_LLM_STREAMS: std::sync::LazyLock<std::sync::Mutex<VecDeque<Vec<llm::StreamEvent>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(VecDeque::new()));

static TEST_LLM_STREAM_TOOL_IDS: std::sync::LazyLock<std::sync::Mutex<Vec<Vec<String>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

static TEST_LLM_STREAM_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

pub(crate) fn test_llm_stream_guard() -> std::sync::MutexGuard<'static, ()> {
    let guard = TEST_LLM_STREAM_LOCK.lock().expect("lock scripted LLM stream tests");
    TEST_LLM_STREAMS.lock().expect("lock scripted LLM streams").clear();
    TEST_LLM_MESSAGES.lock().unwrap().clear();
    TEST_LLM_STREAM_TOOL_IDS.lock().expect("lock scripted LLM tool ids").clear();
    guard
}

pub(crate) fn enqueue_test_llm_stream(events: Vec<llm::StreamEvent>) {
    TEST_LLM_STREAMS.lock().expect("lock scripted LLM streams").push_back(events);
}

pub(crate) fn take_test_llm_stream_tool_ids() -> Vec<Vec<String>> {
    std::mem::take(&mut *TEST_LLM_STREAM_TOOL_IDS.lock().expect("lock scripted LLM tool ids"))
}

pub(super) fn spawn_test_llm_stream_thread(
    session: &str,
    tools: std::collections::HashMap<String, ToolSpec>,
    mut abort_rx: tokio::sync::watch::Receiver<bool>,
    tx: std::sync::mpsc::Sender<llm::StreamEvent>,
) -> std::thread::JoinHandle<()> {
    if let Some(stream) = CHANNEL_CANCEL_STREAMS.lock().unwrap().remove(session) {
        return std::thread::spawn(move || {
            stream.started.store(true, std::sync::atomic::Ordering::Release);
            loop {
                let cancelled = *abort_rx.borrow_and_update();
                if stream.release.load(std::sync::atomic::Ordering::Acquire) {
                    if !cancelled {
                        let _ = tx.send(llm::StreamEvent::Delta("completed naturally".into()));
                        let _ = tx.send(llm::StreamEvent::Done {
                            finish_reason: Some("stop".into()),
                            usage: Default::default(),
                        });
                    }
                    break;
                }
                if cancelled && !stream.hold_exit {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            stream.exited.store(true, std::sync::atomic::Ordering::Release);
        });
    }
    let mut tool_ids = tools.keys().cloned().collect::<Vec<_>>();
    tool_ids.sort();
    TEST_LLM_STREAM_TOOL_IDS.lock().expect("lock scripted LLM tool ids").push(tool_ids);
    let events =
        TEST_LLM_STREAMS.lock().expect("lock scripted LLM streams").pop_front().unwrap_or_else(
            || {
                vec![llm::StreamEvent::Error(
                    crate::app::agent::session::message::AssistantError::Unknown {
                        message: "missing scripted LLM stream".to_string(),
                    },
                )]
            },
        );

    std::thread::spawn(move || {
        for event in events {
            if *abort_rx.borrow_and_update() {
                break;
            }
            let _ = tx.send(event);
        }
    })
}









// ── llm_error_message ──────────────────────────────────────────────────────
















#[derive(Default)]
pub(crate) struct ChannelCancelStream {
    pub(crate) started: std::sync::atomic::AtomicBool,
    pub(crate) exited: std::sync::atomic::AtomicBool,
    pub(crate) release: std::sync::atomic::AtomicBool,
    pub(crate) hold_exit: bool,
}
static CHANNEL_CANCEL_STREAMS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<ChannelCancelStream>>>,
> = std::sync::LazyLock::new(Default::default);
pub(crate) fn register_channel_cancel_stream(
    session: &str,
    stream: std::sync::Arc<ChannelCancelStream>,
) {
    CHANNEL_CANCEL_STREAMS.lock().unwrap().insert(session.into(), stream);
}

#[test]
fn shell_outer_timeout_model_idle_does_not_cover_background_runtime() {
    use crate::tools::shell::processes::{ShellProcessScope, ShellProcessState, ShellProcesses};
    let dir = tempfile::tempdir().unwrap();
    let scope =
        ShellProcessScope::new("idle-tenant".into(), dir.path(), "idle-session".into()).unwrap();
    let host = ShellProcesses::new::<crate::tools::context::AgentContextHost>().unwrap();
    let caller = tokio::runtime::Runtime::new().unwrap();
    let started = caller
        .block_on(host.start(scope.clone(), || {
            let mut command = tokio::process::Command::new("sh");
            command.args(["-c", "printf idle-ready; exec sleep 60"]);
            Ok(command)
        }))
        .unwrap();
    let ready = caller
        .block_on(async {
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    let value = host
                        .wait(&scope, &started.process_id, 0, Duration::from_millis(100))
                        .await
                        .unwrap();
                    if value.output.contains("idle-ready") {
                        break value;
                    }
                }
            })
            .await
        })
        .unwrap();
    assert_eq!(ready.state, ShellProcessState::Running);
    let (tx, rx) = mpsc::channel();
    let (abort_tx, abort_rx) = tokio::sync::watch::channel(false);
    let result = super::collect_llm_step_from_stream(
        rx,
        &mut |_| true,
        &abort_tx,
        Duration::from_millis(10),
        None,
    );
    drop(tx);
    let after = host.snapshot(&scope, &started.process_id).unwrap();
    host.shutdown().unwrap();
    assert_eq!(result.unwrap_err(), "模型响应超时");
    assert!(*abort_rx.borrow(), "only the model stream is aborted");
    assert_eq!(after.state, ShellProcessState::Running);
    assert_eq!(
        host.snapshot(&scope, &started.process_id).unwrap().state,
        ShellProcessState::Cancelled
    );
    eprintln!(
        "model idle expired; process={} remained Running; shutdown reaped it",
        after.process_id
    );
}
