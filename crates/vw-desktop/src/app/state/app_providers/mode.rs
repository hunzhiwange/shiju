use vw_gateway_client::{
    AgentAppsReadError,
    vw_api_types::app_providers::{
        mode_apply::ModeApplyResult,
        mode_state::{ProviderMode, ProviderModeStatus},
    },
};

pub(crate) const SCROLL_ID: &str = "provider-mode-scroll";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    Read,
    RestoreDirect,
}

#[derive(Clone, Debug)]
pub enum Outcome {
    Observed(ProviderModeStatus),
    Applied(ModeApplyResult),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Form {
    pub status: Option<ProviderModeStatus>,
    pub confirming_direct: bool,
    pub pending: Option<(u64, Operation)>,
    pub error: Option<&'static str>,
    pub needs_refresh: bool,
}

impl Form {
    pub fn writing(&self) -> bool {
        self.pending.is_some_and(|(_, work)| work == Operation::RestoreDirect)
    }

    pub fn can_restore_direct(&self) -> bool {
        self.pending.is_none()
            && !self.needs_refresh
            && self.status.as_ref().is_some_and(|status| {
                (status.mode == ProviderMode::Proxy || status.attached)
                    && status.direct_provider.is_some()
            })
    }

    pub fn finish(
        &mut self,
        generation: u64,
        result: Result<Outcome, AgentAppsReadError>,
    ) -> Option<ModeApplyResult> {
        let (expected, work) = self.pending?;
        if generation != expected {
            return None;
        }
        self.pending = None;
        match (work, result) {
            (Operation::Read, Ok(Outcome::Observed(status))) => {
                self.status = Some(status);
                self.needs_refresh = false;
            }
            (Operation::RestoreDirect, Ok(Outcome::Applied(result))) if !result.attached => {
                self.confirming_direct = false;
                return Some(result);
            }
            (_, Err(AgentAppsReadError::Http { status: 401 | 403, .. })) => {
                self.error = Some("执行主机拒绝了此操作。");
            }
            (Operation::RestoreDirect, Err(AgentAppsReadError::Http { status: 422, .. })) => {
                self.error = Some("执行主机未通过直连配置校验，请检查供应商后重试。");
            }
            (Operation::RestoreDirect, _) => {
                // A conflict can also mean a pending file transaction was recovered.
                self.needs_refresh = true;
                self.error = Some("无法确认切换结果，请重新读取模式状态。");
            }
            (Operation::Read, _) => {
                self.error = Some("无法读取模式状态，请重试。");
            }
        }
        None
    }
}
