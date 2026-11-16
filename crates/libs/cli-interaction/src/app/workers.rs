//! 后台网关请求执行。

use session_ui_types::ChatRole;
use super::{
    ActiveStream,
    ActiveStreamKind,
    Arc,
    AtomicBool,
    ChatSession,
    CronJobSnapshot,
    Duration,
    GatewayChatStreamEvent,
    GatewayChatStreamRequest,
    GatewayStreamAttemptResult,
    GatewayUiRuntime,
    HostMessage,
    MemoryMutation,
    OrchestrationSnapshot,
    Ordering,
    OwnedWorkflowRunRequest,
    PathBuf,
    PendingPermissionReplyDto,
    StreamMessage,
    TaskPlanRunRequest,
    UiRuntimeEvent,
    UiRuntimeTerminalEvent,
    WorkflowResumeRequest,
    bootstrap_directory_switch,
    load_gateway_diagnostics_blocking,
    load_hook_audit_blocking,
    load_hook_catalog_blocking,
    load_owned_cron_detail_blocking,
    load_owned_task_detail_blocking,
    load_session_diff_blocking,
    load_workspace_files_blocking,
    mpsc,
    mutate_hook_and_refresh_blocking,
    mutate_memory_and_refresh_blocking,
    query_memory_blocking,
    read_workspace_file_blocking,
    restore_or_create_session_state,
    resume_workflow_run_blocking,
    search_files_blocking,
    stop_owned_plan_run_blocking,
    stream_plan_run_resumable_blocking,
    stream_workflow_run_blocking,
};

pub(super) fn spawn_stream(
    runtime: GatewayUiRuntime,
    request: GatewayChatStreamRequest,
    request_id: String,
    draft_prompt: Option<String>,
) -> ActiveStream {
    let (sender, receiver) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stream_cancel = Arc::clone(&cancel);
    let worker_request_id = request_id.clone();
    std::thread::spawn(move || {
        let mut done = false;
        let result = runtime.stream_chat_resumable_blocking(
            &request,
            &worker_request_id,
            &stream_cancel,
            |event| {
                if matches!(&event, GatewayChatStreamEvent::Done { .. }) {
                    done = matches!(
                        cli_gateway_transport::stream_adapter::adapt_gateway_stream_event(event.clone()),
                        UiRuntimeEvent::Terminal(UiRuntimeTerminalEvent::Done { .. })
                    );
                }
                sender.send(StreamMessage::ChatEvent(event)).is_ok()
            },
        );
        let mut result = if stream_cancel.load(Ordering::Acquire) {
            Ok(GatewayStreamAttemptResult::CallerStopped)
        } else {
            result
        };
        if let Some(prompt) = draft_prompt {
            let needs_reconciliation =
                matches!(result, Ok(GatewayStreamAttemptResult::Incomplete) | Err(_));
            if !stream_cancel.load(Ordering::Acquire)
                && (done || needs_reconciliation)
                && let Ok(Some(preview)) = runtime.session_publication_blocking()
            {
                if done {
                    let _ = sender.send(StreamMessage::ChatPublished { preview, recovered: None });
                } else if let Ok(snapshot) = runtime.session_ui_load_blocking(&preview.id)
                    && is_recovered_draft_turn(&snapshot, &prompt)
                    && !stream_cancel.load(Ordering::Acquire)
                {
                    let _ = sender.send(StreamMessage::ChatPublished {
                        preview,
                        recovered: Some(Box::new(snapshot)),
                    });
                    result = Ok(GatewayStreamAttemptResult::Terminated);
                }
            }
        }
        let _ = sender.send(StreamMessage::Finished(result));
    });
    ActiveStream { kind: ActiveStreamKind::Chat, request_id, receiver, cancel }
}

pub(super) fn is_recovered_draft_turn(snapshot: &ChatSession, prompt: &str) -> bool {
    matches!(snapshot.messages.as_slice(), [.., user, assistant]
        if user.role == ChatRole::User && user.content == prompt
            && assistant.role == ChatRole::Assistant && !assistant.content.trim().is_empty())
        && snapshot.messages.len() == snapshot.message_ids.len()
        && matches!(snapshot.message_ids.as_slice(), [.., Some(_), Some(_)])
}

pub(super) fn spawn_session_title_read(
    runtime: GatewayUiRuntime,
    request_id: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        for delay in [1, 2, 4, 8, 16, 16, 16] {
            std::thread::sleep(Duration::from_secs(delay));
            match runtime.session_publication_blocking() {
                Ok(Some(preview)) if !preview.title.trim().is_empty() => {
                    let _ = sender.send(HostMessage::SessionTitleLoaded {
                        directory: runtime.directory().to_path_buf(),
                        session_id: preview.id,
                        request_id,
                        title: preview.title,
                    });
                    break;
                }
                Ok(None) => break,
                Ok(Some(_)) | Err(_) => {}
            }
        }
    });
}

