//! Gateway 权限请求的展示分类、审批决定与 FIFO 队列。
//!
//! DTO 中的元数据只用于构建预览和调试信息；是否允许“始终允许”必须以服务端
//! 下发的 `always` 模式为准，不在界面端臆测扩大授权。

use std::collections::VecDeque;

use serde_json::Value;
use vw_gateway_client::PendingPermissionRequestDto;

/// 根据权限名与元数据选择的预览样式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PermissionPreviewKind {
    Shell,
    File,
    Network,
    Plan,
    Skill,
    Fallback,
}

/// 一条待审批权限请求的界面快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PermissionRequest {
    pub(crate) id: String,
    pub(crate) session_id: String,
    pub(crate) permission: String,
    pub(crate) patterns: Vec<String>,
    pub(crate) always_patterns: Vec<String>,
    pub(crate) preview: PermissionPreviewKind,
    pub(crate) explanation: Option<String>,
    pub(crate) debug: Option<String>,
}

impl PermissionRequest {
    /// 将 Gateway DTO 转为视图模型，保留服务端给出的模式边界。
    pub(crate) fn from_dto(dto: PendingPermissionRequestDto) -> Self {
        let preview = preview_kind(&dto.permission, &dto.metadata);
        let explanation =
            dto.metadata.get("explanation").and_then(Value::as_str).map(str::to_string);
        let debug = dto.metadata.get("debug").map(Value::to_string);
        Self {
            id: dto.id,
            session_id: dto.session_id,
            permission: dto.permission,
            patterns: dto.patterns,
            always_patterns: dto.always,
            preview,
            explanation,
            debug,
        }
    }

    /// 返回服务端是否提供可持久授权的模式。
    pub(crate) fn always_available(&self) -> bool {
        !self.always_patterns.is_empty()
    }
}

/// 用户对当前权限请求的决定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PermissionDecision {
    Once,
    Always,
    Reject { reason: Option<String> },
}

/// 按 Gateway 下发顺序展示的权限请求队列。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PermissionQueue {
    requests: VecDeque<PermissionRequest>,
}

impl PermissionQueue {
    /// 用最新服务端快照整体替换本地队列。
    pub(crate) fn replace(&mut self, requests: Vec<PermissionRequest>) {
        self.requests = requests.into();
    }

    /// 查看但不移除队首请求。
    pub(crate) fn current(&self) -> Option<&PermissionRequest> {
        self.requests.front()
    }

    pub(crate) fn len(&self) -> usize {
        self.requests.len()
    }

    /// 移除已处理的队首请求。
    pub(crate) fn pop_current(&mut self) -> Option<PermissionRequest> {
        self.requests.pop_front()
    }
}

fn preview_kind(
    permission: &str,
    metadata: &serde_json::Map<String, Value>,
) -> PermissionPreviewKind {
    let normalized = permission.to_ascii_lowercase();
    if normalized.contains("shell") || normalized.contains("command") {
        PermissionPreviewKind::Shell
    } else if normalized.contains("file")
        || normalized.contains("read")
        || normalized.contains("edit")
        || metadata.contains_key("path")
    {
        PermissionPreviewKind::File
    } else if normalized.contains("network") || normalized.contains("web") {
        PermissionPreviewKind::Network
    } else if normalized.contains("plan") {
        PermissionPreviewKind::Plan
    } else if normalized.contains("skill") {
        PermissionPreviewKind::Skill
    } else {
        PermissionPreviewKind::Fallback
    }
}
