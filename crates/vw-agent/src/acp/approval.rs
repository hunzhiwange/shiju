//! 原生非 CLI 审批与 ACP 权限请求之间的一次性桥接。

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use vw_acp::server::{
    BackendPermissionOption, BackendPermissionOptionKind, BackendPermissionRequest,
};

use crate::agent::loop_::approval::NonCliApprovalPrompt;
use crate::app::agent::approval::{ApprovalManager, ApprovalResponse};

const ALLOW_ONCE: &str = "allow_once";
const REJECT_ONCE: &str = "reject_once";
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(300);

pub(crate) struct ApprovalBridge {
    manager: Arc<ApprovalManager>,
    prompts: mpsc::UnboundedReceiver<NonCliApprovalPrompt>,
    permissions: mpsc::UnboundedSender<BackendPermissionRequest>,
    cancellation: CancellationToken,
    timeout: Duration,
}

impl ApprovalBridge {
    pub(crate) fn new(
        manager: Arc<ApprovalManager>,
        prompts: mpsc::UnboundedReceiver<NonCliApprovalPrompt>,
        permissions: mpsc::UnboundedSender<BackendPermissionRequest>,
        cancellation: CancellationToken,
    ) -> Self {
        Self {
            manager,
            prompts,
            permissions,
            cancellation,
            timeout: APPROVAL_TIMEOUT,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub(crate) async fn run(mut self) {
        loop {
            let prompt = tokio::select! {
                prompt = self.prompts.recv() => {
                    let Some(prompt) = prompt else {
                        return;
                    };
                    prompt
                }
                () = self.cancellation.cancelled() => return,
            };
            self.resolve_prompt(prompt).await;
        }
    }

    async fn resolve_prompt(&self, prompt: NonCliApprovalPrompt) {
        let (response, selected) = tokio::sync::oneshot::channel();
        let request = BackendPermissionRequest {
            request_id: prompt.request_id.clone(),
            call_id: prompt.call_id.clone().unwrap_or_else(|| prompt.request_id.clone()),
            title: prompt.tool_name.clone(),
            raw_input: prompt.arguments.clone(),
            options: vec![
                BackendPermissionOption {
                    id: ALLOW_ONCE.to_string(),
                    name: "Allow once".to_string(),
                    kind: BackendPermissionOptionKind::AllowOnce,
                },
                BackendPermissionOption {
                    id: REJECT_ONCE.to_string(),
                    name: "Reject".to_string(),
                    kind: BackendPermissionOptionKind::RejectOnce,
                },
            ],
            response,
        };

        if self.permissions.send(request).is_err() {
            self.resolve_native(&prompt, ApprovalResponse::No);
            return;
        }

        let decision = tokio::select! {
            response = selected => match response {
                Ok(Some(option)) if option == ALLOW_ONCE => ApprovalResponse::Yes,
                Ok(Some(option)) if option == REJECT_ONCE => ApprovalResponse::No,
                Ok(Some(_)) | Ok(None) | Err(_) => ApprovalResponse::No,
            },
            () = self.cancellation.cancelled() => ApprovalResponse::No,
            () = tokio::time::sleep(self.timeout) => ApprovalResponse::No,
        };
        self.resolve_native(&prompt, decision);
    }

    fn resolve_native(&self, prompt: &NonCliApprovalPrompt, decision: ApprovalResponse) {
        let resolved = match decision {
            ApprovalResponse::Yes => self.manager.confirm_non_cli_pending_request(
                &prompt.request_id,
                &prompt.sender,
                &prompt.channel,
                &prompt.reply_target,
            ),
            ApprovalResponse::No | ApprovalResponse::Always => {
                self.manager.reject_non_cli_pending_request(
                    &prompt.request_id,
                    &prompt.sender,
                    &prompt.channel,
                    &prompt.reply_target,
                )
            }
        };
        if resolved.is_ok() {
            self.manager.record_non_cli_pending_resolution(&prompt.request_id, decision);
        }
    }
}