pub(super) fn spawn_file_search(
    runtime: GatewayUiRuntime,
    generation: u64,
    query: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = search_files_blocking(&runtime, query.as_str());
        let _ = sender.send(HostMessage::FileSearchFinished { generation, query, result });
    });
}

pub(super) fn spawn_session_diff(
    runtime: GatewayUiRuntime,
    generation: u64,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let session_id = runtime.session_id().map(str::to_string);
        let result = load_session_diff_blocking(&runtime);
        let _ = sender.send(HostMessage::SessionDiffFinished { generation, session_id, result });
    });
}

pub(super) fn spawn_doctor(runtime: GatewayUiRuntime, generation: u64, sender: mpsc::Sender<HostMessage>) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result = load_gateway_diagnostics_blocking(&runtime);
        let _ = sender.send(HostMessage::DoctorFinished { generation, directory, result });
    });
}

pub(super) fn spawn_cron_detail(
    runtime: GatewayUiRuntime,
    snapshot: Vec<CronJobSnapshot>,
    generation: u64,
    job_id: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result =
            load_owned_cron_detail_blocking(&runtime, &snapshot, job_id.as_str()).map(Box::new);
        let _ =
            sender.send(HostMessage::CronDetailFinished { generation, directory, job_id, result });
    });
}

pub(super) fn spawn_task_detail(
    runtime: GatewayUiRuntime,
    snapshot: OrchestrationSnapshot,
    generation: u64,
    task_id: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result =
            load_owned_task_detail_blocking(&runtime, &snapshot, task_id.as_str()).map(Box::new);
        let _ =
            sender.send(HostMessage::TaskDetailFinished { generation, directory, task_id, result });
    });
}

pub(super) fn spawn_workspace_files(
    runtime: GatewayUiRuntime,
    generation: u64,
    requested_path: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result = load_workspace_files_blocking(&runtime, requested_path.as_str());
        let _ = sender.send(HostMessage::WorkspaceFilesFinished {
            generation,
            directory,
            requested_path,
            result,
        });
    });
}

pub(super) fn spawn_workspace_file_read(
    runtime: GatewayUiRuntime,
    generation: u64,
    requested_path: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result = read_workspace_file_blocking(&runtime, requested_path.as_str());
        let _ = sender.send(HostMessage::WorkspaceFileReadFinished {
            generation,
            directory,
            requested_path,
            result,
        });
    });
}

pub(super) fn spawn_hook_catalog(
    runtime: GatewayUiRuntime,
    generation: u64,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result = load_hook_catalog_blocking(&runtime);
        let _ = sender.send(HostMessage::HookCatalogFinished { generation, directory, result });
    });
}

pub(super) fn spawn_hook_audit(runtime: GatewayUiRuntime, generation: u64, sender: mpsc::Sender<HostMessage>) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result = load_hook_audit_blocking(&runtime);
        let _ = sender.send(HostMessage::HookAuditFinished { generation, directory, result });
    });
}

pub(super) fn spawn_hook_mutation(
    runtime: GatewayUiRuntime,
    generation: u64,
    plugin_id: String,
    hook_hash: String,
    trusted: bool,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result = mutate_hook_and_refresh_blocking(
            &runtime,
            plugin_id.as_str(),
            hook_hash.as_str(),
            trusted,
        );
        let _ = sender.send(HostMessage::HookMutationFinished {
            generation,
            directory,
            plugin_id,
            hook_hash,
            trusted,
            result,
        });
    });
}

pub(super) fn spawn_memory_search(
    runtime: GatewayUiRuntime,
    generation: u64,
    query: Option<String>,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let result = query_memory_blocking(&runtime, query.as_deref());
        let _ =
            sender.send(HostMessage::MemorySearchFinished { generation, directory, query, result });
    });
}

pub(super) fn spawn_memory_mutation(
    runtime: GatewayUiRuntime,
    generation: u64,
    mutation: MemoryMutation,
    query: Option<String>,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let action = mutation.action_name();
        let key = mutation.key().to_string();
        let result = mutate_memory_and_refresh_blocking(&runtime, &mutation, query.as_deref());
        let _ = sender.send(HostMessage::MemoryMutationFinished {
            generation,
            directory,
            query,
            action,
            key,
            result,
        });
    });
}

