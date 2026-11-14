//! Native Codex settings and stored bytes are accessed only through the SDK.
use super::{Scope, message};
use crate::app::{App, Message, state::AgentAppsContext, components::system_settings_common::*};
use iced::{Element, Length, Task, widget::{button, checkbox, column, container, row, text}};
use vw_gateway_client::{AgentAppsReadError, vw_api_types::external_session::*};

#[derive(Clone, Default)]
pub(super) struct State {
    target: Option<HistoryMaintenanceTarget>,
    value: Option<SessionCompressionState>,
    draft: bool,
    pending: bool,
    saving: bool,
    request: u64,
    error: bool,
}
impl State {
    pub(super) fn is_open(&self) -> bool { self.target.is_some() }
}
#[derive(Clone, Debug)]
pub enum Event {
    Open(HistoryMaintenanceTarget), Close, Reload, Toggle(bool), Save, Cancel,
    Loaded(Scope, HistoryMaintenanceTarget, u64, Option<bool>, Result<SessionCompressionState, AgentAppsReadError>),
}
fn msg(event: Event) -> Message { message(super::Event::Compression(event)) }

pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else {
        return app.show_error_toast("网关不可用，请重新连接");
    };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let Some(project) = app.project_id.clone() else { return Task::none(); };
    let state = &mut app.apps_catalog.external_sessions;
    if !state.open || app.apps_catalog.context.as_ref() != Some(&context)
        || state.host != app.apps_catalog.host || app.apps_catalog.token.is_empty()
        || state.maintenance.is_open() { return Task::none(); }
    let scope = Scope { context, host: state.host.clone(), project,
        catalog: app.apps_catalog.generation, generation: state.generation, append: false };
    let current = |other: &Scope, target: &HistoryMaintenanceTarget| {
        other.context == scope.context && other.host == scope.host && other.project == scope.project
            && other.catalog == scope.catalog && other.generation == scope.generation
            && target.target_host == scope.host && target.scope.project_id().as_ref() == scope.project.as_str()
    };
    let compression = &mut state.compression;
    match event {
        Event::Open(target) => {
            if compression.is_open() || state.busy || state.deletion.busy() || state.resume.busy
                || !current(&scope, &target) || !state.page.as_ref().is_some_and(|page|
                    page.sessions.iter().any(|s| s.identity.provider == SessionProvider::Codex
                        && s.identity.instance_id == target.instance_id && s.identity.scope == target.scope
                        && s.identity.target_host == target.target_host)) { return Task::none(); }
            let request = compression.request.wrapping_add(1);
            *compression = State { target: Some(target), request, ..State::default() };
            return Task::done(msg(Event::Reload));
        }
        Event::Loaded(other, target, request, expected, result) => {
            if !current(&other, &target) || compression.target.as_ref() != Some(&target)
                || compression.request != request || !compression.pending { return Task::none(); }
            compression.pending = false;
            compression.saving = false;
            match result {
                Ok(value) if expected.is_none_or(|enabled| enabled == value.enabled) => {
                    compression.draft = value.enabled;
                    compression.value = Some(value);
                    compression.error = false;
                    if expected.is_some() { return app.show_success_toast("压缩设置已保存并重新读取确认"); }
                }
                _ => {
                    compression.value = None;
                    compression.error = true;
                    return app.show_error_toast("压缩设置或磁盘统计未确认，请重新读取；保存可能已生效");
                }
            }
        }
        Event::Close | Event::Cancel => {
            // Writes cannot be undone by dismissing their response. Wait for read-back.
            if compression.saving { return Task::none(); }
            let request = compression.request.wrapping_add(1);
            *compression = State { request, ..State::default() };
        }
        _ if !compression.is_open() || compression.pending => return Task::none(),
        Event::Toggle(value) => {
            if compression.value.is_some() { compression.draft = value; }
        }
        Event::Reload | Event::Save => {
            let Some(target) = compression.target.clone() else { return Task::none(); };
            let expected = if matches!(event, Event::Save) {
                if compression.value.is_none() { return Task::none(); }
                Some(compression.draft)
            } else { None };
            compression.request = compression.request.wrapping_add(1);
            let request = compression.request;
            compression.pending = true;
            compression.saving = expected.is_some();
            compression.error = false;
            compression.value = None;
            let token = app.apps_catalog.token.clone();
            return Task::perform(async move {
                let result = async {
                    if let Some(enabled) = expected {
                        client.external_session_compression_set(&SessionCompressionUpdate {
                            target: target.clone(), enabled }, &token).await?;
                    }
                    client.external_session_compression(&target, &token).await
                }.await;
                msg(Event::Loaded(scope, target, request, expected, result))
            }, |m| m);
        }
    }
    Task::none()
}

pub(super) fn view<'a>(state: &'a super::State, base: Element<'a, Message>) -> Element<'a, Message> {
    let compression = &state.compression;
    let Some(target) = &compression.target else { return base; };
    let mut body = column![text(format!("主机：{} · Codex 实例：{}", target.target_host, target.instance_id)),
        text("此开关由 Codex 在启动后处理历史压缩；桌面只保存原生配置并查询磁盘占用。")
            .style(settings_muted_text_style)].spacing(12);
    if let Some(value) = &compression.value {
        let mut control = checkbox(compression.draft).label("启用 Codex 会话压缩").style(settings_checkbox_style);
        if !compression.pending { control = control.on_toggle(|v| msg(Event::Toggle(v))); }
        body = body.push(control)
            .push(text(format!("当前已确认：{}", if value.enabled { "已开启" } else { "已关闭" })))
            .push(text(format!("活动会话：{} 字节", value.active_bytes)))
            .push(text(format!("归档会话：{} 字节", value.archived_bytes)))
            .push(text("统计为实际存储文件字节数，压缩文件按压缩后的大小计算。"));
    }
    if compression.pending {
        body = body.push(text(if compression.saving { "正在保存并重新读取确认，请稍候…" } else { "正在读取压缩状态和磁盘统计…" }));
    }
    if compression.error { body = body.push(settings_error_banner("读取或保存结果未确认，请刷新后再操作。")); }
    let footer = row![
        button(text("刷新")).padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe((!compression.pending).then(|| msg(Event::Reload))),
        button(text("保存设置")).padding([8, 12]).style(primary_action_btn_style)
            .on_press_maybe((!compression.pending && compression.value.as_ref().is_some_and(|v| v.enabled != compression.draft))
                .then(|| msg(Event::Save))),
        button(text("取消 / 关闭")).padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe((!compression.saving).then(|| msg(Event::Cancel)))].spacing(8);
    let header = row![text("压缩设置与磁盘统计").size(18), settings_close_button(msg(Event::Close))].spacing(12);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(Length::Fixed(620.0)).height(Length::Fixed(420.0));
    settings_modal_overlay(Some(base), msg(Event::Close), container(card).padding(12).center(Length::Fill))
}
