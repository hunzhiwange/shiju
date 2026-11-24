use super::{Event, Page};
use crate::app::{App, Message};
use desktop_session::session::agent::{EMPLOYEE_CHAT_PROFILE, EmployeeKind, EmployeeTarget};
use gateway_endpoint::GatewayEndpoint;
use gateway_shiju_account::{
    GatewayShijuAccountClient, SHIJU_CREDENTIAL_HEADER, ShijuAccountMeDto, ShijuCredential,
};
use iced::Task;
use iced::widget::image::Handle;

#[derive(Clone)]
pub(crate) struct Selection {
    pub target: EmployeeTarget,
    pub name: String,
    pub(super) portrait: Option<Handle>,
}

#[derive(Clone)]
pub(crate) enum ChatIntent {
    Open(Selection),
    Bind(Selection),
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct HistoryLabelRequest {
    session_id: String,
    employee_id: String,
    navigation_generation: u64,
}

#[derive(Clone)]
pub(crate) struct EmployeeAppearance {
    pub(super) name: String,
    pub(super) portrait: Option<Handle>,
}

pub(crate) fn refresh_history_label(app: &mut App, session_id: &str) -> Task<Message> {
    let Some(runtime) = app.session_state.session_runtime_states.get(session_id) else {
        return Task::none();
    };
    if !runtime.is_employee_chat() || runtime.employee_display_name.is_some() {
        return Task::none();
    }
    let Some(employee_id) = runtime.agent_label.as_deref()
        .and_then(|label| label.strip_prefix("数字员工 · "))
        .filter(|id| (1..=128).contains(&id.len()) && id.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')
        }))
    else {
        return Task::none();
    };
    let request = HistoryLabelRequest {
        session_id: session_id.to_owned(),
        employee_id: employee_id.to_owned(),
        navigation_generation: app.session_state.session_view_cache.navigation_generation,
    };
    if app.digital_employees.history_label_request.as_ref() == Some(&request) {
        return Task::none();
    }
    let Some(credential) = credential(app, &crate::app::config::gateway_client_endpoint()) else {
        return Task::none();
    };
    app.digital_employees.history_label_request = Some(request.clone());
    let employee_id = request.employee_id.clone();
    Task::perform(
        async move {
            #[derive(serde::Deserialize)]
            struct EmployeeDetail {
                employee: EmployeeName,
            }
            #[derive(serde::Deserialize)]
            struct EmployeeName {
                employee_id: String,
                name: String,
                visibility: Option<String>,
                agent_kind: Option<String>,
                role_title: Option<String>,
            }
            let client = super::api::Client::new(credential.expose()).ok()?;
            let detail: EmployeeDetail = client.get_path(&["employees", &employee_id], &[]).await.ok()?;
            let name = detail.employee.name.trim();
            (detail.employee.employee_id == employee_id && !name.is_empty()).then(|| {
                let portrait = if detail.employee.visibility.as_deref() == Some("org") {
                    super::portraits::shared_avatar(
                        name,
                        detail.employee.agent_kind.as_deref() != Some("worker"),
                        detail.employee.role_title.as_deref().map(str::trim).unwrap_or_default(),
                    )
                } else {
                    super::portraits::avatar(name, false)
                };
                EmployeeAppearance { name: name.to_owned(), portrait }
            })
        },
        move |name| Message::DigitalEmployees(Event::HistoryLabelLoaded(request, name)),
    )
}

pub(super) fn apply_history_label(
    app: &mut App,
    request: HistoryLabelRequest,
    appearance: Option<EmployeeAppearance>,
) {
    if app.digital_employees.history_label_request.as_ref() != Some(&request)
        || app.session_state.session_view_cache.navigation_generation != request.navigation_generation
        || app.session_state.active_session_id.as_deref() != Some(request.session_id.as_str())
        || credential(app, &crate::app::config::gateway_client_endpoint()).is_none()
    {
        return;
    }
    if let Some(appearance) = appearance
        && let Some(runtime) = app.session_state.session_runtime_states.get_mut(&request.session_id)
        && runtime.is_employee_chat()
    {
        runtime.employee_display_name = Some(appearance.name);
        runtime.employee_portrait = appearance.portrait;
    }
}

impl Selection {
    pub(super) fn private(employee_id: String, name: String) -> Self {
        let portrait = super::portraits::avatar(&name, false);
        Self { target: EmployeeTarget { employee_id, kind: EmployeeKind::Private }, name, portrait }
    }

    pub(super) fn shared(employee: &super::shared_api::SharedEmployee) -> Self {
        let portrait = super::portraits::shared_avatar(
            &employee.name, employee.manager(), employee.role().unwrap_or_default(),
        );
        Self {
            target: EmployeeTarget { employee_id: employee.employee_id.clone(), kind: EmployeeKind::Shared },
            name: employee.name.clone(),
            portrait,
        }
    }
}

