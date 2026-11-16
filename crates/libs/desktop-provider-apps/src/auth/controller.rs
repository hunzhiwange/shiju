//! Complete authorization transition controller with explicit host effects.
use super::{
    binding::{self, Effects, Origin},
    state::{State, Usage},
    *,
};
use crate::Notice;
use desktop_settings_account::types::{Connection, LoginPhase, Scope, now};
use iced::Task;

/// Native resources remain owned by the desktop process.
pub trait Ports {
    fn store_preference(&self, open: bool) -> Result<(), &'static str>;
    fn deliver(&self, action: browser::LinkAction, url: &reqwest::Url) -> Result<(), &'static str>;
    fn connect_auth(
        &self,
        client: GatewayClient,
    ) -> impl std::future::Future<Output = Result<local_app_auth::Credentials, String>> + Send + 'static;
}

pub fn update<U: Usage>(
    state: &mut State<U>,
    notice: &mut Option<Notice>,
    effects: &mut Effects,
    origin: Origin<'_>,
    catalog_context: Option<&crate::AgentAppsContext>,
    ports: &impl Ports,
    event: Event,
    endpoint: gateway_endpoint::GatewayEndpoint,
) -> Task<Event> {
    let visible = origin.visible_auth;
    let scope = Scope::from(&endpoint);
    state.select_endpoint(&endpoint);

    if let Event::AccountAction(action) = event {
        return super::actions::update(state, notice, action, endpoint, scope, visible);
    }

    if matches!(
        event,
        Event::OpenBinding { .. }
            | Event::LoadBinding(_)
            | Event::BindingLoaded(..)
            | Event::SelectBinding(..)
            | Event::SaveBinding(_)
            | Event::BindingSaved(..)
            | Event::CloseBinding(_)
    ) {
        if state.account_action.is_some() {
            return Task::none();
        }
        return binding::update(state, notice, effects, origin, event, endpoint);
    }

    if let Event::Loaded { scope: loaded_scope, generation, rows } = event {
        let failed = rows.iter().all(|row| row.accounts.is_err());
        if state.finish(&loaded_scope, generation, rows) && failed && visible {
            return crate::error(notice, "授权账号读取失败，请检查各服务的提示");
        }
        return Task::none();
    }

    if let Event::OpenBrowserPreference(open) = event {
        if visible {
            state.copy_login_link = !open;
            if let Err(error) = ports.store_preference(open) {
                return crate::error(notice, error);
            }
        }
        return Task::none();
    }

    // GatewayEndpoint currently uses HTTP. Keep management credentials on the direct loopback.
    if !state.loopback {
        return Task::none();
    }

    match event {
        Event::AccountAction(_) => unreachable!("account action events handled above"),
        Event::OpenBinding { .. }
        | Event::LoadBinding(_)
        | Event::BindingLoaded(..)
        | Event::SelectBinding(..)
        | Event::SaveBinding(_)
        | Event::BindingSaved(..)
        | Event::CloseBinding(_) => unreachable!("binding events handled above"),
        Event::Open => {
            if visible
                && (state.pending.is_none() || state.binding.is_some())
                && state.form.is_none()
                && state.login.is_none()
                && state.account_action.is_none()
            {
                let mut form = state.connection.clone().unwrap_or_else(|| Connection {
                    username: "vibewindow".into(),
                    ..Connection::default()
                });

                if state.connection.is_none()
                    && catalog_context.as_ref().is_some_and(|context| {
                        context.gateway == scope.gateway && context.tenant == scope.tenant
                    })
                {
                    form.host = origin.host.to_owned();
                    form.token = origin.token.to_owned();
                }
                if let Some(binding) = &state.binding {
                    form.host.clone_from(&binding.host);
                    form.token = origin.token.to_owned();
                }
                state.form = Some(form);
            }
        }
        Event::Close => state.form = None,
        Event::LocalReady { scope: ready_scope, generation, result } => {
            if scope != ready_scope || state.pending != Some(generation) {
                return Task::none();
            }
            state.pending = None;
            match result {
                Ok(credentials) => {
                    state.connection = Some(Connection::local(credentials));
                    return update(
                        state,
                        notice,
                        effects,
                        origin,
                        catalog_context,
                        ports,
                        Event::Refresh,
                        endpoint,
                    );
                }
                Err(error) => {
                    state.connection_error = Some(error.clone());
                    if visible {
                        return crate::error(notice, error);
                    }
                }
            }
        }
        Event::Host(value) => {
            if state.binding.is_none()
                && let Some(form) = &mut state.form
            {
                form.host = value;
            }
        }
        Event::Username(value) => {
            if let Some(form) = &mut state.form {
                form.username = value;
            }
        }
        Event::Password(value) => {
            if let Some(form) = &mut state.form {
                form.password = value;
            }
        }
        Event::Token(value) => {
            if state.binding.is_none()
                && let Some(form) = &mut state.form
            {
                form.token = value;
            }
        }
        Event::Connect | Event::Refresh | Event::RetryService(_) => {
            if !visible || state.account_action.is_some() || state.login.is_some() {
                return Task::none();
            }
            if matches!(event, Event::RetryService(_)) && state.form.is_some() {
                return Task::none();
            }
            if let Some(binding) = &state.binding {
                let generation = binding.generation;
                if matches!(event, Event::Connect)
                    && !binding.loading
                    && !binding.saving
                    && state.form.as_ref().is_some_and(|form| {
                        binding::connection_matches(binding, form, origin.token)
                    })
                {
                    state.connection = state.form.take();
                    state.usage.clear();
                    state.rows = None;
                    state.pending = None;
                    return binding::update(
                        state,
                        notice,
                        effects,
                        origin,
                        Event::LoadBinding(generation),
                        endpoint,
                    );
                }
                return Task::none();
            }
            if state.pending.is_some() {
                return Task::none();
            }
            if matches!(event, Event::Connect) {
                if !state.form.as_ref().is_some_and(Connection::valid_management) {
                    return Task::none();
                }
                state.connection = state.form.take();
            }
            let Ok(client) = GatewayClient::new(endpoint) else {
                return crate::error(notice, "无法建立管理连接，请重试");
            };
            let Some(connection) = state.connection.clone() else {
                if !cfg!(unix) {
                    return Task::none();
                }
                let generation = state.begin();
                return Task::perform(ports.connect_auth(client), move |result| {
                    message(Event::LocalReady { scope, generation, result })
                });
            };
            let retry = if let Event::RetryService(service) = event {
                let Some(rows) = state.rows.clone().filter(|rows| {
                    rows.iter().any(|row| row.service == service && row.accounts.is_err())
                }) else {
                    return Task::none();
                };
                Some((service, rows))
            } else {
                None
            };
            let generation = state.begin();
            return Task::perform(
                async move {
                    let credentials = AgentAppsAuthCredentials {
                        username: connection.username.trim(),
                        password: &connection.password,
                        principal_token: connection.token.trim(),
                    };
                    if let Some((service, mut rows)) = retry {
                        let refreshed =
                            load_service(&client, connection.host.trim(), &credentials, service)
                                .await;
                        if let Some(row) = rows.iter_mut().find(|row| row.service == service) {
                            *row = refreshed;
                        }
                        rows
                    } else {
                        load(&client, connection.host.trim(), &credentials).await
                    }
                },
                move |rows| message(Event::Loaded { scope, generation, rows }),
            );
        }
        Event::StartLogin(_) | Event::RetryLogin => {
            if !visible
                || state.form.is_some()
                || state.pending.is_some()
                || state.binding.is_some()
                || state.account_action.is_some()
            {
                return Task::none();
            }
            let target = match (&state.login, event) {
                (None, Event::StartLogin(target)) => target,
                (Some(login), Event::RetryLogin)
                    if matches!(login.phase, LoginPhase::Failed(_)) =>
                {
                    login.target
                }
                _ => return Task::none(),
            };
            let Some(connection) = state.connection.clone() else { return Task::none() };
            let Ok(client) = GatewayClient::new(endpoint) else {
                return crate::error(notice, "无法建立管理连接，请重试");
            };
            return start(state, scope, client, connection, target);
        }
        Event::CloseLogin => state.login = None,
        Event::LoginTick => {}
        Event::LoginLink(generation, action) if visible => {
            if let Some(login) = state.login_mut(generation)
                && let LoginPhase::Pending(challenge) = &login.phase
                && challenge.expires_at.0 > now().0
            {
                let url = challenge.verification_url.clone();
                return deliver_link(notice, ports, action, url);
            }
        }
        Event::PasteInput(generation, input) if visible => {
            browser::edit_paste(state.login_mut(generation), input);
        }
        Event::PasteSubmit(generation) if visible => {
            if let Err(error) = browser::submit_paste(state.login_mut(generation)) {
                return crate::error(notice, error);
            }
        }
        Event::LoginLink(..) | Event::PasteInput(..) | Event::PasteSubmit(_) => {}
        Event::LoginUpdated { scope, generation, update } => {
            if !visible || !state.is_latest_login_update(&scope, generation) {
                return Task::none();
            }
            if matches!(update, flow::Update::CancelFailed) {
                return crate::error(notice, "执行主机未确认取消授权登录");
            }
            let action = LinkAction::for_preference(state.copy_login_link);
            let Some(login) = state.login_mut(generation) else { return Task::none() };
            match update {
                flow::Update::Started(challenge) => {
                    if matches!(login.phase, LoginPhase::Starting) {
                        let active = challenge.expires_at.0 > now().0;
                        let url = challenge.verification_url.clone();
                        login.phase = LoginPhase::Pending(challenge);
                        if active {
                            return deliver_link(notice, ports, action, url);
                        }
                    }
                }
                flow::Update::Pending(expires_at) => {
                    if let LoginPhase::Pending(challenge) = &mut login.phase {
                        challenge.expires_at = expires_at;
                    }
                }
                flow::Update::Approved => {
                    let label = if login.target.is_device() {
                        "设备码登录成功"
                    } else {
                        "授权登录成功"
                    };
                    state.login = None;
                    return Task::batch([
                        crate::success(notice, label),
                        Task::done(message(Event::Refresh)),
                    ]);
                }
                flow::Update::Failed(reason) => {
                    login.paste_input.clear();
                    login.phase = LoginPhase::Failed(reason);
                    return crate::error(notice, reason);
                }
                flow::Update::PasteFinished(result) => {
                    login.paste_submitting = false;
                    return match result {
                        Ok(()) => crate::success(notice, "已提交回填，正在等待执行主机确认登录"),
                        Err(error) => crate::error(notice, error),
                    };
                }
                flow::Update::CancelFailed => unreachable!("cancellation errors handled above"),
            }
        }
        Event::Loaded { .. } | Event::OpenBrowserPreference(_) => {
            unreachable!("local events handled above")
        }
    }
    Task::none()
}

fn deliver_link(
    notice: &mut Option<Notice>,
    ports: &impl Ports,
    action: browser::LinkAction,
    url: reqwest::Url,
) -> Task<Event> {
    match ports.deliver(action, &url) {
        Ok(()) if matches!(action, browser::LinkAction::Copy) => {
            crate::success(notice, "已复制授权链接")
        }
        Ok(()) => Task::none(),
        Err(error) => crate::error(notice, error),
    }
}
fn start<U: Usage>(
    state: &mut State<U>,
    scope: Scope,
    client: GatewayClient,
    connection: Connection,
    target: LoginTarget,
) -> Task<Event> {
    let copy_link = state.copy_login_link;
    let (generation, cancelled, pastes) = state.begin_login(target);
    Task::run(
        iced::stream::channel(4, async move |updates| {
            flow::run(client, connection, target, copy_link, cancelled, pastes, updates).await;
        }),
        move |update| message(Event::LoginUpdated { scope: scope.clone(), generation, update }),
    )
}
