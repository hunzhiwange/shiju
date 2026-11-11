//! Codex subagent intent; configuration and publication remain gateway-owned.
use crate::app::{
    App, Message,
    message::app_providers::list::{self, ListMessage},
    state::app_providers::list::Scope,
};
use iced::Task;
use vw_gateway_client::{
    AgentAppsSettingsError, GatewayAuth, GatewayClient,
    vw_api_types::{
        ReasoningEffortDto,
        agent_apps::{AppId, aggregation::*},
        id::ProjectId,
    },
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub scope: Scope,
    pub project: ProjectId,
    pub access: u64,
}
#[derive(Clone, Default)]
pub(crate) struct Form {
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
    pub(crate) fn selected(&self) -> Option<&CodexSettingsChoice> {
        let id = self.draft.subagent_model_id.as_ref()?;
        self.snapshot.as_ref()?.choices.iter().find(|c| &c.model_id == id)
    }
    pub(crate) fn unavailable(&self) -> bool {
        self.snapshot.is_some()
            && self.draft.subagent_model_id.is_some()
            && self.selected().is_none()
    }
    pub(crate) fn select_model(&mut self, id: Option<String>) {
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
    pub(crate) fn select_effort(&mut self, effort: Option<ReasoningEffortDto>) {
        if self.selected().is_some_and(|c| effort.is_none_or(|e| c.efforts.contains(&e))) {
            self.draft.subagent_effort = effort;
            self.saved = false;
        }
    }
    pub(crate) fn write(&self, context: &Context) -> Option<CodexSettingsRequest> {
        let s = self.snapshot.as_ref().filter(|s| !s.revision.is_empty())?;
        Some(CodexSettingsRequest {
            target_host_id: context.scope.host.clone(),
            project_id: context.project.clone(),
            expected_revision: s.revision.clone(),
            requested: self.draft.clone(),
        })
    }
    pub(crate) fn finish(
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
pub(crate) fn feedback(e: &AgentAppsSettingsError) -> &'static str {
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
        AgentAppsSettingsError::Http { status: 501, .. } => "当前执行主机不支持此 Codex 设置。",
        AgentAppsSettingsError::Http { status: 502..=504, .. } => {
            "目标不可用、超时或执行失败；请刷新确认实际状态，勿重复提交。"
        }
        _ => "Codex 请求结果未知；请刷新确认实际状态，勿重复提交。",
    }
}
pub(crate) fn message(e: Event) -> Message {
    list::message(ListMessage::CodexSubagent(e))
}
fn current(app: &App) -> Result<(GatewayClient, Context), String> {
    let (client, scope) = list::current(app, AppId::Codex)?;
    let project = app.project_id.as_ref().filter(|p| !p.is_empty()).ok_or("请先打开已注册项目")?;
    if app.app_providers.scope.as_ref() != Some(&scope)
        || scope.host.is_empty()
        || app.apps_catalog.token.is_empty()
    {
        return Err("请先连接已授权应用执行主机".into());
    }
    let endpoint = client
        .endpoint()
        .clone()
        .with_auth(GatewayAuth { skey: Some(app.apps_catalog.token.clone()) });
    Ok((
        GatewayClient::new(endpoint)?,
        Context { scope, project: project.as_str().into(), access: app.apps_catalog.generation },
    ))
}
pub(crate) async fn execute(
    client: GatewayClient,
    context: Context,
    write: Option<CodexSettingsRequest>,
) -> Result<CodexAggregationSettings, AgentAppsSettingsError> {
    match write {
        Some(body) => client.aggregation_codex_settings_save(&body).await,
        None => {
            client
                .aggregation_codex_settings(&AggregationQuery {
                    target_host_id: context.scope.host,
                    project_id: context.project,
                })
                .await
        }
    }
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let (client, context) = match current(app) {
        Ok(v) => v,
        Err(e) => {
            app.app_providers.codex_subagent = None;
            return if matches!(event, Event::Finished { .. }) {
                Task::none()
            } else {
                app.show_error_toast(e)
            };
        }
    };
    if matches!(event, Event::Open) {
        let s = &mut app.app_providers;
        if s.loading || s.form.is_some() || s.editor.is_some() || s.codex_subagent.is_some() {
            return Task::none();
        }
        s.codex_subagent = Some(Form { context: Some(context.clone()), ..Default::default() });
        return update(app, Event::Refresh);
    }
    let state = &mut app.app_providers;
    let Some(form) = &mut state.codex_subagent else { return Task::none() };
    if form.context.as_ref() != Some(&context) {
        state.codex_subagent = None;
        return Task::none();
    }
    if let Event::Finished { context: old, generation, saved, result } = event {
        if old != context || !form.finish(generation, saved, result) {
            return Task::none();
        }
        return if let Some(e) = form.error {
            app.show_error_toast(e)
        } else if saved {
            app.show_info_toast("子代理设置已保存，请新建 Codex 会话后使用。")
        } else {
            Task::none()
        };
    }
    if form.pending.is_some() {
        return Task::none();
    }
    let write = match event {
        Event::Close => {
            state.codex_subagent = None;
            return Task::none();
        }
        Event::Model(id) if form.snapshot.is_some() => {
            form.select_model(id);
            return Task::none();
        }
        Event::Effort(e) if form.snapshot.is_some() => {
            form.select_effort(e);
            return Task::none();
        }
        Event::Refresh => {
            form.snapshot = None;
            None
        }
        Event::Save => match form.write(&context) {
            Some(w) => Some(w),
            None => return app.show_error_toast("请刷新配置后再保存"),
        },
        _ => return Task::none(),
    };
    let saved = write.is_some();
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    form.pending = Some(generation);
    form.error = None;
    form.saved = false;
    Task::perform(execute(client, context.clone(), write), move |result| {
        message(Event::Finished { context: context.clone(), generation, saved, result })
    })
}
