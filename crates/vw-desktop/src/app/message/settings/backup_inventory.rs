//! Settings entry for gateway-owned configuration backups; no local file reads.
use crate::app::{App, Message, state::AgentAppsContext};
use super::SettingsMessage;
use iced::{Element, Task};
use vw_gateway_client::vw_api_types::{ProjectScope, agent_apps::data_management::DataBackupSummary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Identity {
    context: AgentAppsContext,
    project: String,
    host: String,
    catalog: u64,
}
#[derive(Debug, Clone)]
pub(crate) enum Event {
    Refresh,
    Loaded(Identity, u64, Result<Vec<DataBackupSummary>, Option<u16>>),
}
#[derive(Clone, Default)]
pub(crate) struct State {
    identity: Option<Identity>,
    generation: u64,
    loading: bool,
    rows: Option<Vec<DataBackupSummary>>,
    error: bool,
}
fn identity(app: &App) -> Option<Identity> {
    let client = crate::app::config::gateway_client().ok()?;
    let context = AgentAppsContext { gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    if app.apps_catalog.context.as_ref() != Some(&context) || !app.apps_catalog.connection_open
        || app.apps_catalog.host.trim().is_empty() || app.apps_catalog.token.is_empty() { return None; }
    Some(Identity { context, project: app.project_id.clone().filter(|p| !p.trim().is_empty())?,
        host: app.apps_catalog.host.clone(), catalog: app.apps_catalog.generation })
}
fn message(event: Event) -> Message { Message::Settings(SettingsMessage::BackupInventory(event)) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Some(current) = identity(app) else {
        app.apps_catalog.backups = State::default();
        return if matches!(event, Event::Refresh) {
            app.show_error_toast("请先连接网关并选择项目与执行主机")
        } else { Task::none() };
    };
    let state = &mut app.apps_catalog.backups;
    match event {
        Event::Refresh => {
            if state.identity.as_ref() == Some(&current) && state.loading { return Task::none(); }
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            state.identity = Some(current.clone());
            state.loading = true;
            state.rows = None;
            state.error = false;
            let Ok(client) = crate::app::config::gateway_client() else { state.loading = false; return Task::none(); };
            let Ok(project) = ProjectScope::new(current.context.tenant.clone(), current.project.clone().into()) else {
                state.loading = false;
                state.error = true;
                return Task::none();
            };
            Task::perform(async move {
                let result = client.data_backup_inventory(&project, &current.host).await.map_err(|error| {
                    match error { vw_gateway_client::AgentAppsReadError::Http { status, .. } => Some(status), _ => None }
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
                    return app.show_error_toast(match status {
                        Some(401 | 403) => "没有读取配置备份的权限",
                        Some(501) => "执行主机暂不支持配置备份读取",
                        _ => "配置备份读取失败，请重试",
                    });
                }
            }
            Task::none()
        }
    }
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    use iced::widget::{button, column, text};
    use crate::app::components::system_settings_common::{rounded_action_btn_style, settings_muted_text_style};
    let current = identity(app);
    let state = &app.apps_catalog.backups;
    let visible = current.is_some() && current.as_ref() == state.identity.as_ref();
    let loading = visible && state.loading;
    let mut content = column![
        text("配置备份").size(16),
        text("读取当前项目在执行主机上的备份元数据；恢复需另行校验各领域能力。")
            .size(13).style(settings_muted_text_style),
        button(text(if loading { "读取中…" } else { "读取配置备份" }))
            .padding([8, 14]).style(rounded_action_btn_style)
            .on_press_maybe((current.is_some() && !loading).then_some(message(Event::Refresh))),
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
        content = content.push(text("请先连接网关并选择项目与执行主机").size(13).style(settings_muted_text_style));
    }
    content.into()
}
