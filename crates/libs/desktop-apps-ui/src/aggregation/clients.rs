use gateway_app_settings::AgentAppsSettingsError;
use vw_api_app_aggregation::*;
use vw_api_app_aggregation::operations::ProviderOperationProgress;
use vw_api_task::TaskStatus;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection { pub scope: crate::ProviderScope, pub project: vw_api_transport::ProjectId, pub catalog_generation: u64 }
#[derive(Clone, Debug, Default)]
pub struct Session {
    pub snapshot: Option<CodexClientsState>,
    pub pending: bool,
    pub confirming: bool,
    pub acknowledged: bool,
    pub operation: Option<ProviderOperationProgress>,
    pub error: Option<&'static str>,
}
impl Session {
    pub fn running(&self) -> bool {
        self.operation
            .as_ref()
            .is_some_and(|p| matches!(p.status, TaskStatus::Queued | TaskStatus::Running))
    }
    pub fn can_restart(&self) -> bool {
        !self.pending
            && self.error.is_none()
            && self.operation.is_none()
            && self.snapshot.as_ref().is_some_and(|s| s.daemon && !s.revision.is_empty())
    }
    pub fn begin(&mut self, action: Action) -> bool {
        if self.pending {
            return false;
        }
        let allowed = match action {
            Action::Read => !self.running(),
            Action::Ack => {
                !self.running()
                    && self.operation.is_none()
                    && !self.acknowledged
                    && self.error.is_none()
                    && self
                        .snapshot
                        .as_ref()
                        .is_some_and(|s| s.unverified && !s.revision.is_empty())
            }
            Action::Restart => self.confirming && self.can_restart(),
            Action::Poll => self.operation.is_some(),
            Action::Cancel => {
                self.running() && self.operation.as_ref().is_some_and(|p| !p.cancellation_requested)
            }
        };
        if allowed {
            self.pending = true;
            self.error = None;
            if matches!(action, Action::Read) {
                self.snapshot = None;
                self.acknowledged = false;
                self.operation = None;
                self.confirming = false;
            }
        }
        allowed
    }
    pub fn finish(&mut self, action: Action, reply: Result<Reply, AgentAppsSettingsError>) {
        self.pending = false;
        match reply {
            Ok(Reply::State(state)) => {
                self.acknowledged = matches!(action, Action::Ack)
                    && self.snapshot.as_ref().is_some_and(|old| old.revision == state.revision);
                self.snapshot = Some(state);
            }
            Ok(Reply::Operation(p)) => {
                if p.status == TaskStatus::Failed
                    || matches!(p.aggregation_outcome, Some(AggregationOutcome::Failed { .. }))
                {
                    self.error = Some("重启失败，请重新探测实际状态；不会自动重试。");
                } else if p.status == TaskStatus::Completed
                    && !matches!(p.aggregation_outcome, Some(AggregationOutcome::Restarted { .. }))
                {
                    self.error = Some("重启结果不完整，请重新探测实际状态。");
                }
                self.operation = Some(p);
            }
            Err(e) => {
                self.error = Some(feedback(&e));
                // The request may have reached the host. Require a fresh observation.
                if matches!(action, Action::Ack | Action::Restart) {
                    self.snapshot = None;
                }
            }
        }
    }
    pub fn result_label(&self) -> Option<&'static str> {
        let p = self.operation.as_ref()?;
        if self.error.is_some() {
            return None;
        }
        if p.status == TaskStatus::Completed {
            match p.aggregation_outcome.as_ref()? {
                AggregationOutcome::Restarted { result: CodexRestartResult::Restarted } => {
                    Some("托管守护进程已重启；桌面版与编辑器仍需手动彻底退出重开。")
                }
                AggregationOutcome::Restarted { result: CodexRestartResult::NotRunning } => {
                    Some("守护进程未运行，未执行重启或启动。")
                }
                _ => None,
            }
        } else if p.status == TaskStatus::Cancelled {
            Some("操作已取消；请重新探测客户端状态。")
        } else {
            Some("重启尚未完成；取消请求不代表进程已停止或恢复。")
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Read,
    Ack,
    Restart,
    Poll,
    Cancel,
}
#[derive(Clone)]
pub enum Reply {
    State(CodexClientsState),
    Operation(ProviderOperationProgress),
}
#[derive(Clone)]
pub enum Event {
    Open,
    Close,
    AskRestart,
    Back,
    Act(Action),
    Finished {
        selection: Selection,
        generation: u64,
        action: Action,
        result: Result<Reply, AgentAppsSettingsError>,
    },
}
pub fn feedback(e: &AgentAppsSettingsError) -> &'static str {
    match e {
        AgentAppsSettingsError::Http { status: 401 | 403, .. } => {
            "无权访问或重启此项目的 Codex；重启需要独立授权。"
        }
        AgentAppsSettingsError::Http { status: 409, .. } => {
            "客户端修订或操作冲突，请重新探测后确认。"
        }
        AgentAppsSettingsError::Http { status: 400 | 404 | 422 | 501, .. } => {
            "目标或托管安装身份不受支持，未确认安全时拒绝重启。"
        }
        AgentAppsSettingsError::Http { status: 502, .. } => {
            "目标执行失败，请重新探测，勿重复提交。"
        }
        AgentAppsSettingsError::Http { status: 503, .. } => {
            "执行主机不可用，请恢复连接后重新探测。"
        }
        _ => "结果未知或超时，请重新探测实际状态，勿重复提交。",
    }
}

impl std::fmt::Debug for Reply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CodexClientsReply([redacted])")
    }
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CodexClientsEvent([redacted])")
    }
}

pub use super::clients_update::{execute, update};
