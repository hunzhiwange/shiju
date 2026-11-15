//! Scan controls send intents to the authorized host; closing the dialog never cancels work.
use crate::Identity;
use super::Outcome;

use iced::{Element, Task, Length, widget::{button, checkbox, column, row, text}};
use gateway_usage_scan::{UsageJobSnapshot, UsageScanScope, UsageSourceSettingsState, UsageLastScan};
use desktop_ui_controls::system_settings_common::*;

#[derive(Default)]
pub struct State {
    pub open: bool,
    scope: Option<Identity>,
    sequence: u64,
    pending: bool,
    paused: bool,
    sources: Vec<UsageSourceSettingsState>,
    last: Option<UsageLastScan>,
    job: Option<UsageJobSnapshot>,
    error: Option<String>,
    confirm_rebuild: bool,
}
#[derive(Clone, Debug)]
pub enum Action { Load, Sync, Cancel, Poll, Pause(bool), Source(String, bool), Rebuild, Rollup, Backfill }
#[derive(Clone, Debug)]
pub enum Event {
    Feedback(bool, String),
    Open, Close, ConfirmRebuild, Run(Action),
    ActivityDone(Result<(), String>),
    Done(u64, Identity, Result<Reply, String>),
    Poll(u64, Identity),
}
#[derive(Clone, Debug)]
pub enum Reply {
    Settings(Vec<UsageSourceSettingsState>, UsageLastScan),
    Job(UsageJobSnapshot), Paused(bool),
}
fn message(event: Event) -> Outcome<Event> { Outcome::Event(event) }
fn terminal(job: &UsageJobSnapshot) -> bool {
    job.progress.get("phase").and_then(|v| v.as_str()).is_some_and(|s|
        matches!(s, "Completed" | "Cancelled" | "Failed" | "Terminal"))
}
fn poll(sequence: u64, scope: Identity) -> Task<Outcome<Event>> {
    Task::perform(async { tokio::time::sleep(std::time::Duration::from_secs(2)).await },
        move |_| message(Event::Poll(sequence, scope.clone())))
}
pub fn update(state: &mut State, current: Option<Identity>, client: Result<vw_gateway_client::GatewayClient, String>, event: Event) -> Task<Outcome<Event>> {
    match event {
        Event::Feedback(..) => {},
        Event::ActivityDone(result) => { if let Err(error) = result { return Task::done(Outcome::Error(error.into())); } }
        Event::Open => {
            let scope = current.clone();
            if state.scope != scope {
                let sequence = state.sequence.wrapping_add(1);
                (*state) = State { sequence, scope, ..State::default() };
            }
            state.open = true;
            return update(state, current, client, Event::Run(Action::Load));
        }
        Event::Close => { state.open = false; }
        Event::ConfirmRebuild => state.confirm_rebuild = true,
        Event::Poll(sequence, scope) => {
            if state.sequence == sequence && current.as_ref() == Some(&scope) {
                if state.pending { return poll(sequence, scope); }
                return update(state, current, client, Event::Run(Action::Poll));
            }
        }
        Event::Done(sequence, scope, result) => {
            if state.sequence != sequence || current.as_ref() != Some(&scope) { return Task::none(); }

            state.pending = false;
            match result {
                Err(error) => {
                    state.error = Some(error.clone());
                    return Task::done(Outcome::Error(format!("用量操作失败：{error}").into()));
                }
                Ok(Reply::Settings(sources, last)) => { state.sources = sources; state.last = Some(last);
                    if state.job.as_ref().is_some_and(|job| !terminal(job)) { return poll(sequence, scope); } }
                Ok(Reply::Paused(paused)) => state.paused = paused,
                Ok(Reply::Job(job)) => {
                    let active = !terminal(&job);
                    state.job = Some(job);
                    if active { return poll(sequence, scope); }
                    return update(state, current, client, Event::Run(Action::Load));
                }
            }
        }
        Event::Run(action) => {
            let Some(scope) = current.clone() else { return Task::done(Outcome::Error("请选择项目与执行主机".into())); };

            if state.scope.as_ref() != Some(&scope) || state.pending { return Task::none(); }
            let active = state.job.as_ref().is_some_and(|job| !terminal(job));
            if active && matches!(action, Action::Sync | Action::Rebuild | Action::Rollup | Action::Backfill) { return Task::none(); }
            if matches!(action, Action::Rebuild) && !state.confirm_rebuild { return Task::none(); }
            let job_id = state.job.as_ref().map(|j| j.task_id.clone());
            if matches!(action, Action::Cancel | Action::Poll) && !active { return Task::none(); }
            let source = if let Action::Source(ref name, enabled) = action {
                state.sources.iter().find(|s| &s.app == name).map(|s| {
                    let mut settings = s.settings.clone(); settings.scan_enabled = enabled;
                    (s.app.clone(), s.revision, settings)
                })
            } else { None };
            if matches!(action, Action::Source(..)) && source.is_none() { return Task::none(); }
            state.confirm_rebuild = false;
            state.pending = true;
            state.sequence = state.sequence.wrapping_add(1);
            state.error = None;
            let sequence = state.sequence;

            let target = UsageScanScope { project_id: scope.project.clone().into(), target_host: scope.host.clone() };
            return Task::perform(async move {
                let result = async {
                    let client = gateway_usage_scan::GatewayUsageScanClient::new(client?.transport().clone());
                    let result = match action {
                        Action::Load => Ok(Reply::Settings(client.usage_source_settings(&target).await.map_err(|e| e.to_string())?, client.usage_last_scan(&target).await.map_err(|e| e.to_string())?)),
                        Action::Sync => client.usage_start_scan(&target).await.map(Reply::Job),
                        Action::Cancel => client.usage_cancel_job(&target, job_id.as_deref().unwrap_or_default()).await.map(Reply::Job),
                        Action::Poll => client.usage_job(&target, job_id.as_deref().unwrap_or_default()).await.map(Reply::Job),
                        Action::Pause(paused) => client.usage_activity(&target, true, paused).await.map(|_| Reply::Paused(paused)),
                        Action::Rebuild => client.usage_rebuild_codex(&target).await.map(Reply::Job),
                        Action::Rollup => client.usage_rollup(&target).await.map(Reply::Job),
                        Action::Backfill => client.usage_backfill(&target).await.map(Reply::Job),
                        Action::Source(..) => {
                            let (name, revision, settings) = source.expect("validated source");
                            client.usage_save_source_settings(&target, &name, revision, &settings).await.map_err(|e| e.to_string())?;
                            Ok(Reply::Settings(client.usage_source_settings(&target).await.map_err(|e| e.to_string())?, client.usage_last_scan(&target).await.map_err(|e| e.to_string())?))
                        }
                    };
                    result.map_err(|e: gateway_usage_scan::UsageScanError| e.to_string())
                }.await;
                (scope, result)
            }, move |(scope, result)| message(Event::Done(sequence, scope, result)));
        }
    }
    Task::none()
}
fn action(label: &'static str, event: Event, enabled: bool) -> Element<'static, Outcome<Event>> {
    button(text(label).size(13)).padding([6, 10]).style(rounded_action_btn_style)
        .on_press_maybe(enabled.then(|| message(event))).into()
}
pub fn overlay<'a, M: Clone + 'a>(state: &'a State, current: Option<Identity>, base: Element<'a, M>, map: fn(Outcome<Event>) -> M) -> Element<'a, M> {

    if !state.open { return base; }
    let ready = !state.pending && state.scope.as_ref() == current.as_ref();
    let active = state.job.as_ref().is_some_and(|job| !terminal(job));
    let mut body = column![
        text("操作范围：当前项目与执行主机。关闭此窗口不取消后台任务。"),
        text("页面暂停仅暂停页面扫描；来源开关控制该来源后台自动扫描。"),
        row![action("立即同步", Event::Run(Action::Sync), ready && !active),
            action(if state.paused { "恢复页面扫描" } else { "暂停页面扫描" }, Event::Run(Action::Pause(!state.paused)), ready),
            action("重新读取设置", Event::Run(Action::Load), ready)].spacing(8),
    ].spacing(12);
    let last = state.last.as_ref().and_then(|last| last.last_complete_scan)
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|t| chrono::DateTime::from_timestamp(t.as_secs() as i64, 0))
        .map(|t| t.to_rfc3339()).unwrap_or_else(|| "尚无完整同步".into());
    body = body.push(text(format!("上次完整同步（UTC）：{last}")));
    for source in &state.sources {
        let name = source.app.clone();
        body = body.push(checkbox(source.settings.scan_enabled).label(format!("{} 后台自动扫描", source.app))
            .style(settings_checkbox_style)
            .on_toggle_maybe(ready.then_some(move |enabled| message(Event::Run(Action::Source(name.clone(), enabled))))));
    }
    if let Some(job) = &state.job {
        // Render only public progress fields, never source paths or log bodies.
        for key in ["phase", "status", "current_adapter", "completed_adapters", "counts", "errors", "cancel_requested"] {
            if let Some(value) = job.progress.get(key) { body = body.push(text(format!("{key}: {value}"))); }
        }
        body = body.push(row![action("取消当前任务", Event::Run(Action::Cancel), ready && active), action("重试查询进度", Event::Run(Action::Poll), ready && active)].spacing(8));
    }
    if let Some(error) = &state.error { body = body.push(text(format!("操作失败：{error}；可重新读取设置或重试任务。"))); }
    body = body.push(text("Codex 重建将替换当前项目和主机的原生 Codex 用量；汇总维护会归档历史明细；补价仅更新未知计算费用。"))
        .push(row![action("重建 Codex…", Event::ConfirmRebuild, ready && !active),
            action("归档汇总维护", Event::Run(Action::Rollup), ready && !active),
            action("补齐未知价格", Event::Run(Action::Backfill), ready && !active)].spacing(8));
    if state.confirm_rebuild { body = body.push(action("确认替换 Codex 用量", Event::Run(Action::Rebuild), ready && !active)); }
    let card = settings_modal_layout(
        settings_modal_header(text("扫描与修复"), settings_close_button(message(Event::Close))),
        body, Some(action("关闭（后台继续）", Event::Close, true)),
    ).width(Length::Fixed(760.0)).height(Length::Fixed(560.0));
    settings_modal_overlay(Some(base), map(message(Event::Close)), Element::from(card).map(map))
}

/// Activity only controls the page scheduler, never the persistent source policy.
pub fn activity(state: &State, current: Option<Identity>, client: Result<vw_gateway_client::GatewayClient, String>, active: bool) -> Task<Outcome<Event>> {
    let Some(scope) = current.clone() else { return Task::none(); };
    let paused = state.paused;

    Task::perform(async move {
        let client = gateway_usage_scan::GatewayUsageScanClient::new(client?.transport().clone());
        client.usage_activity(&UsageScanScope { project_id: scope.project.into(), target_host: scope.host }, active, paused)
            .await.map_err(|e| e.to_string())
    }, |result| message(Event::ActivityDone(result)))
}
