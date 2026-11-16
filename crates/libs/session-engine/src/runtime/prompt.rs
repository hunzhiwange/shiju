//! 单轮提示执行、事件归一化和显式取消。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

use tokio_util::sync::CancellationToken;

use crate::runner_types::Request;
use crate::processor_events::StreamEvent;
use tool_context::host::ContextHost;

use session_port::{PromptCancelHandle, PromptEvent, PromptTermination, PromptTurn};

pub type EventSink = session_port::EventSink<StreamEvent>;

fn map_non_terminal(event: StreamEvent) -> Option<PromptEvent> {
    match event {
        StreamEvent::LlmRawArtifact(artifact) => Some(PromptEvent::LlmRawArtifact(artifact)),
        StreamEvent::ContextLoaded { paths } => Some(PromptEvent::ContextLoaded { paths }),
        StreamEvent::Delta(text) => Some(PromptEvent::Text(text)),
        StreamEvent::Tool(event) => Some(PromptEvent::Tool(event)),
        StreamEvent::TodoContinuation => Some(PromptEvent::TodoContinuation),
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
        StreamEvent::ModelToolCalls(_) | StreamEvent::Done(_) | StreamEvent::Error(_) => None,
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

struct WorkerCompletion(Arc<AtomicBool>);

impl Drop for WorkerCompletion {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

pub fn start_prompt_with_runner<H: ContextHost, R>(mut request: Request<H::ApprovalManager, H::SessionSkillSources>, runner: R) -> PromptTurn
where
    H: Send + Sync,
    tool_context::context::ToolUseContext<H>: Send + Sync,
    H::ApprovalManager: Send + Sync,
    H::SessionSkillSources: Send,
    R: FnOnce(Request<H::ApprovalManager, H::SessionSkillSources>, EventSink) + Send + 'static,
{
    let external = request.abort_token.take();
    let tenant = session_store::ui_store::current_tenant_scope();
    #[cfg(not(target_arch = "wasm32"))]
    let shell_scope = request
        .root
        .as_ref()
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .and_then(|root| {
            tool_shell::shell::processes::ShellProcessScope::new(
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
    let cancel = PromptCancelHandle::new(
        token.clone(),
        Arc::clone(&terminal),
        events_tx.clone(),
        Arc::new(ShellCleanup {
            #[cfg(not(target_arch = "wasm32"))]
            scope: shell_scope,
        }),
    );

    let finished = Arc::new(AtomicBool::new(false));
    if let Some(external) = external {
        let monitor = cancel.clone();
        let finished = Arc::clone(&finished);
        std::thread::spawn(move || {
            while !finished.load(Ordering::Acquire) {
                if external.is_cancelled() {
                    monitor.cancel();
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });
    }
    let cleanup = cancel.clone();
    let delegated_context = tool_context::context::current_tool_use_context::<H>();
    let worker = std::thread::spawn(move || {
        let _completion = WorkerCompletion(finished);
        #[cfg(not(target_arch = "wasm32"))]
        let _worktree_use = match request
            .root
            .as_ref()
            .map(|root| worktree_core::registry::use_lease(std::path::Path::new(root)))
            .transpose()
        {
            Ok(lease) => lease,
            Err(error) => {
                send_terminal(
                    &events_tx,
                    terminal.as_ref(),
                    PromptTermination::Failed(error.to_string()),
                );
                return Ok(());
            }
        };
        let callback_events = events_tx.clone();
        let callback_terminal = Arc::clone(&terminal);
        let callback_token = token.clone();
        let callback_cleanup = cleanup.clone();
        session_store::ui_store::with_tenant_scope_sync(tenant, move || {
            let run = || {
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
                            // Internal model calls are observed by the processor only.
                            // Ignoring them must keep the tool-executing consumer alive.
                            StreamEvent::ModelToolCalls(_) => true,
                            event => map_non_terminal(event)
                                .is_some_and(|event| callback_events.send(event).is_ok()),
                        }
                    }),
                )
            };
            match delegated_context {
                Some(context) => tool_context::context::scope_tool_use_context_sync::<H, _>(context, run),
                None => run(),
            }
        });
        // A tool may have crossed spawn just as explicit stop arrived.
        if cleanup.is_explicit()
            && let Err(error) = cleanup.reap_shell_processes()
        {
            send_terminal(
                &events_tx,
                terminal.as_ref(),
                PromptTermination::Failed(error.to_string()),
            );
            return Err(format!("execution shutdown unconfirmed: {error}"));
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
        Ok(())
    });

    PromptTurn { events: events_rx, cancel, worker }
}

#[derive(Debug, Default)]
pub struct ShellCleanup {
    #[cfg(not(target_arch = "wasm32"))]
    pub scope: Option<tool_shell::shell::processes::ShellProcessScope>,
}

impl session_port::PromptCleanup for ShellCleanup {
    fn reap(&self, cutoff: Option<std::time::Instant>) -> anyhow::Result<()> {
        #[cfg(target_arch = "wasm32")]
        let _ = cutoff;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(scope) = &self.scope
            && let Some(host) = tool_shell::shell::processes::ShellProcesses::existing()
        {
            host.cancel_scope_before(scope, cutoff)?;
        }
        Ok(())
    }
}
