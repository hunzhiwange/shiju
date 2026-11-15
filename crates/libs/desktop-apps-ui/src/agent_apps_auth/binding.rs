//! Supplier-to-account selection. The execution host owns the binding transaction.
use gateway_app_auth::AgentAppsAuthError;
use gateway_app_auth::AgentAppsAuthCredentials;

use super::{Event, message, read_error_label};
use desktop_settings_account::types::{Binding, Connection, Scope};
use desktop_provider_apps::state::{claude::Kind, edit::{EditForm, Mode}};
use super::state::{State, Usage};
use crate::{Notice, error, success};
type Message = Event;
pub struct Origin<'a> {
    pub visible_apps: bool,
    pub visible_auth: bool,
    pub provider_generation: u64,
    pub provider_scope: Option<&'a desktop_provider_apps::Scope>,
    pub form: Option<&'a EditForm>,
    pub project_path: &'a Option<String>,
    pub token: &'a str,
    pub host: &'a str,
    pub catalog_generation: u64,
}
#[derive(Default)]
pub struct Effects { pub opened: bool, pub saved: bool }
use iced::Task;
use {gateway_endpoint::{GatewayEndpoint}, {vw_gateway_client::GatewayClient, vw_api_app_identity::AppId}};
use vw_api_app_auth::{
            AuthAccount, AuthCredentialState, AuthErrorCode, AuthRegion, AuthService,
        };

pub fn entry(form: &EditForm, app: AppId, provider_generation: u64) -> Option<Message> {
    service(form, app)?;
    Some(message(Event::OpenBinding { provider_id: form.id.clone(), provider_generation }))
}

fn service(form: &EditForm, app: AppId) -> Option<(AuthService, Option<AuthRegion>)> {
    if !matches!(form.mode, Mode::Update) || form.saving {
        return None;
    }
    let service = match app {
        AppId::Claude if form.claude.as_ref().is_some_and(|draft| draft.kind == Kind::Official) => {
            AuthService::ClaudeOfficial
        }
        AppId::Codex if form.codex_can_bind => AuthService::CodexOauth,
        AppId::Agy
            if form
                .agy
                .as_ref()
                .is_some_and(|draft| !draft.has_saved_key && draft.base_url.is_empty()) =>
        {
            AuthService::AgyOauth
        }
        AppId::Kimi => return Some((AuthService::KimiOauth, Some(form.kimi_auth_region?))),
        _ => return None,
    };
    Some((service, None))
}

fn origin_matches(origin_input: &Origin<'_>, scope: &Scope, provider_generation: u64) -> bool {
    origin_input.provider_generation == provider_generation
        && !origin_input.token.trim().is_empty()
        && origin_input.provider_scope.is_some_and(|origin| {
            origin.context.gateway == scope.gateway
                && origin.context.tenant == scope.tenant
                && origin.context.project == *origin_input.project_path
                && !origin.host.trim().is_empty()
                && origin.host == origin_input.host
        })
}

pub fn connection_matches(
    binding: &Binding,
    connection: &Connection,
    supplier_token: &str,
) -> bool {
    connection.valid_management()
        && connection.host.trim() == binding.host
        && connection.token.trim() == supplier_token.trim()
}

fn credentials(connection: &Connection) -> AgentAppsAuthCredentials<'_> {
    AgentAppsAuthCredentials {
        username: connection.username.trim(),
        password: &connection.password,
        principal_token: connection.token.trim(),
    }
}

fn compatible(binding: &Binding, account: &AuthAccount) -> bool {
    account.service == binding.service
        && account.credential_state == AuthCredentialState::Ready
        && !account.account_id.is_empty()
        && binding.region.is_none_or(|region| account.region == Some(region))
}

