use super::{Event, Page};
use crate::app::{App, Message};
use desktop_session::session::agent::{EMPLOYEE_CHAT_PROFILE, EmployeeKind, EmployeeTarget};
use gateway_endpoint::GatewayEndpoint;
use gateway_shiju_account::{
    GatewayShijuAccountClient, SHIJU_CREDENTIAL_HEADER, ShijuAccountMeDto, ShijuCredential,
};
use iced::Task;

#[derive(Clone)]
pub(crate) struct Selection {
    pub target: EmployeeTarget,
    pub name: String,
}

impl Selection {
    pub(super) fn private(employee_id: String, name: String) -> Self {
        Self { target: EmployeeTarget { employee_id, kind: EmployeeKind::Private }, name }
    }

    pub(super) fn shared(employee_id: String, name: String) -> Self {
        Self { target: EmployeeTarget { employee_id, kind: EmployeeKind::Shared }, name }
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
        return app.show_error_toast("该数字员工当前不可聊天，请刷新列表");
    };
    check_scope(app, Some(selection))
}

pub(crate) fn check_scope(app: &mut App, selection: Option<Selection>) -> Task<Message> {
    let endpoint = crate::app::config::gateway_client_endpoint();
    let Some(account) = app.account.active_account() else {
        return if selection.is_some() {
            app.show_error_toast("请先登录识矩")
        } else {
            Task::none()
        };
    };
    if !endpoint.is_loopback() || endpoint.user_id().as_ref() == vw_api_transport::DEFAULT_USER_ID {
        return if selection.is_some() {
            app.show_error_toast("请连接与当前识矩账号匹配的本机网关")
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
            if account.user_id != endpoint.user_id().as_ref()
                || account.company_id != endpoint.tenant_scope().company_id().as_ref()
                || account.status != "enabled"
            {
                return Err("本机网关与当前识矩账号不匹配".into());
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
    result: Result<Scope, String>,
    selection: Option<Selection>,
) -> Task<Message> {
    if request != app.digital_employees.chat_scope_request {
        return Task::none();
    }
    match result {
        Ok(scope) if scope.matches(&crate::app::config::gateway_client_endpoint()) => {
            app.digital_employees.chat_scope = Some(scope);
        }
        Ok(_) => return Task::none(),
        Err(error) => {
            return if selection.is_some() { app.show_error_toast(error) } else { Task::none() };
        }
    }
    let Some(selection) = selection else { return Task::none() };
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
    let blank = app.session_state.session_runtime_states.get(crate::app::session::EMPTY_SESSION_ID);
    if app.workspace_state.new_session_creating
        || blank.is_some_and(|runtime| !runtime.input_editor.text().trim().is_empty())
        || !app.session_state.files.is_empty()
    {
        return app.show_warning_toast("请先处理已有的会话草稿或附件");
    }
    let path = app
        .navigation_tabs
        .project_path
        .clone()
        .or_else(|| app.workspace_state.recent_projects.first().cloned());
    let path = match path {
        Some(path) => path,
        None => {
            let Some(dirs) = crate::app::project_dirs() else {
                return app.show_error_toast("无法确定聊天目录，请先打开一个项目");
            };
            let path = dirs.data_dir().join("employee-chat");
            if std::fs::create_dir_all(&path).is_err() {
                return app.show_error_toast("无法创建聊天目录，请先打开一个项目");
            }
            path.to_string_lossy().into_owned()
        }
    };
    let project = app.open_project(path.clone());
    let draft = crate::app::message::project::open_session_draft(app, path.clone(), path);
    let runtime = app.current_session_runtime_mut();
    runtime.agent = Some(EMPLOYEE_CHAT_PROFILE.into());
    runtime.agent_label = Some(selection.name);
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
    app.sync_active_session_preferences();
    Task::batch([project, draft])
}

pub(crate) fn invalidate_chats(
    app: &mut App,
    outgoing: Option<crate::app::account::SecretSkey>,
) -> Task<Message> {
    use crate::app::message::chat::stream::stop_agent_stream;
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
