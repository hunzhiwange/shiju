use vw_gateway_client::{PendingPermissionReplyDto, PendingPermissionRequestDto};

use super::super::model::permission::{PermissionDecision, PermissionQueue, PermissionRequest};
#[cfg(not(target_arch = "wasm32"))]
use super::gateway::block_on_gateway;
use super::gateway::{GatewayUiRuntime, normalize_optional_str_ref};

pub(crate) trait PermissionRuntime {
    fn reply(
        &mut self,
        request_id: &str,
        reply: PendingPermissionReplyDto,
        reason: Option<String>,
    ) -> Result<(), String>;
}

pub(crate) trait FullAccessPermissionRuntime {
    fn pending_for_current_directory(&mut self) -> Result<Vec<PermissionRequest>, String>;
    fn approve(&mut self, request_id: &str, reply: PendingPermissionReplyDto)
    -> Result<(), String>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FullAccessPendingDrain {
    pub(crate) approved: usize,
    pub(crate) failed: usize,
    pub(crate) warning: Option<String>,
}

impl PermissionRuntime for GatewayUiRuntime {
    fn reply(
        &mut self,
        request_id: &str,
        reply: PendingPermissionReplyDto,
        reason: Option<String>,
    ) -> Result<(), String> {
        self.permission_reply_blocking(request_id, reply, reason)
    }
}

impl FullAccessPermissionRuntime for GatewayUiRuntime {
    fn pending_for_current_directory(&mut self) -> Result<Vec<PermissionRequest>, String> {
        self.permission_list_for_directory_blocking()
    }

    fn approve(
        &mut self,
        request_id: &str,
        reply: PendingPermissionReplyDto,
    ) -> Result<(), String> {
        self.permission_reply_blocking(request_id, reply, None)
    }
}

pub(crate) fn approve_current_directory_for_full_access(
    runtime: &mut dyn FullAccessPermissionRuntime,
) -> FullAccessPendingDrain {
    let requests = match runtime.pending_for_current_directory() {
        Ok(requests) => requests,
        Err(error) => {
            return FullAccessPendingDrain {
                warning: Some(format!("当前目录待处理权限请求加载失败: {error}")),
                ..FullAccessPendingDrain::default()
            };
        }
    };
    let mut summary = FullAccessPendingDrain::default();
    let mut errors = Vec::new();
    for request in requests {
        let reply = if request.always_available() {
            PendingPermissionReplyDto::Always
        } else {
            PendingPermissionReplyDto::Once
        };
        match runtime.approve(request.id.as_str(), reply) {
            Ok(()) => summary.approved = summary.approved.saturating_add(1),
            Err(error) => {
                summary.failed = summary.failed.saturating_add(1);
                errors.push(format!("{}: {error}", request.id));
            }
        }
    }
    if !errors.is_empty() {
        summary.warning = Some(format!(
            "{} 个当前目录待处理权限请求批准失败: {}",
            summary.failed,
            errors.join("; ")
        ));
    }
    summary
}

pub(crate) fn reply_current_permission(
    queue: &mut PermissionQueue,
    decision: PermissionDecision,
    runtime: &mut dyn PermissionRuntime,
) -> Result<(), String> {
    let request = queue.current().ok_or_else(|| "no pending permission request".to_string())?;
    let (reply, reason) = match decision {
        PermissionDecision::Once => (PendingPermissionReplyDto::Once, None),
        PermissionDecision::Always => {
            if !request.always_available() {
                return Err(
                    "always is unavailable because the server supplied no persistable pattern"
                        .to_string(),
                );
            }
            (PendingPermissionReplyDto::Always, None)
        }
        PermissionDecision::Reject { reason } => (PendingPermissionReplyDto::Reject, reason),
    };
    runtime.reply(&request.id, reply, reason)?;
    queue.pop_current();
    Ok(())
}

impl GatewayUiRuntime {
    pub(crate) async fn permission_list_for_directory(
        &self,
    ) -> Result<Vec<PermissionRequest>, String> {
        let directory = self.directory_value();
        self.client().permission_list(directory.as_deref()).await.map(
            |requests: Vec<PendingPermissionRequestDto>| {
                requests.into_iter().map(PermissionRequest::from_dto).collect()
            },
        )
    }

    pub(crate) async fn permission_reply(
        &self,
        request_id: &str,
        reply: PendingPermissionReplyDto,
        reason: Option<String>,
    ) -> Result<(), String> {
        let request_id = normalize_optional_str_ref(Some(request_id))
            .ok_or_else(|| "permission request id is required".to_string())?;
        let directory = self.directory_value();
        self.client()
            .permission_reply(request_id, reply, directory.as_deref(), reason)
            .await
            .and_then(|accepted| {
                accepted
                    .then_some(())
                    .ok_or_else(|| "gateway rejected permission reply".to_string())
            })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn permission_list_for_directory_blocking(
        &self,
    ) -> Result<Vec<PermissionRequest>, String> {
        block_on_gateway(self.permission_list_for_directory())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn permission_reply_blocking(
        &self,
        request_id: &str,
        reply: PendingPermissionReplyDto,
        reason: Option<String>,
    ) -> Result<(), String> {
        block_on_gateway(self.permission_reply(request_id, reply, reason))
    }
}
