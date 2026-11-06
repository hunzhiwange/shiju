//! 会话访问意图与可证明的权限能力。供应商选项未经固定版本验证时只报告降级。

use agent_client_protocol::{SessionConfigOption, SessionModeState};

use crate::PermissionMode;

pub(crate) mod claude;
pub(crate) mod codex;
pub(crate) mod gemini;
pub(crate) mod qoder;
pub(crate) mod qwen;
pub(crate) mod vibe;

/// 原生权限与客户端回调是不同的执行边界。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullAccessSupport {
    Native,
    ClientOnly,
    Partial,
    Unavailable,
}

/// 可用于 UI 和协议同步的非敏感原因，不包含供应商自由文本。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessPolicyReason {
    NotRequested,
    VerifiedNativeControl,
    DeniedByHost,
    ReadApprovalOnly,
    NoAdvertisedControls,
    UnverifiedControls,
}

/// 只读判定结果；不能通过该结果启用供应商权限。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessPolicy {
    pub full_access_requested: bool,
    pub permission_mode: PermissionMode,
    pub support: FullAccessSupport,
    pub reason: AccessPolicyReason,
}

impl AccessPolicy {
    pub(crate) fn resolve(
        full_access_requested: bool,
        permission_mode: PermissionMode,
        modes: Option<&SessionModeState>,
        config_options: Option<&[SessionConfigOption]>,
    ) -> Self {
        let (support, reason) = match (permission_mode, full_access_requested) {
            (PermissionMode::DenyAll, _) => {
                (FullAccessSupport::Unavailable, AccessPolicyReason::DeniedByHost)
            }
            (_, false) => (FullAccessSupport::Unavailable, AccessPolicyReason::NotRequested),
            (PermissionMode::ApproveReads, true) => {
                (FullAccessSupport::Partial, AccessPolicyReason::ReadApprovalOnly)
            }
            (PermissionMode::ApproveAll, true) => {
                // 通用解析不识别供应商权限语义，公布的选项本身不是验证证据。
                // 后端适配须另外核对运行时身份和当前会话协议回读，才可报告 Native。
                let has_controls = modes.is_some_and(|state| !state.available_modes.is_empty())
                    || config_options.is_some_and(|options| !options.is_empty());
                let reason = if has_controls {
                    AccessPolicyReason::UnverifiedControls
                } else {
                    AccessPolicyReason::NoAdvertisedControls
                };
                (FullAccessSupport::ClientOnly, reason)
            }
        };
        Self { full_access_requested, permission_mode, support, reason }
    }
}

#[cfg(test)]
#[path = "access_policy_tests.rs"]
mod tests;
