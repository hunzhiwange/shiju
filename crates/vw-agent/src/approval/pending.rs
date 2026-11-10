use super::{
    ApprovalManager, ApprovalResponse, PendingApprovalError, PendingNonCliApprovalRequest,
};
use chrono::{Duration, Utc};
use std::collections::HashMap;
use uuid::Uuid;

impl ApprovalManager {
    /// 创建一个待处理的非 CLI 审批请求。
    pub fn create_non_cli_pending_request(
        &self,
        tool_name: &str,
        requested_by: &str,
        requested_channel: &str,
        requested_reply_target: &str,
        reason: Option<String>,
        arguments: serde_json::Value,
        message_id: Option<String>,
        call_id: Option<String>,
    ) -> PendingNonCliApprovalRequest {
        let mut pending = self.pending_non_cli_requests.lock();
        prune_expired_pending_requests(&mut pending);

        if let Some(existing) = pending
            .values()
            .find(|req| {
                req.tool_name == tool_name
                    && req.arguments == arguments
                    && req.message_id == message_id
                    && req.call_id == call_id
                    && req.requested_by == requested_by
                    && req.requested_channel == requested_channel
                    && req.requested_reply_target == requested_reply_target
                    && req.reason == reason
            })
            .cloned()
        {
            return existing;
        }

        let now = Utc::now();
        let expires = now + Duration::minutes(30);

        let mut request_id = format!("apr-{}", &Uuid::new_v4().simple().to_string()[..8]);
        while pending.contains_key(&request_id) {
            request_id = format!("apr-{}", &Uuid::new_v4().simple().to_string()[..8]);
        }

        let req = PendingNonCliApprovalRequest {
            request_id: request_id.clone(),
            tool_name: tool_name.to_string(),
            arguments,
            message_id,
            call_id,
            requested_by: requested_by.to_string(),
            requested_channel: requested_channel.to_string(),
            requested_reply_target: requested_reply_target.to_string(),
            reason,
            created_at: now.to_rfc3339(),
            expires_at: expires.to_rfc3339(),
        };
        pending.insert(request_id, req.clone());

        self.resolved_non_cli_requests.lock().remove(&req.request_id);
        req
    }

    /// 确认一个待处理的非 CLI 审批请求。
    pub fn confirm_non_cli_pending_request(
        &self,
        request_id: &str,
        confirmed_by: &str,
        confirmed_channel: &str,
        confirmed_reply_target: &str,
    ) -> Result<PendingNonCliApprovalRequest, PendingApprovalError> {
        self.resolve_pending_request(
            request_id,
            confirmed_by,
            confirmed_channel,
            confirmed_reply_target,
            None,
        )
    }

    /// 拒绝一个待处理的非 CLI 审批请求。
    pub fn reject_non_cli_pending_request(
        &self,
        request_id: &str,
        rejected_by: &str,
        rejected_channel: &str,
        rejected_reply_target: &str,
    ) -> Result<PendingNonCliApprovalRequest, PendingApprovalError> {
        self.resolve_pending_request(
            request_id,
            rejected_by,
            rejected_channel,
            rejected_reply_target,
            None,
        )
    }

    pub(crate) fn reply_to_non_cli_pending_once(
        &self,
        request: &PendingNonCliApprovalRequest,
        approved: bool,
    ) -> Result<(), PendingApprovalError> {
        self.resolve_pending_request(
            &request.request_id,
            &request.requested_by,
            &request.requested_channel,
            &request.requested_reply_target,
            Some(if approved { ApprovalResponse::Yes } else { ApprovalResponse::No }),
        )?;
        Ok(())
    }

    /// 检查指定的待处理非 CLI 请求是否仍存在。
    pub fn has_non_cli_pending_request(&self, request_id: &str) -> bool {
        let mut pending = self.pending_non_cli_requests.lock();
        prune_expired_pending_requests(&mut pending);
        pending.contains_key(request_id)
    }

    /// 记录待处理非 CLI 请求的 Yes/No 决策。
    pub fn record_non_cli_pending_resolution(&self, request_id: &str, decision: ApprovalResponse) {
        if !matches!(decision, ApprovalResponse::Yes | ApprovalResponse::No) {
            return;
        }

        let mut resolved = self.resolved_non_cli_requests.lock();
        if resolved.len() >= 1024 {
            if let Some(first_key) = resolved.keys().next().cloned() {
                resolved.remove(&first_key);
            }
        }
        resolved.insert(request_id.to_string(), decision);
    }

    /// 消费已解析的待处理请求决策（如果存在）。
    pub fn take_non_cli_pending_resolution(&self, request_id: &str) -> Option<ApprovalResponse> {
        self.resolved_non_cli_requests.lock().remove(request_id)
    }

    /// 列出活跃的待处理非 CLI 审批请求。
    pub fn list_non_cli_pending_requests(
        &self,
        requested_by: Option<&str>,
        requested_channel: Option<&str>,
        requested_reply_target: Option<&str>,
    ) -> Vec<PendingNonCliApprovalRequest> {
        let mut pending = self.pending_non_cli_requests.lock();
        prune_expired_pending_requests(&mut pending);

        let mut rows = pending
            .values()
            .filter(|req| {
                requested_by.map_or(true, |by| req.requested_by == by)
                    && requested_channel.map_or(true, |channel| req.requested_channel == channel)
                    && requested_reply_target
                        .map_or(true, |reply_target| req.requested_reply_target == reply_target)
            })
            .cloned()
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        rows
    }

    /// 移除特定工具的所有待处理请求。
    pub fn clear_non_cli_pending_requests_for_tool(&self, tool_name: &str) -> usize {
        let mut pending = self.pending_non_cli_requests.lock();
        prune_expired_pending_requests(&mut pending);
        let mut resolved = self.resolved_non_cli_requests.lock();
        let before = pending.len();

        pending.retain(|request_id, req| {
            let keep = req.tool_name != tool_name;
            if !keep {
                resolved.remove(request_id);
            }
            keep
        });
        before.saturating_sub(pending.len())
    }

    fn resolve_pending_request(
        &self,
        request_id: &str,
        actor: &str,
        channel: &str,
        reply_target: &str,
        decision: Option<ApprovalResponse>,
    ) -> Result<PendingNonCliApprovalRequest, PendingApprovalError> {
        let mut pending = self.pending_non_cli_requests.lock();
        prune_expired_pending_requests(&mut pending);

        let Some(req) = pending.remove(request_id) else {
            return Err(PendingApprovalError::NotFound);
        };

        if is_pending_request_expired(&req) {
            return Err(PendingApprovalError::Expired);
        }

        if req.requested_by != actor
            || req.requested_channel != channel
            || req.requested_reply_target != reply_target
        {
            pending.insert(req.request_id.clone(), req);
            return Err(PendingApprovalError::RequesterMismatch);
        }

        if let Some(decision) = decision {
            self.record_non_cli_pending_resolution(request_id, decision);
        }
        Ok(req)
    }
}

/// 检查待处理请求是否已过期。
pub(super) fn is_pending_request_expired(req: &PendingNonCliApprovalRequest) -> bool {
    chrono::DateTime::parse_from_rfc3339(&req.expires_at)
        .map(|dt| dt.with_timezone(&Utc) <= Utc::now())
        .unwrap_or(true)
}

/// 清理已过期的待处理请求。
pub(super) fn prune_expired_pending_requests(
    pending: &mut HashMap<String, PendingNonCliApprovalRequest>,
) -> usize {
    let before = pending.len();
    pending.retain(|_, req| !is_pending_request_expired(req));
    before.saturating_sub(pending.len())
}