pub fn update<U: Usage>(state: &mut State<U>, notice: &mut Option<Notice>, effects: &mut Effects, origin_input: Origin<'_>, event: Event, endpoint: GatewayEndpoint) -> Task<Message> {
    let scope = Scope::from(&endpoint);
    if let Event::OpenBinding { provider_id, provider_generation } = event {
        if !origin_input.visible_apps {
            return Task::none();
        }
        if !endpoint.is_loopback() {
            return error(notice, "选择授权账号需要本机回环管理连接");
        }
        let candidate = origin_input.provider_scope.and_then(|origin| {
            let form = origin_input.form?;
            (form.id == provider_id && origin_matches(&origin_input, &scope, provider_generation))
                .then(|| service(form, origin.app))?
                .map(|(service, region)| {
                    (origin.host.clone(), form.fields.name.clone(), service, region)
                })
        });
        let Some((host, provider_name, service, region)) = candidate else {
            return error(notice, "供应商已变化或不支持账号绑定，请重新打开编辑页");
        };
        state.form = None;
        state.login = None;
        state.binding_generation = state.binding_generation.wrapping_add(1);
        let generation = state.binding_generation;
        state.focused_service = Some(service);
        state.binding = Some(Binding {
            generation,
            provider_generation,
            catalog_generation: origin_input.catalog_generation,
            provider_id,
            provider_name,
            host,
            service,
            region,
            accounts: Vec::new(),
            connected: false,
            selected: None,
            loading: false,
            saving: false,
            error: None,
        });
        effects.opened = true;
        return update(state, notice, effects, origin_input, Event::LoadBinding(generation), endpoint);
    }

    if !(origin_input.visible_auth || effects.opened)
        || !endpoint.is_loopback()
    {
        return Task::none();
    }
    let generation = match event {
        Event::LoadBinding(generation)
        | Event::BindingLoaded(generation, _)
        | Event::SelectBinding(generation, _)
        | Event::SaveBinding(generation)
        | Event::BindingSaved(generation, _)
        | Event::CloseBinding(generation) => generation,
        _ => unreachable!("expected binding event"),
    };
    let Some(binding) = state.binding.as_ref().filter(|b| b.generation == generation)
    else {
        return Task::none();
    };
    if !origin_matches(&origin_input, &scope, binding.provider_generation)
        || origin_input.catalog_generation != binding.catalog_generation
        || origin_input.form.is_none_or(|form| form.id != binding.provider_id)
    {
        state.binding = None;
        return error(notice, "供应商连接已变化，请重新打开编辑页");
    }
    let binding = state.binding.as_mut().expect("validated binding");
    if let Event::CloseBinding(_) = event {
        if !binding.saving {
            state.binding = None;
            state.form = None;
            if state.rows.is_none() && state.connection.is_some() {
                return Task::done(message(Event::Refresh));
            }
        }
        return Task::none();
    }
    let connection = state
        .connection
        .as_ref()
        .filter(|connection| connection_matches(binding, connection, &origin_input.token));
    let Some(connection) = connection else {
        return Task::none();
    };
    binding.connected = true;
    match event {
        Event::LoadBinding(_) if !binding.saving && !binding.loading => {
            let Ok(client) = GatewayClient::new(endpoint) else {
                binding.error = Some("无法建立管理连接，请重试");
                return Task::none();
            };
            binding.accounts.clear();
            binding.selected = None;
            binding.error = None;
            binding.loading = true;
            let connection = connection.clone();
            let service = binding.service;
            return Task::perform(
                async move {
                    gateway_app_auth::GatewayAppAuthClient::new(client.transport().clone())
                        .agent_apps_auth_accounts(
                            connection.host.trim(),
                            service,
                            &credentials(&connection),
                        )
                        .await
                        .map_err(read_error_label)
                },
                move |result| message(Event::BindingLoaded(generation, result)),
            );
        }
        Event::BindingLoaded(_, result) if binding.loading => {
            binding.loading = false;
            match result {
                Ok(mut accounts) => {
                    accounts.retain(|account| compatible(binding, account));
                    binding.accounts = accounts;
                }
                Err(error) => binding.error = Some(error),
            }
        }
        Event::SelectBinding(_, account_id) if !binding.loading && !binding.saving => {
            binding.selected = binding
                .accounts
                .iter()
                .any(|account| account.account_id == account_id && compatible(binding, account))
                .then_some(account_id);
        }
        Event::SaveBinding(_) if !binding.loading && !binding.saving => {
            let Some(id) = binding.selected.as_ref().filter(|id| {
                binding
                    .accounts
                    .iter()
                    .any(|account| &account.account_id == *id && compatible(binding, account))
            }) else {
                return Task::none();
            };
            let Ok(client) = GatewayClient::new(endpoint) else {
                binding.error = Some("保存失败：无法建立管理连接");
                return error(notice, "保存失败：无法建立管理连接");
            };
            binding.saving = true;
            binding.error = None;
            let connection = connection.clone();
            let (provider, service, id) =
                (binding.provider_id.clone(), binding.service, id.clone());
            return Task::perform(
                async move {
                    gateway_app_auth::GatewayAppAuthClient::new(client.transport().clone())
                        .agent_apps_auth_bind(
                            connection.host.trim(),
                            service,
                            &credentials(&connection),
                            &provider,
                            Some(&id),
                        )
                        .await
                        .map_err(save_error)
                },
                move |result| message(Event::BindingSaved(generation, result)),
            );
        }
        Event::BindingSaved(_, result) if binding.saving => {
            binding.saving = false;
            match result {
                Ok(()) => {
                    state.binding = None;
                    state.usage.clear();
                    effects.saved = true;
                    let refresh = state.rows.is_none();
                    let toast = success(notice, "已保存供应商授权账号");
                    return if refresh {
                        Task::batch([toast, Task::done(message(Event::Refresh))])
                    } else {
                        toast
                    };
                }
                Err(error) => {
                    binding.error = Some(error);
                    return error(notice, error);
                }
            }
        }
        _ => {}
    }
    Task::none()
}

fn save_error(error: AgentAppsAuthError) -> &'static str {
    use AuthErrorCode::*;
    match error {
        AgentAppsAuthError::Request(code) | AgentAppsAuthError::Http { code: Some(code), .. } => {
            match code {
                Unauthenticated | Forbidden => "保存失败：当前管理连接没有账号绑定权限",
                NotFound => "保存失败：供应商或账号已不在当前范围",
                ReauthRequired => "保存失败：所选账号需要重新授权",
                Unsupported => "保存失败：执行主机不支持此供应商绑定",
                IdentityMismatch => "保存失败：账号与供应商的地区或身份不兼容",
                Conflict | AccountInUse => "保存失败：账号或供应商状态已变化，请刷新后重试",
                ExecutionHostUnavailable | ExecutionChannelUnavailable => {
                    "保存失败：无法连接授权执行主机"
                }
                _ => "保存失败：执行主机拒绝绑定，请重试",
            }
        }
        AgentAppsAuthError::Http { status: 401 | 403, .. }
        | AgentAppsAuthError::InvalidPrincipalToken => "保存失败：当前管理连接没有账号绑定权限",
        AgentAppsAuthError::Transport => "未能确认保存结果，请检查管理连接后重试",
        _ => "未能确认保存结果，请刷新后重试",
    }
}