#[derive(Clone)]
pub(crate) struct Scope {
    address: String,
    tenant: vw_api_transport::TenantScope,
    user: vw_api_transport::UserId,
}

impl Scope {
    fn matches(&self, endpoint: &GatewayEndpoint) -> bool {
        self.address == endpoint.base_url()
            && &self.tenant == endpoint.tenant_scope()
            && &self.user == endpoint.user_id()
    }
}

pub(crate) fn credential(app: &App, endpoint: &GatewayEndpoint) -> Option<ShijuCredential> {
    app.digital_employees.chat_scope.as_ref().filter(|scope| scope.matches(endpoint))?;
    app.account.active_account()?.skill_hub_credential().gateway_credential().ok()
}

pub(super) fn select(app: &mut App, page: Page, selection: Option<Selection>) -> Task<Message> {
    if app.digital_employees.page != page
        || app.navigation_tabs.screen != crate::app::Screen::DigitalEmployees
    {
        return Task::none();
    }
    let Some(selection) = selection else {
        return app.show_localized_toast(crate::app::state::ToastKind::Error, "该数字员工当前不可聊天，请刷新列表", vec![]);
    };
    check_scope(app, Some(ChatIntent::Open(selection)))
}

pub(crate) fn check_scope(app: &mut App, selection: Option<ChatIntent>) -> Task<Message> {
    let endpoint = crate::app::config::gateway_client_endpoint();
    let Some(account) = app.account.active_account() else {
        return if selection.is_some() {
            app.show_localized_toast(crate::app::state::ToastKind::Error, "请先登录识矩", vec![])
        } else {
            Task::none()
        };
    };
    if !endpoint.is_loopback() {
        return if selection.is_some() {
            app.show_localized_toast(crate::app::state::ToastKind::Error, "请连接本机网关", vec![])
        } else {
            Task::none()
        };
    }
    let Ok(credential) = account.skill_hub_credential().gateway_credential() else {
        return Task::none();
    };
    app.digital_employees.chat_scope_request =
        app.digital_employees.chat_scope_request.wrapping_add(1);
    let request = app.digital_employees.chat_scope_request;
    Task::perform(
        async move {
            let transport = gateway_http::GatewayTransport::new(endpoint.clone())?;
            let verification_failed = |_| "无法验证识矩账号，请重新登录后重试";
            let mut header = reqwest::header::HeaderValue::from_str(credential.expose())
                .map_err(|_| "识矩登录凭据无效")?;
            header.set_sensitive(true);
            let request = transport
                .native_transport()?
                .get(format!("{}/v1/shiju/account/me", endpoint.base_url()))
                .header(SHIJU_CREDENTIAL_HEADER, header);
            let account: ShijuAccountMeDto =
                gateway_http::apply_request_context(request, &endpoint)
                    .send()
                    .await
                    .map_err(verification_failed)?
                    .error_for_status()
                    .map_err(verification_failed)?
                    .json()
                    .await
                    .map_err(verification_failed)?;
            if account.status != "enabled"
                || account.user_id.trim().is_empty()
                || account.company_id.trim().is_empty()
            {
                return Err("当前识矩账号不可用，请重新登录".into());
            }
            Ok(Scope {
                address: endpoint.base_url(),
                tenant: endpoint.tenant_scope().clone(),
                user: endpoint.user_id().clone(),
            })
        },
        move |result| {
            Message::DigitalEmployees(Event::ChatScopeChecked(request, result, selection))
        },
    )
}

pub(super) fn scope_checked(
    app: &mut App,
    request: u64,
    result: Result<Scope, desktop_i18n::workflow::Feedback>,
    selection: Option<ChatIntent>,
) -> Task<Message> {
    if request != app.digital_employees.chat_scope_request {
        return Task::none();
    }
    if matches!(selection, Some(ChatIntent::Bind(_)))
        && !super::chat_picker::finish_verification(app, request)
    {
        return Task::none();
    }
    match result {
        Ok(scope) if scope.matches(&crate::app::config::gateway_client_endpoint()) => {
            app.digital_employees.chat_scope = Some(scope);
        }
        Ok(_) => return Task::none(),
        Err(error) => {
            return if selection.is_some() { app.show_message_toast(crate::app::state::ToastKind::Error, error.message()) } else { Task::none() };
        }
    }
    let selection = match selection {
        Some(ChatIntent::Bind(selection)) => {
            app.digital_employees.chat_picker = None;
            // A project resolution for a previous first send must not use a new target.
            app.session_state.agent_stream_id = app.session_state.agent_stream_id.wrapping_add(1);
            bind_selection(app, selection);
            return Task::none();
        }
        Some(ChatIntent::Open(selection)) => selection,
        None => return Task::none(),
    };
    let page = match selection.target.kind {
        EmployeeKind::Private => Page::Assistants,
        EmployeeKind::Shared => Page::Shared,
    };
    if app.digital_employees.page != page
        || app.navigation_tabs.screen != crate::app::Screen::DigitalEmployees
    {
        return Task::none();
    }
    open_draft(app, selection)
}

