use crate::ProviderScope as Scope;
use vw_api_transport::ProjectId;
use vw_api_app_aggregation::*;
use gateway_app_settings::AgentAppsSettingsError;
use vw_api_chat_identity::ReasoningEffortDto;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub scope: Scope,
    pub project: ProjectId,
    pub access: u64,
}
#[derive(Clone, Default)]
pub struct Form {
    pub context: Option<Context>,
    pub snapshot: Option<CodexAggregationSettings>,
    pub draft: CodexSettingsValues,
    pub pending: Option<u64>,
    pub error: Option<&'static str>,
    pub saved: bool,
}
impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CodexSubagentForm([redacted])")
    }
}
impl Form {
    pub fn selected(&self) -> Option<&CodexSettingsChoice> {
        let id = self.draft.subagent_model_id.as_ref()?;
        self.snapshot.as_ref()?.choices.iter().find(|c| &c.model_id == id)
    }
    pub fn unavailable(&self) -> bool {
        self.snapshot.is_some()
            && self.draft.subagent_model_id.is_some()
            && self.selected().is_none()
    }
    pub fn select_model(&mut self, id: Option<String>) {
        if let Some(id) = &id {
            let Some(choice) =
                self.snapshot.as_ref().and_then(|s| s.choices.iter().find(|c| &c.model_id == id))
            else {
                return;
            };
            if self.draft.subagent_effort.is_some_and(|e| !choice.efforts.contains(&e)) {
                self.draft.subagent_effort = None;
            }
        } else {
            self.draft.subagent_effort = None;
        }
        self.draft.subagent_model_id = id;
        self.saved = false;
    }
    pub fn select_effort(&mut self, effort: Option<ReasoningEffortDto>) {
        if self.selected().is_some_and(|c| effort.is_none_or(|e| c.efforts.contains(&e))) {
            self.draft.subagent_effort = effort;
            self.saved = false;
        }
    }
    pub fn select_review(&mut self, id: Option<String>) {
        let Some(state) = &self.snapshot else { return };
        if id.as_ref().is_some_and(|id| !state.choices.iter().any(|c| &c.model_id == id)) {
            return;
        }
        self.draft.auto_review_model_id = id;
        self.saved = false;
    }
    pub fn select_classic(&mut self, enabled: bool) {
        if self.snapshot.is_none() {
            return;
        }
        self.draft.classic_tools = enabled;
        self.saved = false;
    }
    pub fn write(&self, context: &Context) -> Option<CodexSettingsRequest> {
        let s = self.snapshot.as_ref().filter(|s| !s.revision.is_empty())?;
        Some(CodexSettingsRequest {
            target_host_id: context.scope.host.clone(),
            project_id: context.project.clone(),
            expected_revision: s.revision.clone(),
            requested: self.draft.clone(),
        })
    }
    pub fn finish(
        &mut self,
        generation: u64,
        saved: bool,
        result: Result<CodexAggregationSettings, AgentAppsSettingsError>,
    ) -> bool {
        if self.pending != Some(generation) {
            return false;
        }
        self.pending = None;
        self.saved = false;
        match result {
            Ok(s) => {
                self.draft = s.requested.clone();
                self.snapshot = Some(s);
                self.error = None;
                self.saved = saved;
            }
            Err(e) => {
                self.snapshot = None;
                self.error = Some(feedback(&e));
            }
        }
        true
    }
}
#[derive(Clone)]
pub enum Event {
    Open,
    Close,
    Refresh,
    Model(Option<String>),
    Effort(Option<ReasoningEffortDto>),
    Review(Option<String>),
    Classic(bool),
    Save,
    Finished {
        context: Context,
        generation: u64,
        saved: bool,
        result: Result<CodexAggregationSettings, AgentAppsSettingsError>,
    },
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CodexSubagentEvent([redacted])")
    }
}
pub fn feedback(e: &AgentAppsSettingsError) -> &'static str {
    match e {
        AgentAppsSettingsError::Http { status: 401 | 403, .. } => {
            "无权访问此主机或项目的 Codex 子代理配置。"
        }
        AgentAppsSettingsError::Http { status: 409, .. } => {
            "配置修订已变化或存在待恢复事务，请刷新后重新选择。"
        }
        AgentAppsSettingsError::Http { status: 400 | 404 | 422, .. } => {
            "模型、强度或目标不受支持，请刷新已发布目录。"
        }
        AgentAppsSettingsError::Http { status: 429, .. } => {
            "外部账号额度或请求频率受限，请检查账号限制后刷新；客户端不能绕过。"
        }
        AgentAppsSettingsError::Http { status: 501, .. } => "当前执行主机不支持此 Codex 设置。",
        AgentAppsSettingsError::Http { status: 502..=504, .. } => {
            "目标不可用、超时或执行失败；请刷新确认实际状态，勿重复提交。"
        }
        _ => "Codex 请求结果未知；请刷新确认实际状态，勿重复提交。",
    }
}
