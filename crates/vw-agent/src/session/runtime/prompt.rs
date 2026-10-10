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
    let token = CancellationToken::new();
    request.abort_token = Some(token.clone());
    let terminal = Arc::new(AtomicBool::new(false));
    let (events_tx, events_rx) = mpsc::channel();
    let cancel = PromptCancelHandle {
        token: token.clone(),
        terminal: Arc::clone(&terminal),
        events: events_tx.clone(),
    };

    std::thread::spawn(move || {
        let callback_events = events_tx.clone();
        let callback_terminal = Arc::clone(&terminal);
        let callback_token = token.clone();
        runner(
            request,
            Box::new(move |event| {
                if callback_token.is_cancelled() {
                    send_terminal(
                        &callback_events,
                        callback_terminal.as_ref(),
                        PromptTermination::Cancelled,
                    );
                    return false;
                }

                match event {
                    StreamEvent::Done(usage) => {
                        send_terminal(
                            &callback_events,
                            callback_terminal.as_ref(),
                            PromptTermination::Completed(usage),
                        );
                        false
                    }
                    StreamEvent::Error(error) => {
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
