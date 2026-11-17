//! Settings entry for gateway-owned configuration backups; no local file reads.
use vw_gateway_client::GatewayClient;
use iced::{Element, Task};
use {
    vw_api_transport::ProjectScope,
    vw_api_app_operations::DataBackupSummary,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub gateway: String,
    pub tenant: vw_api_transport::TenantScope,
    pub project_path: Option<String>,
    pub project: String,
    pub host: String,
    pub catalog: u64,
}
#[derive(Debug, Clone)]
pub enum Event {
    Refresh,
    NotifyError(&'static str),
    Loaded(Identity, u64, Result<Vec<DataBackupSummary>, Option<u16>>),
}
#[derive(Clone, Default)]
pub struct State {
    identity: Option<Identity>,
    generation: u64,
    loading: bool,
    rows: Option<Vec<DataBackupSummary>>,
    error: bool,
}
fn message(event: Event) -> Event { event }
pub fn update(state: &mut State, current: Option<Identity>, client: Option<GatewayClient>, event: Event) -> Task<Event> {
    let Some(current) = current else {
        *state = State::default();
        return if matches!(event, Event::Refresh) {
            Task::done(Event::NotifyError("网关不可用，请检查连接"))
        } else { Task::none() };
    };
    match event {
        Event::NotifyError(_) => Task::none(),
        Event::Refresh => {
            if state.identity.as_ref() == Some(&current) && state.loading { return Task::none(); }
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            state.identity = Some(current.clone());
            state.loading = true;
            state.rows = None;
            state.error = false;
            let Some(client) = client else { state.loading = false; return Task::none(); };
            let Ok(project) = ProjectScope::new(current.tenant.clone(), current.project.clone().into()) else {
                state.loading = false;
                state.error = true;
                return Task::none();
            };
            Task::perform(async move {
                let result = gateway_app_data::GatewayAppDataClient::new(client.transport().clone()).data_backup_inventory(&project, &current.host).await.map_err(|error| {
                    match error { gateway_http::AgentAppsReadError::Http { status, .. } => Some(status), _ => None }
                });
                message(Event::Loaded(current, generation, result))
            }, |message| message)
        }
        Event::Loaded(request, generation, result) => {
            if request != current || state.identity.as_ref() != Some(&current)
                || state.generation != generation || !state.loading { return Task::none(); }
            state.loading = false;
            match result {
                Ok(rows) => state.rows = Some(rows),
                Err(status) => {
                    state.error = true;
                    return Task::done(Event::NotifyError(match status {
                        Some(401 | 403) => "没有读取配置备份的权限",
                        Some(501) => "执行主机暂不支持配置备份读取",
                        _ => "配置备份读取失败，请重试",
                    }));
                }
            }
            Task::none()
        }
    }
}

pub fn view<'a>(state: &'a State, current: Option<Identity>) -> Element<'a, Event> {
    use iced::widget::{button, column, text};
    use desktop_ui_controls::system_settings_common::{rounded_action_btn_style, settings_muted_text_style};
    let visible = current.is_some() && current.as_ref() == state.identity.as_ref();
    let loading = visible && state.loading;
    let mut content = column![
        text("配置备份").size(16),
        text("读取当前项目在执行主机上的备份元数据；恢复需另行校验各领域能力。")
            .size(13).style(settings_muted_text_style),
        button(text(if loading { "读取中…" } else { "读取配置备份" }))
            .padding([8, 14]).style(rounded_action_btn_style)
            .on_press_maybe((current.is_some() && !loading).then_some(Event::Refresh)),
    ].spacing(8);
    if visible {
        if state.error { content = content.push(text("读取失败，可重试").size(13)); }
        if let Some(rows) = &state.rows {
            if rows.is_empty() { content = content.push(text("当前范围内没有配置备份").size(13)); }
            for entry in rows {
                content = content.push(text(format!("{} · {} 字节 · 创建时间 {} ms", entry.name,
                    entry.size_bytes, entry.created_at_ms)).size(13));
            }
        }
    } else if current.is_none() {
        content = content.push(text("网关不可用，请检查连接").size(13).style(settings_muted_text_style));
    }
    content.into()
}
