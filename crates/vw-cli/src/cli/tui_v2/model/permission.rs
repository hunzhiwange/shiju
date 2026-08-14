use std::collections::VecDeque;

use serde_json::Value;
use vw_gateway_client::PendingPermissionRequestDto;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PermissionPreviewKind {
    Shell,
    File,
    Network,
    Plan,
    Skill,
    Fallback,
}

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

    pub(crate) fn always_available(&self) -> bool {
        !self.always_patterns.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PermissionDecision {
    Once,
    Always,
    Reject { reason: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PermissionQueue {
    requests: VecDeque<PermissionRequest>,
}

impl PermissionQueue {
    pub(crate) fn replace(&mut self, requests: Vec<PermissionRequest>) {
        self.requests = requests.into();
    }

    pub(crate) fn current(&self) -> Option<&PermissionRequest> {
        self.requests.front()
    }

    pub(crate) fn len(&self) -> usize {
        self.requests.len()
    }

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