pub(super) fn spawn_plan_run(
    runtime: GatewayUiRuntime,
    request: TaskPlanRunRequest,
    request_id: String,
) -> ActiveStream {
    let (sender, receiver) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stream_cancel = Arc::clone(&cancel);
    let worker_request_id = request_id.clone();
    std::thread::spawn(move || {
        let result = stream_plan_run_resumable_blocking(
            &runtime,
            &request,
            worker_request_id.as_str(),
            |event| {
                !stream_cancel.load(Ordering::Acquire)
                    && sender.send(StreamMessage::RuntimeEvent(event)).is_ok()
            },
        );
        let result = if stream_cancel.load(Ordering::Acquire) {
            Ok(GatewayStreamAttemptResult::CallerStopped)
        } else {
            result
        };
        let _ = sender.send(StreamMessage::Finished(result));
    });
    ActiveStream { kind: ActiveStreamKind::PlanRun, request_id, receiver, cancel }
}

pub(super) fn spawn_workflow_run(
    runtime: GatewayUiRuntime,
    request: OwnedWorkflowRunRequest,
    request_id: String,
) -> ActiveStream {
    let (sender, receiver) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stream_cancel = Arc::clone(&cancel);
    std::thread::spawn(move || {
        let result = stream_workflow_run_blocking(&runtime, &request, |event| {
            !stream_cancel.load(Ordering::Acquire)
                && sender.send(StreamMessage::WorkflowEvent(event)).is_ok()
        });
        let result = if stream_cancel.load(Ordering::Acquire) {
            Ok(GatewayStreamAttemptResult::CallerStopped)
        } else {
            result
        };
        let _ = sender.send(StreamMessage::Finished(result));
    });
    ActiveStream { kind: ActiveStreamKind::Workflow, request_id, receiver, cancel }
}

pub(super) fn spawn_stop(runtime: GatewayUiRuntime, request_id: String, sender: mpsc::Sender<HostMessage>) {
    std::thread::spawn(move || {
        let result = runtime.chat_stream_stop_blocking(request_id.as_str());
        let _ = sender.send(HostMessage::StopFinished { request_id, result });
    });
}

pub(super) fn spawn_plan_run_stop(
    runtime: GatewayUiRuntime,
    request_id: String,
    task_id: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = stop_owned_plan_run_blocking(
            &runtime,
            Some(request_id.as_str()),
            Some(task_id.as_str()),
            request_id.as_str(),
            task_id.as_str(),
        );
        let _ = sender.send(HostMessage::PlanRunStopFinished { request_id, task_id, result });
    });
}

pub(super) fn spawn_interaction_refresh(
    runtime: GatewayUiRuntime,
    session_id: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = runtime.interaction_snapshot_blocking(Some(session_id.as_str()));
        let _ = sender.send(HostMessage::InteractionsRefreshed(result));
    });
}

pub(super) fn spawn_session_bootstrap(mut runtime: GatewayUiRuntime, sender: mpsc::Sender<HostMessage>) {
    std::thread::spawn(move || {
        let result = restore_or_create_session_state(&mut runtime).map(Box::new);
        let _ = sender.send(HostMessage::SessionBootstrapFinished(result));
    });
}

pub(super) fn spawn_directory_switch(
    runtime: GatewayUiRuntime,
    requested_path: PathBuf,
    generation: u64,
    source_directory: PathBuf,
    recovery_draft: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = bootstrap_directory_switch(runtime, requested_path.as_path()).map(Box::new);
        let _ = sender.send(HostMessage::DirectorySwitchFinished {
            generation,
            source_directory,
            recovery_draft,
            result,
        });
    });
}

pub(super) fn spawn_permission_reply(
    runtime: GatewayUiRuntime,
    request_id: String,
    reply: PendingPermissionReplyDto,
    message: Option<String>,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = runtime.permission_reply_blocking(request_id.as_str(), reply, message);
        let _ = sender.send(HostMessage::PermissionReplyFinished { request_id, result });
    });
}

pub(super) fn spawn_question_reply(
    runtime: GatewayUiRuntime,
    request_id: String,
    answers: Option<Vec<Vec<String>>>,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = match answers {
            Some(answers) => runtime.question_reply_blocking(request_id.as_str(), answers),
            None => runtime.question_reject_blocking(request_id.as_str()),
        };
        let _ = sender.send(HostMessage::QuestionReplyFinished { request_id, result });
    });
}

pub(super) fn spawn_workflow_resume(
    runtime: GatewayUiRuntime,
    pause_request_id: String,
    request: WorkflowResumeRequest,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = resume_workflow_run_blocking(&runtime, &request).map(Box::new);
        let _ = sender.send(HostMessage::WorkflowResumeFinished { pause_request_id, result });
    });
}

/// 只捕获独立 runtime 快照；每个 runner 至多有一个列表请求在途。
pub(super) fn spawn_session_candidates(
    runtime: GatewayUiRuntime,
    generation: u64,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let directory = runtime.directory().to_path_buf();
        let session_id = runtime.session_id().map(str::to_string);
        let result = runtime.session_resume_candidates_blocking();
        let _ = sender.send(HostMessage::SessionCandidatesFinished {
            generation, directory, session_id, result,
        });
    });
}
