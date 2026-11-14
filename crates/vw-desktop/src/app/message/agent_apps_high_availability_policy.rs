//! Scoped policy drafts; only explicit Save issues a write.
use crate::app::{App, Message, state::AgentAppsContext};
use iced::Task;
use vw_gateway_client::{AgentAppsHighAvailabilityError as Error, vw_api_types::{agent_apps::AppId, agent_apps_high_availability::*}};

#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    pub open: bool,
    pub busy: bool,
    pub loaded: bool,
    pub fields: [String; 9],
    pub error: Option<String>,
    generation: u64,
    scope: Option<FailoverQueueQuery>,
}
impl State {
    pub fn invalidate(&mut self) {
        *self = Self { generation: self.generation.wrapping_add(1), ..Default::default() };
    }
    fn load(&mut self, p: FailoverPolicy) {
        let r = p.retry_timeout;
        let c = p.circuit_breaker;
        self.fields = [r.max_retries.to_string(), r.first_byte_timeout_secs.to_string(),
            r.idle_timeout_secs.to_string(), r.non_streaming_timeout_secs.to_string(),
            c.failure_threshold.to_string(), c.success_threshold.to_string(), c.cooldown_secs.to_string(),
            (c.error_rate_threshold * 100.0).to_string(), c.min_requests.to_string()];
        self.loaded = true;
    }
    fn policy(&self) -> Result<FailoverPolicy, String> {
        fn number<T: std::str::FromStr>(value: &str) -> Result<T, String> {
            value.trim().parse().map_err(|_| "请输入有效数字；范围由服务端校验".into())
        }
        let f = &self.fields;
        let percentage: f64 = number(&f[7])?;
        if !percentage.is_finite() { return Err("错误率必须是有限数字".into()); }
        Ok(FailoverPolicy {
            retry_timeout: RetryTimeoutPolicy { max_retries: number(&f[0])?,
                first_byte_timeout_secs: number(&f[1])?, idle_timeout_secs: number(&f[2])?,
                non_streaming_timeout_secs: number(&f[3])? },
            circuit_breaker: CircuitBreakerPolicy { failure_threshold: number(&f[4])?,
                success_threshold: number(&f[5])?, cooldown_secs: number(&f[6])?,
                error_rate_threshold: percentage / 100.0, min_requests: number(&f[8])? },
        })
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Open(AppId), Close, Reload, Edit(usize, String), Save,
    Reply(AgentAppsContext, u64, u64, FailoverQueueQuery, bool, Result<FailoverPolicy, Error>),
}
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(crate::app::message::settings::SettingsMessage::AppsCatalog(
        crate::app::message::settings::AppsCatalogMessage::HighAvailability(event)))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) {
        app.apps_catalog.high_availability.invalidate();
        return Task::none();
    }
    let client = match crate::app::config::gateway_client() {
        Ok(client) => client,
        Err(error) => return app.show_error_toast(error),
    };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let catalog_generation = app.apps_catalog.generation;
    if let Event::Reply(request, catalog, generation, scope, saving, result) = event {
        let state = &mut app.apps_catalog.high_availability;
        if request != context || catalog != catalog_generation || state.generation != generation
            || state.scope.as_ref() != Some(&scope) || app.project_id.as_deref() != Some(scope.project_id.as_ref())
            || !state.open || !state.busy { return Task::none(); }
        state.busy = false;
        match result {
            Ok(policy) => {
                state.load(policy);
                state.error = None;
                if saving { return app.show_success_toast("高可用策略已保存，新请求生效"); }
            }
            Err(Error::Http { status: 400 | 422, body: Some(body) }) => state.error = Some(body.error.message),
            Err(_) => state.error = Some("策略请求失败，请检查授权与连接；保存结果不明确时请重新读取".into()),
        }
        return Task::none();
    }
    let state = &mut app.apps_catalog.high_availability;
    match event {
        Event::Close => { state.invalidate(); return Task::none(); }
        Event::Edit(index, value) => {
            if !state.busy && state.open && index < 9 { state.fields[index] = value; }
            return Task::none();
        }
        Event::Open(id) => {
            if app.apps_catalog.token.is_empty() || app.apps_catalog.context.as_ref() != Some(&context)
                || !app.apps_catalog.rows.iter().any(|row| row.entry.id == id) {
                return app.show_error_toast("请先连接应用管理");
            }
            let Some(project) = app.project_id.as_deref().filter(|id| !id.is_empty()) else {
                return app.show_error_toast("请先选择项目");
            };
            state.invalidate();
            state.open = true;
            state.scope = Some(FailoverQueueQuery { project_id: project.into(), target_host_id: app.apps_catalog.host.clone(), app_id: id });
        }
        Event::Reload | Event::Save => {
            if !state.open || state.busy { return Task::none(); }
        }
        Event::Reply(..) => unreachable!(),
    }
    let saving = matches!(event, Event::Save);
    let body = if saving {
        match state.policy() {
            Ok(policy) => Some(policy),
            Err(error) => { state.error = Some(error); return Task::none(); }
        }
    } else { state.loaded = false; None };
    let Some(scope) = state.scope.clone() else { return Task::none(); };
    if app.apps_catalog.context.as_ref() != Some(&context)
        || scope.target_host_id != app.apps_catalog.host
        || app.project_id.as_deref() != Some(scope.project_id.as_ref())
        || app.apps_catalog.token.is_empty()
    {
        state.invalidate();
        return app.show_error_toast("连接或项目已更改，请重新打开策略");
    }
    state.busy = true;
    state.error = None;
    let generation = state.generation;
    let token = app.apps_catalog.token.clone();
    Task::perform(async move {
        let result = match body {
            Some(policy) => client.agent_apps_failover_save_policy(&scope, &policy, Some(&token)).await,
            None => client.agent_apps_failover_policy(&scope, Some(&token)).await,
        };
        Event::Reply(context, catalog_generation, generation, scope, saving, result)
    }, message)
}

#[cfg(test)]
#[path = "../components/agent_apps_high_availability/policy_modal_tests.rs"]
mod tests;
