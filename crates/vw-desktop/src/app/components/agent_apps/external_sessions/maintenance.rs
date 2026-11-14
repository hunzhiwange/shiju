//! Host-owned history settings and maintenance; no history files are read by desktop.
use super::{message, Scope};
use crate::app::{App, Message, state::AgentAppsContext, components::system_settings_common::*};
use iced::{Element, Length, Task, widget::{button, checkbox, column, container, row, text}};
use vw_gateway_client::{AgentAppsReadError, vw_api_types::{external_session::*, task::TaskStatus}};

#[derive(Clone, Default)]
pub(super) struct State {
    target: Option<HistoryMaintenanceTarget>,
    options: Option<HistoryOptions>,
    saved_options: Option<HistoryOptions>,
    backups: Vec<HistoryBackupSummary>,
    progress: Option<HistoryMaintenanceProgress>,
    confirmation: Option<String>,
    pending: bool,
    uncertain: bool,
}
impl State {
    pub(super) fn is_open(&self) -> bool { self.target.is_some() }
    pub(super) fn busy(&self) -> bool {
        self.pending || self.uncertain || self.progress.as_ref().is_some_and(|p|
            matches!(p.status, TaskStatus::Queued | TaskStatus::Running))
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Open(HistoryMaintenanceTarget), Close, Reload,
    Unify(bool), Migrate(bool), Save, Start(HistoryMaintenanceAction),
    AskRestore(String), ConfirmRestore, DismissRestore, Query, Cancel,
    Loaded(Scope, HistoryMaintenanceTarget, Result<(HistoryOptions, HistoryBackups), AgentAppsReadError>),
    Saved(Scope, HistoryMaintenanceTarget, Result<HistoryOptions, AgentAppsReadError>),
    Progress(Scope, HistoryMaintenanceTarget, Result<HistoryMaintenanceProgress, AgentAppsReadError>),
}
fn msg(event: Event) -> Message { message(super::Event::Maintenance(event)) }

pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let Some(project) = app.project_id.clone() else { return Task::none(); };
    let state = &mut app.apps_catalog.external_sessions;
    if !state.open || app.apps_catalog.context.as_ref() != Some(&context)
        || state.host != app.apps_catalog.host || app.apps_catalog.token.is_empty() { return Task::none(); }
    let scope = Scope { context, host: state.host.clone(), project,
        catalog: app.apps_catalog.generation, generation: state.generation, append: false };
    let current = |other: &Scope, target: &HistoryMaintenanceTarget| {
        other.context == scope.context && other.host == scope.host && other.project == scope.project
            && other.catalog == scope.catalog && other.generation == scope.generation
            && target.target_host == scope.host && target.scope.project_id().as_ref() == scope.project.as_str()
    };
    let token = app.apps_catalog.token.clone();
    let maintenance = &mut state.maintenance;
    match event {
        Event::Loaded(other, target, result) => {
            if !current(&other, &target) || maintenance.target.as_ref() != Some(&target) || !maintenance.pending { return Task::none(); }
            maintenance.pending = false;
            match result {
                Ok((options, backups)) => { maintenance.saved_options = Some(options.clone()); maintenance.options = Some(options); maintenance.backups = backups.generations; }
                Err(_) => return app.show_error_toast("历史设置或备份读取失败，请检查授权后重试"),
            }
        }
        Event::Saved(other, target, result) => {
            if !current(&other, &target) || maintenance.target.as_ref() != Some(&target) || !maintenance.pending { return Task::none(); }
            maintenance.pending = false;
            return match result {
                Ok(options) => { maintenance.saved_options = Some(options.clone()); maintenance.options = Some(options); app.show_success_toast("历史设置已保存；关闭开关不会自动恢复备份") }
                Err(_) => { maintenance.options = None; app.show_error_toast("保存结果未确认，请重新读取设置") }
            };
        }
        Event::Progress(other, target, result) => {
            if !current(&other, &target) || maintenance.target.as_ref() != Some(&target) || !maintenance.pending { return Task::none(); }
            maintenance.pending = false;
            match result {
                Ok(progress) => { maintenance.uncertain = false; maintenance.progress = Some(progress); }
                Err(_) => return app.show_error_toast("维护结果未确认；已有任务请查询进度，勿重复提交"),
            }
        }
        Event::Open(target) => {
            if maintenance.busy() || state.busy || state.deletion.busy() || state.resume.busy { return Task::none(); }
            // Only catalog-issued Codex identities may select the maintenance instance.
            if !current(&scope, &target) || !state.page.as_ref().is_some_and(|page| page.sessions.iter().any(|s|
                s.identity.provider == SessionProvider::Codex && s.identity.instance_id == target.instance_id
                    && s.identity.scope == target.scope && s.identity.target_host == target.target_host)) { return Task::none(); }
            *maintenance = State { target: Some(target), ..State::default() };
            return Task::done(msg(Event::Reload));
        }
        Event::Close => {
            if !maintenance.busy() { *maintenance = State::default(); }
        }
        Event::Query | Event::Cancel => {
            if maintenance.pending { return Task::none(); }
            let (Some(target), Some(progress)) = (maintenance.target.clone(), maintenance.progress.as_ref()) else { return Task::none(); };
            let cancel = matches!(event, Event::Cancel);
            if cancel && (!maintenance.busy() || progress.cancellation_requested) { return Task::none(); }
            let query = HistoryMaintenanceQuery { target: target.clone(), operation_id: progress.operation_id.clone() };
            maintenance.pending = true;
            return Task::perform(async move {
                let result = if cancel { client.external_history_maintenance_cancel(&query, &token).await }
                    else { client.external_history_maintenance_query(&query, &token).await };
                msg(Event::Progress(scope, target, result))
            }, |m| m);
        }
        _ if maintenance.busy() => return Task::none(),
        Event::Reload => {
            let Some(target) = maintenance.target.clone() else { return Task::none(); };
            maintenance.pending = true;
            maintenance.options = None;
            maintenance.backups.clear();
            return Task::perform(async move {
                let result = async {
                    let options = client.external_history_options(&target, &token).await?;
                    let backups = client.external_history_backups(&target, &token).await?;
                    Ok((options, backups))
                }.await;
                msg(Event::Loaded(scope, target, result))
            }, |m| m);
        }
        Event::Unify(value) => { if let Some(options) = &mut maintenance.options { options.unify_enabled = value; } }
        Event::Migrate(value) => { if let Some(options) = &mut maintenance.options { options.migrate_existing = value; } }
        Event::Save => {
            let (Some(target), Some(options)) = (maintenance.target.clone(), maintenance.options.clone()) else { return Task::none(); };
            maintenance.pending = true;
            return Task::perform(async move {
                let result = client.external_history_options_set(&HistoryOptionsUpdate { target: target.clone(), options }, &token).await;
                msg(Event::Saved(scope, target, result))
            }, |m| m);
        }
        Event::AskRestore(id) => {
            if maintenance.backups.iter().any(|b| b.complete && b.generation_id == id) { maintenance.confirmation = Some(id); }
        }
        Event::DismissRestore => maintenance.confirmation = None,
        Event::Start(action) if action == HistoryMaintenanceAction::Restore => return Task::none(),
        Event::Start(_) | Event::ConfirmRestore => {
            let Some(target) = maintenance.target.clone() else { return Task::none(); };
            let (action, generation_id) = match event {
                Event::Start(action) => (action, None),
                _ => {
                    let Some(id) = maintenance.confirmation.take() else { return Task::none(); };
                    if !maintenance.backups.iter().any(|b| b.complete && b.generation_id == id) { return Task::none(); }
                    (HistoryMaintenanceAction::Restore, Some(id))
                }
            };
            if maintenance.options.is_none() || maintenance.options != maintenance.saved_options {
                return app.show_error_toast("请先保存历史设置，再启动维护");
            }
            maintenance.pending = true;
            // A transport failure may hide an accepted operation. Never enable blind resubmission.
            maintenance.uncertain = true;
            maintenance.progress = None;
            return Task::perform(async move {
                let result = client.external_history_maintenance_submit(&HistoryMaintenanceRequest {
                    target: target.clone(), action, generation_id }, &token).await;
                msg(Event::Progress(scope, target, result))
            }, |m| m);
        }
    }
    Task::none()
}

