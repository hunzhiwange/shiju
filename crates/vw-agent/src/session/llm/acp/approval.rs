//! Bind remote permission prompts to the current gateway turn.

use std::sync::Arc;

use parking_lot::Mutex;
use tokio_util::sync::CancellationToken;
use vw_acp::PermissionConfirmation;

use crate::agent::loop_::approval::await_non_cli_approval_decision;
use crate::approval::{
    ACP_APPROVAL_CHANNEL, ApprovalManager, ApprovalResponse, PendingNonCliApprovalRequest,
};

pub(super) struct PromptApproval<'a> {
    slot: &'a Mutex<Option<PermissionConfirmation>>,
    cancellation: CancellationToken,
}

impl<'a> PromptApproval<'a> {
    pub(super) fn bind(slot: &'a Mutex<Option<PermissionConfirmation>>, session_id: &str) -> Self {
        let context = crate::tools::current_tool_use_context().filter(|context| {
            !session_id.is_empty()
                && context.channel() == Some("desktop")
                && context.approval_manager().is_some()
                && context.non_cli_approval_context().is_some()
        });
        let cancellation = context
            .as_ref()
            .and_then(|context| context.abort_token())
            .map(CancellationToken::child_token)
            .unwrap_or_default();
        let prompt_cancellation = cancellation.clone();
        *slot.lock() = context.map(|context| {
            let session_id = session_id.to_owned();
            let confirm: PermissionConfirmation = Arc::new(move |title, arguments| {
                let context = context.clone();
                let session_id = session_id.clone();
                let cancellation = prompt_cancellation.clone();
                Box::pin(async move {
                    if cancellation.is_cancelled() {
                        return false;
                    }
                    let (Some(manager), Some(approval)) =
                        (context.approval_manager(), context.non_cli_approval_context())
                    else {
                        return false;
                    };
                    let arguments = crate::agent::loop_::scrub_credentials(&arguments.to_string());
                    let title = crate::agent::loop_::scrub_credentials(&title);
                    let request = manager.create_non_cli_pending_request(
                        "ACP",
                        &approval.sender,
                        ACP_APPROVAL_CHANNEL,
                        &session_id,
                        Some(title),
                        serde_json::from_str(&arguments)
                            .unwrap_or(serde_json::Value::String(arguments)),
                        context.message_id().map(str::to_owned),
                        Some(uuid::Uuid::new_v4().to_string()),
                    );
                    let pending = PendingApproval { manager, request };
                    let request = &pending.request;
                    await_non_cli_approval_decision(
                        manager,
                        &request.request_id,
                        &request.requested_by,
                        &request.requested_channel,
                        &request.requested_reply_target,
                        Some(&cancellation),
                    )
                    .await
                        == ApprovalResponse::Yes
                })
            });
            confirm
        });
        Self { slot, cancellation }
    }
}

impl Drop for PromptApproval<'_> {
    fn drop(&mut self) {
        self.cancellation.cancel();
        self.slot.lock().take();
    }
}

struct PendingApproval<'a> {
    manager: &'a ApprovalManager,
    request: PendingNonCliApprovalRequest,
}

impl Drop for PendingApproval<'_> {
    fn drop(&mut self) {
        let request = &self.request;
        let _ = self.manager.reject_non_cli_pending_request(
            &request.request_id,
            &request.requested_by,
            &request.requested_channel,
            &request.requested_reply_target,
        );
        self.manager.take_non_cli_pending_resolution(&request.request_id);
    }
}