fn open_draft(app: &mut App, selection: Selection) -> Task<Message> {
    let Ok(id) = ordered_id::descending(ordered_id::Prefix::Session, None) else {
        return app.show_localized_toast(crate::app::state::ToastKind::Error, "无法生成数字员工会话 ID，请重试", vec![]);
    };
    let path = app
        .navigation_tabs
        .project_path
        .clone()
        .or_else(|| app.workspace_state.recent_projects.first().cloned());
    let path = match path {
        Some(path) => path,
        None => {
            let Some(dirs) = crate::app::project_dirs() else {
                return app.show_localized_toast(crate::app::state::ToastKind::Error, "无法确定聊天目录，请先打开一个项目", vec![]);
            };
            let path = dirs.data_dir().join("employee-chat");
            if std::fs::create_dir_all(&path).is_err() {
                return app.show_localized_toast(crate::app::state::ToastKind::Error, "无法创建聊天目录，请先打开一个项目", vec![]);
            }
            path.to_string_lossy().into_owned()
        }
    };
    let project = crate::app::message::project::open_employee_session_draft(app, path, id);
    bind_selection(app, selection);
    project
}

fn bind_selection(app: &mut App, selection: Selection) {
    let runtime = app.current_session_runtime_mut();
    runtime.agent = Some(EMPLOYEE_CHAT_PROFILE.into());
    runtime.agent_label = Some(selection.name.clone());
    runtime.employee_display_name = Some(selection.name);
    runtime.employee_portrait = selection.portrait;
    runtime.acp_agent = Some("employee-acp".into());
    runtime.employee_target = Some(selection.target);
    runtime.acp_options_state = Default::default();
    runtime.acp_model_source = vw_api_acp_model::acp_model_source::AcpModelSource::CliLocal;
    runtime.full_access_enabled = false;
    runtime.full_access_initialized = false;
    runtime.full_access_overridden = false;
    runtime.task_mode_enabled = false;
    runtime.workflow_mode_enabled = false;
    runtime.goal_mode_enabled = false;
    app.invalidate_project_chat_preferences_read();
    app.session_state.acp_agent = Some("employee-acp".into());
    app.popover_visibility.reasoning_effort = false;
    app.sync_active_session_preferences();
}

pub(crate) fn invalidate_chats(
    app: &mut App,
    outgoing: Option<desktop_account::SecretSkey>,
) -> Task<Message> {
    use crate::app::message::chat::stream::stop_agent_stream;
    app.digital_employees.chat_picker = None;
    let ids: Vec<_> = app
        .session_state
        .session_runtime_states
        .iter()
        .filter(|(_, runtime)| runtime.is_employee_chat())
        .map(|(id, _)| id.clone())
        .collect();
    let mut stops = Vec::new();
    let endpoint = crate::app::config::gateway_client_endpoint();
    if app.digital_employees.chat_scope.take().is_some_and(|scope| scope.matches(&endpoint))
        && let Some(credential) = outgoing.and_then(|key| key.gateway_credential().ok())
    {
        stops.push(Task::perform(
            async move {
                let client = vw_gateway_client::GatewayClient::new(endpoint)?;
                GatewayShijuAccountClient::new(client.transport().clone())
                    .shiju_employee_account_revoke(&credential)
                    .await
            },
            |result| {
                if result.is_err() {
                    tracing::warn!("failed to invalidate outgoing employee gateway credential");
                }
                Message::None
            },
        ));
    }
    for id in &ids {
        stops.push(stop_agent_stream(app, id));
        app.session_state.session_runtime_states.remove(id);
        app.session_state.session_chat_cache.remove(id);
        app.session_state.session_chat_message_id_cache.remove(id);
        app.session_state.session_previews.remove(id);
        app.session_state.session_view_cache.remove(id);
    }
    app.session_state.agent_stream_id = app.session_state.agent_stream_id.wrapping_add(1);
    app.digital_employees.chat_request_floor = app.session_state.agent_stream_id;
    if app.session_state.active_session_id.as_ref().is_some_and(|id| ids.contains(id)) {
        app.session_state.active_session_id = None;
        app.chat_view.chat.clear();
        app.chat_view.chat_message_ids.clear();
        app.session_state.paged_chat_timeline = None;
        app.close_session_transient_ui();
        app.invalidate_chat_ui_state();
    } else if app.session_state.active_session_id.is_none()
        && ids.iter().any(|id| id == crate::app::session::EMPTY_SESSION_ID)
    {
        app.session_state.input_editor = iced::widget::text_editor::Content::new();
    }
    Task::batch(stops)
}

pub(crate) fn request_is_retired(app: &App, id: u64) -> bool {
    id < app.digital_employees.chat_request_floor && app.find_session_by_request_id(id).is_none()
}

#[cfg(test)]
#[path = "chat_tests.rs"]
mod tests;
