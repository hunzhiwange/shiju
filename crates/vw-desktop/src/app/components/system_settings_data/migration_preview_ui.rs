//! Read-only migration preview. Paths and local bytes never enter the handle API.
use crate::app::{App, Message, message::SettingsMessage};
use crate::app::components::system_settings_common::*;
use iced::{Element, Task};
use iced::widget::{button, column, text};
use vw_gateway_client::vw_api_types::{ProjectScope, agent_apps::data_management::*};

#[derive(Clone, Default)]
pub(crate) struct State {
    open: bool,
    source: String,
    principal: String,
    generation: u64,
    loading: bool,
    result: Option<DataImportPreview>,
    error: bool,
    result_context: Option<(String, u64)>,
}
#[derive(Debug, Clone)]
pub(crate) enum Event {
    Open, Close, Source(String), Principal(String), Preview,
    Loaded(u64, DataManagementTarget, String, u64, Result<DataImportPreview, ()>),
}
fn message(event: Event) -> Message { Message::Settings(SettingsMessage::MigrationPreview(event)) }
fn target(app: &App) -> Option<DataManagementTarget> {
    let client = crate::app::config::gateway_client().ok()?;
    let context = crate::app::state::AgentAppsContext { gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    if !app.apps_catalog.connection_open || app.apps_catalog.context.as_ref() != Some(&context)
        || app.apps_catalog.host.is_empty() || app.apps_catalog.token.is_empty() { return None; }
    Some(DataManagementTarget {
        scope: ProjectScope::new(context.tenant, app.project_id.clone()?.into()).ok()?,
        target_host_id: app.apps_catalog.host.clone(),
        principal_id: app.apps_catalog.migration_preview.principal.trim().to_owned(),
    })
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Preview = event {
        let Some(target) = target(app).filter(|t| !t.principal_id.is_empty()) else {
            return app.show_error_toast("请连接网关、选择项目与主机并填写用户身份");
        };
        let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
        let gateway = client.endpoint().base_url();
        let catalog = app.apps_catalog.generation;
        let token = app.apps_catalog.token.clone();
        let state = &mut app.apps_catalog.migration_preview;
        if state.loading || state.source.trim().is_empty() { return Task::none(); }
        state.generation = state.generation.wrapping_add(1);
        let generation = state.generation;
        state.loading = true;
        state.result = None;
        state.error = false;
        let transfer_id = DataTransferId(state.source.trim().to_owned());
        return Task::perform(async move {
            let result = async {
                let source = client.data_migration_identify(&IdentifyDataSourceRequest {
                    target: target.clone(), transfer_id,
                }, Some(&token)).await.map_err(|_| ())?;
                let preview = client.data_migration_preview(&PreviewDataImportRequest {
                    target: target.clone(), transfer_id: source.transfer_id.clone(),
                    // Current API explicitly supports only the providers category.
                    selections: vec![DataSelection { category: DataCategory::Providers, entries: DataEntrySelection::All }],
                }, Some(&token)).await.map_err(|_| ())?;
                if preview.target != target || preview.source != source { return Err(()); }
                Ok(preview)
            }.await;
            message(Event::Loaded(generation, target, gateway, catalog, result))
        }, |m| m);
    }
    let current = target(app);
    let gateway = crate::app::config::gateway_client().ok().map(|c| c.endpoint().base_url());
    let catalog = app.apps_catalog.generation;
    let state = &mut app.apps_catalog.migration_preview;
    match event {
        Event::Open => { state.open = true; state.result = None; }
        Event::Close => { state.open = false; state.loading = false; state.generation = state.generation.wrapping_add(1); }
        Event::Source(value) => { state.source = value; state.result = None; state.loading = false; state.generation = state.generation.wrapping_add(1); }
        Event::Principal(value) => { state.principal = value; state.result = None; state.loading = false; state.generation = state.generation.wrapping_add(1); }
        Event::Loaded(generation, requested, requested_gateway, requested_catalog, result) => {
            if !state.open || !state.loading || generation != state.generation { return Task::none(); }
            state.loading = false;
            if current.as_ref() != Some(&requested) || gateway.as_ref() != Some(&requested_gateway) || catalog != requested_catalog { return Task::none(); }
            match result {
                Ok(preview) => { state.result = Some(preview); state.result_context = Some((requested_gateway, requested_catalog)); },
                Err(()) => { state.error = true; return app.show_error_toast("来源读取或预览失败，请检查授权、版本与来源有效期"); }
            }
        }
        Event::Preview => unreachable!(),
    }
    Task::none()
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let base = button(text("选择迁入来源与预览")).padding([8, 14])
        .style(rounded_action_btn_style).on_press(message(Event::Open));
    let state = &app.apps_catalog.migration_preview;
    if !state.open { return base.into(); }
    let mut body = column![
        text("目标主机已授权来源；本机文件上传尚未提供。请勿填写文件路径。"),
        text("用户身份（由服务端重新鉴权）"),
        settings_text_input("用户身份", &state.principal).on_input(|v| message(Event::Principal(v))),
        text("已授权来源句柄"),
        settings_text_input("来源句柄", &state.source).on_input(|v| message(Event::Source(v))),
        text("当前可预览供应商类别；凭据、绝对路径与系统集成仅显示排除计数。"),
    ].spacing(10);
    if state.error { body = body.push(text("来源读取或预览失败；未提交迁入。")); }
    if let Some(preview) = &state.result {
        if target(app).as_ref() == Some(&preview.target)
            && state.result_context == crate::app::config::gateway_client().ok().map(|c| (c.endpoint().base_url(), app.apps_catalog.generation)) {
            // Never display arbitrary source metadata or opaque IDs as labels.
            let version = &preview.source.application_version;
            let version = if version.len() <= 32 && version.bytes().all(|b| b.is_ascii_digit() || b == b'.') { version.as_str() } else { "未知" };
            body = body.push(text(format!("来源 {:?} · 版本 {} · schema {} · 格式 {}", preview.source.kind, version, preview.source.schema_version, preview.source.format_version)));
            if preview.categories.is_empty() { body = body.push(text("没有可迁入类别")); }
            for category in &preview.categories {
                body = body.push(text(format!("{:?}：已选 {}，可迁入 {}，排除 {}", category.category, category.selected, category.eligible, category.excluded)));
            }
            for exclusion in &preview.exclusions {
                body = body.push(text(format!("{:?} · {:?}：{}", exclusion.category, exclusion.reason, exclusion.count)));
            }
        }
    }
    let footer = button(text(if state.loading { "预览中…" } else { "读取脱敏预览" }))
        .padding([8, 14]).style(primary_action_btn_style)
        .on_press_maybe((!state.loading).then_some(message(Event::Preview)));
    let card = settings_modal_layout(
        settings_modal_header(text("迁入来源预览"), settings_close_button(message(Event::Close))),
        body, Some(footer.into()),
    ).width(640).height(520);
    settings_modal_overlay(Some(base.into()), message(Event::Close), card)
}
