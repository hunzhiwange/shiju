//! Authorized production observations; diagnostics never mutate this state.
use desktop_provider_apps::Scope;
use iced::Task;
use {
    vw_api_app_failover::*,
    vw_api_transport::id::ProviderId,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub scope: Scope,
    pub query: FailoverQueueQuery,
    pub generation: u64,
    pub access: u64,
    pub provider: ProviderId,
}
#[derive(Clone, Debug, Default)]
pub struct State {
    pub context: Option<Context>,
    pub serial: u64,
    pub busy: bool,
    pub health: Option<FailoverHealthResponse>,
    pub route: Option<FailoverRouteResponse>,
    pub error: Option<String>,
}
#[derive(Clone, Debug)]
pub enum Event {
    Open(String),
    Refresh,
    Reset,
    Poll(Context, u64),
    Loaded(Context, u64, Result<(FailoverHealthResponse, FailoverRouteResponse), String>),
}
impl State {
    pub fn accept(&mut self, context: &Context, serial: u64, health: FailoverHealthResponse, route: FailoverRouteResponse) -> bool {
        let scope = &health.scope;
        if self.context.as_ref() != Some(context) || serial != self.serial
            || health.provider_id != context.provider || route.scope != *scope
            || scope.project.project_id() != &context.query.project_id
            || scope.project.tenant_scope() != &context.scope.context.tenant
            || scope.target_host_id != context.query.target_host_id || scope.app_id != context.query.app_id {
            return false;
        }
        self.health = Some(health);
        self.route = Some(route);
        true
    }
}
pub fn update(state: &mut State, current: Option<Context>, polling: bool, client: Result<vw_gateway_client::GatewayClient, String>, token: &str, event: Event, notice: &mut Option<crate::Notice>) -> Task<Event> {
    if let Event::Open(_) = &event {
        let Some(context) = current.clone() else { return Task::none(); };
        let serial = state.serial.wrapping_add(1);
        *state = State { context: Some(context), serial, ..Default::default() };
        return update(state, current, polling, client, token, Event::Refresh, notice);
    }
    let Some(context) = state.context.clone() else { return Task::none(); };
    if current.as_ref() != Some(&context) { return Task::none(); }
    match event {
        Event::Loaded(request, serial, result) => {

            if request != context || serial != state.serial || !state.busy { return Task::none(); }
            state.busy = false;
            let accepted = match result {
                Ok((health, route)) => state.accept(&request, serial, health, route),
                Err(_) => false,
            };
            if !accepted {
                state.health = None;
                state.route = None;
                state.error = Some("健康与实际路由未知，请刷新并检查连接和授权".into());
                return crate::error(notice, "健康与实际路由未知，请刷新并检查连接和授权");
            }
            state.error = None;
            return Task::perform(async move {
                sleep(std::time::Duration::from_secs(3)).await;
                Event::Poll(request, serial)
            }, |event| event);
        }
        Event::Poll(request, serial) => {
            if request != context || serial != state.serial
                || !polling {
                return Task::none();
            }
            return update(state, current, polling, client, token, Event::Refresh, notice);
        }
        Event::Refresh | Event::Reset => {
            if state.busy { return Task::none(); }
            let client = match client.clone() { Ok(c) => c, Err(e) => return crate::error(notice, e) };
            let token = token.to_string();
            let reset = matches!(event, Event::Reset);

            state.serial = state.serial.wrapping_add(1);
            state.busy = true;
            state.health = None;
            state.route = None;
            let serial = state.serial;
            return Task::perform(async move {
                let client = gateway_app_failover::GatewayAppFailoverClient::new(client.transport().clone());
                let result = async {
                    let health = if reset {
                        client.agent_apps_failover_reset_health(&context.query, &context.provider, Some(&token)).await
                    } else {
                        client.agent_apps_failover_health(&context.query, &context.provider, Some(&token)).await
                    }.map_err(|_| "健康请求失败".to_string())?;
                    let route = client.agent_apps_failover_route(&context.query, Some(&token)).await.map_err(|_| "路由请求失败".to_string())?;
                    Ok((health, route))
                }.await;
                Event::Loaded(context, serial, result)
            }, |event| event);
        }
        Event::Open(_) => {}
    }
    Task::none()
}


#[cfg(test)]
#[path = "health_state_tests.rs"]
mod tests;
async fn sleep(duration: std::time::Duration) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{JsCast, JsValue, closure::Closure};

        // 将持续时间转换为毫秒，并限制在 i32 范围内
        let millis = duration.as_millis().min(i32::MAX as u128) as i32;

        // 获取浏览器窗口对象，如果失败则直接返回
        let Some(window) = web_sys::window() else {
            return;
        };

        // 创建一个 Promise，在其内部设置 setTimeout
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            // 将 resolve 参数转换为 JavaScript 函数
            let Ok(resolve) = resolve.dyn_into::<js_sys::Function>() else {
                return;
            };

            // 创建一次性回调，在定时器触发时 resolve Promise
            let cb = Closure::once(move || {
                let _ = resolve.call0(&JsValue::NULL);
            });

            // 设置定时器，使用回调作为定时器完成时的处理函数
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                millis,
            );

            // 使用 forget() 让回调在 JavaScript 侧保持存活直到被执行
            // 这是必要的，因为 Rust 无法知道 JavaScript 何时会调用这个回调
            cb.forget();
        });

        // 等待 Promise 完成（即定时器触发）
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::sleep(duration).await;
    }
}
