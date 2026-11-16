//! Own a pending remote prompt until its future finishes or is cancelled.
use agent_approval::{ApprovalManager, PendingNonCliApprovalRequest};

pub struct PendingPromptApproval<'a> {
    manager: &'a ApprovalManager,
    request: PendingNonCliApprovalRequest,
}

impl<'a> PendingPromptApproval<'a> {
    pub fn new(manager: &'a ApprovalManager, request: PendingNonCliApprovalRequest) -> Self {
        Self { manager, request }
    }

    pub fn request(&self) -> &PendingNonCliApprovalRequest {
        &self.request
    }
}

impl Drop for PendingPromptApproval<'_> {
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
