//! Scoped policy drafts; only explicit Save issues a write.
use crate::AgentAppsContext;
use iced::Task;
use gateway_app_failover::AgentAppsHighAvailabilityError as Error;
use vw_api_app_identity::AppId;
use vw_api_app_failover::*;

#[derive(Clone, Debug, Default)]
pub struct State {
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
pub struct Input<'a> {
    pub state: &'a mut State,
    pub client: vw_gateway_client::GatewayClient,
    pub context: AgentAppsContext,
    pub catalog_generation: u64,
    pub catalog_context: Option<&'a AgentAppsContext>,
    pub project: Option<&'a str>,
    pub host: &'a str,
    pub token: &'a str,
    pub requested_app_exists: bool,
}
pub fn update(input: Input<'_>, event: Event, notice: &mut Option<crate::Notice>) -> Task<Event> {
    if matches!(event, Event::Close) {
        input.state.invalidate();
        return Task::none();
    }
    let context = input.context;
    let client = input.client;
    let client = gateway_app_failover::GatewayAppFailoverClient::new(client.transport().clone());
    let catalog_generation = input.catalog_generation;
    if let Event::Reply(request, catalog, generation, scope, saving, result) = event {
        let state = &mut *input.state;
        if request != context || catalog != catalog_generation || state.generation != generation
            || state.scope.as_ref() != Some(&scope) || input.project != Some(scope.project_id.as_ref())
            || !state.open || !state.busy { return Task::none(); }
        state.busy = false;
        match result {
            Ok(policy) => {
                state.load(policy);
                state.error = None;
                if saving { return crate::success(notice, "高可用策略已保存，新请求生效"); }
            }
            Err(Error::Http { status: 400 | 422, body: Some(body) }) => state.error = Some(body.error.message),
            Err(_) => state.error = Some("策略请求失败，请检查授权与连接；保存结果不明确时请重新读取".into()),
        }
        return Task::none();
    }
    let state = &mut *input.state;
    match event {
        Event::Close => { state.invalidate(); return Task::none(); }
        Event::Edit(index, value) => {
            if !state.busy && state.open && index < 9 { state.fields[index] = value; }
            return Task::none();
        }
        Event::Open(id) => {
            if input.catalog_context != Some(&context)
                || !input.requested_app_exists {
                return crate::error(notice, "请先连接应用管理");
            }
            let Some(project) = input.project.filter(|id| !id.is_empty()) else {
                return crate::error(notice, "请先选择项目");
            };
            state.invalidate();
            state.open = true;
            state.scope = Some(FailoverQueueQuery { project_id: project.into(), target_host_id: input.host.to_string(), app_id: id });
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
    if input.catalog_context != Some(&context)
        || scope.target_host_id != input.host
        || input.project != Some(scope.project_id.as_ref())
    {
        state.invalidate();
        return crate::error(notice, "连接或项目已更改，请重新打开策略");
    }
    state.busy = true;
    state.error = None;
    let generation = state.generation;
    let token = input.token.to_string();
    Task::perform(async move {
        let result = match body {
            Some(policy) => client.agent_apps_failover_save_policy(&scope, &policy, Some(&token)).await,
            None => client.agent_apps_failover_policy(&scope, Some(&token)).await,
        };
        Event::Reply(context, catalog_generation, generation, scope, saving, result)
    }, |event| event)
}


#[cfg(test)]
#[path = "policy_state_tests.rs"]
mod tests;
