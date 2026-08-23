use std::collections::VecDeque;

use codex_protocol::approvals::GuardianAssessmentAction;
use codex_protocol::approvals::GuardianAssessmentEvent;
use codex_protocol::approvals::GuardianAssessmentStatus;

const MAX_RECENT_DENIALS: usize = 10;

#[derive(Debug, Default)]
/// 按新到旧顺序保存去重后的有限拒绝记录。
pub(crate) struct RecentAutoReviewDenials {
    entries: VecDeque<GuardianAssessmentEvent>,
}

impl RecentAutoReviewDenials {
    pub(crate) fn push(&mut self, event: GuardianAssessmentEvent) {
        // 仅拒绝结果可进入重试列表，其余审查状态不应出现在该界面。
        if event.status != GuardianAssessmentStatus::Denied {
            return;
        }

        // 同一审查 ID 再次到达时移除旧记录，再把最新载荷置顶。
        self.entries.retain(|entry| entry.id != event.id);
        self.entries.push_front(event);
        self.entries.truncate(MAX_RECENT_DENIALS);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = &GuardianAssessmentEvent> {
        self.entries.iter()
    }

    pub(crate) fn take(&mut self, id: &str) -> Option<GuardianAssessmentEvent> {
        let idx = self.entries.iter().position(|entry| entry.id == id)?;
        self.entries.remove(idx)
    }
}

pub(crate) fn action_summary(action: &GuardianAssessmentAction) -> String {
    // 摘要只展示识别操作所需的最少信息，不展开完整敏感载荷。
    match action {
        GuardianAssessmentAction::Command { command, .. } => command.clone(),
        GuardianAssessmentAction::Execve { program, argv, .. } => {
            // `argv` 为空时仍保留程序名；无法安全 shell 转义则使用空格拼接回退。
            let command = if argv.is_empty() {
                vec![program.clone()]
            } else {
                argv.clone()
            };
            shlex::try_join(command.iter().map(String::as_str))
                .unwrap_or_else(|_| command.join(" "))
        }
        GuardianAssessmentAction::ApplyPatch { files, .. } => {
            if files.len() == 1 {
                format!("apply_patch touching {}", files[0].display())
            } else {
                format!("apply_patch touching {} files", files.len())
            }
        }
        GuardianAssessmentAction::NetworkAccess { target, .. } => {
            format!("network access to {target}")
        }
        GuardianAssessmentAction::McpToolCall {
            server,
            tool_name,
            connector_name,
            ..
        } => {
            let label = connector_name.as_deref().unwrap_or(server.as_str());
            format!("MCP {tool_name} on {label}")
        }
        GuardianAssessmentAction::RequestPermissions { reason, .. } => reason
            .as_deref()
            .map(|reason| format!("permission request: {reason}"))
            .unwrap_or_else(|| "permission request".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use codex_protocol::approvals::GuardianCommandSource;
    use codex_utils_absolute_path::test_support::PathBufExt;
    use codex_utils_absolute_path::test_support::test_path_buf;
    use pretty_assertions::assert_eq;

    use super::*;

    fn denied_event(id: usize) -> GuardianAssessmentEvent {
        // 构造不同 ID 的最小拒绝事件，用于验证队列顺序和容量边界。
        GuardianAssessmentEvent {
            id: format!("review-{id}"),
            target_item_id: None,
            plugin_id: None,
            script_path: None,
            turn_id: "turn-1".to_string(),
            started_at_ms: 0,
            completed_at_ms: Some(1),
            status: GuardianAssessmentStatus::Denied,
            risk_level: None,
            user_authorization: None,
            rationale: Some(format!("rationale {id}")),
            decision_source: None,
            action: GuardianAssessmentAction::Command {
                source: GuardianCommandSource::Shell,
                command: format!("rm -rf /tmp/test-{id}"),
                cwd: test_path_buf("/tmp").abs(),
            },
        }
    }

    #[test]
    // 超过容量后应只保留最新十条，且迭代顺序保持从新到旧。
    fn keeps_only_ten_most_recent_denials() {
        let mut denials = RecentAutoReviewDenials::default();
        for id in 0..12 {
            denials.push(denied_event(id));
        }

        let ids = denials
            .entries()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                "review-11",
                "review-10",
                "review-9",
                "review-8",
                "review-7",
                "review-6",
                "review-5",
                "review-4",
                "review-3",
                "review-2",
            ]
        );
    }
}
//! 缓存最近被自动审查拒绝的操作，并为审批界面生成简短摘要。
