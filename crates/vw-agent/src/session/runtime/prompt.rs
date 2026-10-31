//! 单轮提示执行、事件归一化和显式取消。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

use tokio_util::sync::CancellationToken;

use crate::session::processor::{self, Request, StreamEvent};

use super::types::{PromptCancelHandle, PromptEvent, PromptTermination, PromptTurn};

pub(crate) type EventSink = Box<dyn FnMut(StreamEvent) -> bool + Send>;

fn map_non_terminal(event: StreamEvent) -> Option<PromptEvent> {
    match event {
        StreamEvent::LlmRawArtifact(artifact) => Some(PromptEvent::LlmRawArtifact(artifact)),
        StreamEvent::ContextLoaded { paths } => Some(PromptEvent::ContextLoaded { paths }),
        StreamEvent::Delta(text) => Some(PromptEvent::Text(text)),
        StreamEvent::Tool(event) => Some(PromptEvent::Tool(event)),
        StreamEvent::Compaction { phase } => Some(PromptEvent::Compaction { phase }),
        StreamEvent::StepStart { step_index, created_ms, model } => {
            Some(PromptEvent::StepStart { step_index, created_ms, model })
        }
        StreamEvent::StepFinish { step_index, finished_ms, usage, finish_reason, model } => {
            Some(PromptEvent::StepFinish { step_index, finished_ms, usage, finish_reason, model })
        }
        StreamEvent::PostToolRound { step_index } => {
            Some(PromptEvent::PostToolRound { step_index })
        }
        StreamEvent::ModelRecovery(state) => Some(PromptEvent::ModelRecovery(state)),
        StreamEvent::Done(_) | StreamEvent::Error(_) => None,
    }
}

fn send_terminal(
    events: &mpsc::Sender<PromptEvent>,
    terminal: &AtomicBool,
    termination: PromptTermination,
) {
    if !terminal.swap(true, Ordering::AcqRel) {
        let _ = events.send(PromptEvent::Terminated(termination));
    }
}

/// 启动一个真实处理器提示轮次。
pub fn start_prompt(request: Request) -> PromptTurn {
    start_prompt_with_runner(request, |request, emit| processor::run(request, emit))
}

pub(crate) fn start_prompt_with_runner<R>(mut request: Request, runner: R) -> PromptTurn
where
    R: FnOnce(Request, EventSink) + Send + 'static,
{
    let external = request.abort_token.take();
    let tenant = crate::session::ui_store::current_tenant_scope();
    #[cfg(not(target_arch = "wasm32"))]
    let shell_scope = request
        .root
        .as_ref()
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .and_then(|root| {
            crate::tools::shell::processes::ShellProcessScope::new(
                serde_json::to_string(&tenant).expect("tenant serialization"),
                &root,
                request.session.clone(),
            )
            .ok()
        });
    let token = CancellationToken::new();
    request.abort_token = Some(token.clone());
    let terminal = Arc::new(AtomicBool::new(false));
    let (events_tx, events_rx) = mpsc::channel();
    let cancel = PromptCancelHandle {
        explicit: Arc::new(AtomicBool::new(false)),
        shell_cutoff: Arc::new(std::sync::Mutex::new(None)),
        #[cfg(not(target_arch = "wasm32"))]
        shell_scope,
        token: token.clone(),
        terminal: Arc::clone(&terminal),
        events: events_tx.clone(),
    };

    if let Some(external) = external {
        let monitor = cancel.clone();
        std::thread::spawn(move || {
            while !monitor.terminal.load(Ordering::Acquire) {
                if external.is_cancelled() {
                    monitor.cancel();
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });
    }
    let cleanup = cancel.clone();
    std::thread::spawn(move || {
        let callback_events = events_tx.clone();
        let callback_terminal = Arc::clone(&terminal);
        let callback_token = token.clone();
        let callback_cleanup = cleanup.clone();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::session::ui_store::with_tenant_scope_sync(tenant, move || {
                runner(
                    request,
                    Box::new(move |event| {
                        if callback_token.is_cancelled() {
                            return false;
                        }

                        match event {
                            StreamEvent::Done(usage) => {
                                callback_cleanup.finish_shell_range();
                                send_terminal(
                                    &callback_events,
                                    callback_terminal.as_ref(),
                                    PromptTermination::Completed(usage),
                                );
                                false
                            }
                            StreamEvent::Error(error) => {
                                callback_cleanup.finish_shell_range();
                                send_terminal(
                                    &callback_events,
                                    callback_terminal.as_ref(),
                                    PromptTermination::Failed(error),
                                );
                                false
                            }
                            event => map_non_terminal(event)
                                .is_some_and(|event| callback_events.send(event).is_ok()),
                        }
                    }),
                );
            });
        }));
        if outcome.is_err() {
            token.cancel();
            cleanup.finish_shell_range();
            let error = match cleanup.reap_shell_processes() {
                Ok(()) => "session processor panicked".to_string(),
                Err(error) => {
                    format!("session processor panicked; process cleanup failed: {error}")
                }
            };
            send_terminal(&events_tx, terminal.as_ref(), PromptTermination::Failed(error));
            return;
        }
        // A tool may have crossed spawn just as explicit stop arrived.
        if cleanup.explicit.load(Ordering::Acquire)
            && let Err(error) = cleanup.reap_shell_processes()
        {
            send_terminal(
                &events_tx,
                terminal.as_ref(),
                PromptTermination::Failed(error.to_string()),
            );
            return;
        }

        if token.is_cancelled() {
            send_terminal(&events_tx, terminal.as_ref(), PromptTermination::Cancelled);
        } else {
            send_terminal(
                &events_tx,
                terminal.as_ref(),
                PromptTermination::Failed(
                    "session processor ended without a terminal event".to_string(),
                ),
            );
        }
    });

    PromptTurn { events: events_rx, cancel }
}