pub(super) fn view<'a>(state: &'a super::State, base: Element<'a, Message>) -> Element<'a, Message> {
    let maintenance = &state.maintenance;
    let Some(target) = &maintenance.target else { return base; };
    let enabled = !maintenance.busy();
    let mut body = column![text(format!("主机：{} · Codex 实例：{}", target.target_host, target.instance_id)),
        text("先保存设置，再启动维护。关闭统一历史不会自动回滚；迁移与恢复需要单独启动。").style(settings_muted_text_style)].spacing(12);
    if let Some(options) = &maintenance.options {
        let mut unify = checkbox(options.unify_enabled).label("统一 Codex 会话历史").style(settings_checkbox_style);
        let mut migrate = checkbox(options.migrate_existing).label("迁入既有会话").style(settings_checkbox_style);
        if enabled { unify = unify.on_toggle(|v| msg(Event::Unify(v))); migrate = migrate.on_toggle(|v| msg(Event::Migrate(v))); }
        body = body.push(unify).push(migrate);
        for (label, action) in [("执行统一历史迁移（含备份）", HistoryMaintenanceAction::Unify),
            ("迁移旧官方历史（含备份）", HistoryMaintenanceAction::LegacyOfficial)] {
            body = body.push(button(text(label)).padding([8, 12]).style(rounded_action_btn_style)
                .on_press_maybe((enabled && maintenance.options == maintenance.saved_options).then(|| msg(Event::Start(action)))));
        }
    }
    if let Some(progress) = &maintenance.progress {
        body = body.push(text(format!("{:?} · {} · {}/{} · 已改会话 {} · 状态行 {}",
            progress.status, progress.phase, progress.completed, progress.total, progress.changed_rollouts, progress.changed_state_rows)));
        if progress.interrupted { body = body.push(text("主机重启导致任务中断，请核实备份后操作")); }
        if progress.cancellation_requested { body = body.push(text("取消已受理，仍需查询确认任务停止")); }
        body = body.push(row![
            button(text("查询进度")).padding([8, 12]).style(rounded_action_btn_style)
                .on_press_maybe((!maintenance.pending).then(|| msg(Event::Query))),
            button(text("取消维护")).padding([8, 12]).style(rounded_action_btn_style)
                .on_press_maybe((maintenance.busy() && !maintenance.pending && !progress.cancellation_requested).then(|| msg(Event::Cancel)))].spacing(8));
    } else if maintenance.uncertain && !maintenance.pending {
        body = body.push(settings_error_banner("提交结果未确认，已禁用重复操作。请在主机核实维护记录后重新连接。"));
    }
    for backup in &maintenance.backups {
        body = body.push(button(text(format!("恢复备份 {}（{} 项）{}", backup.generation_id, backup.entries,
            if backup.complete { "" } else { " · 不完整" }))).padding([8, 12]).style(rounded_action_btn_style)
            .on_press_maybe((enabled && backup.complete).then(|| msg(Event::AskRestore(backup.generation_id.clone())))));
    }
    if maintenance.confirmation.is_some() {
        body = body.push(text("确认使用所选备份恢复历史？这会修改目标实例的历史数据。"))
            .push(row![button(text("确认恢复")).padding([8, 12]).style(primary_action_btn_style)
                .on_press_maybe(enabled.then(|| msg(Event::ConfirmRestore))),
                button(text("取消")).padding([8, 12]).style(rounded_action_btn_style)
                .on_press_maybe(enabled.then(|| msg(Event::DismissRestore)))].spacing(8));
    }
    let footer = row![button(text("重新读取")).padding([8, 12]).style(rounded_action_btn_style)
        .on_press_maybe(enabled.then(|| msg(Event::Reload))),
        button(text("保存设置")).padding([8, 12]).style(primary_action_btn_style)
        .on_press_maybe((enabled && maintenance.options.is_some()).then(|| msg(Event::Save))),
        button(text("关闭")).padding([8, 12]).style(rounded_action_btn_style)
        .on_press_maybe(enabled.then(|| msg(Event::Close)))].spacing(8);
    let header = row![text("历史维护设置").size(18), settings_close_button(msg(Event::Close))].spacing(12);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(Length::Fixed(620.0)).height(Length::Fixed(560.0));
    settings_modal_overlay(Some(base), msg(Event::Close), container(card).padding(12).center(Length::Fill))
}
