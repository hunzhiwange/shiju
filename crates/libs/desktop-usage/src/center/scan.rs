//! Scan controls send intents to the authorized host; closing the dialog never cancels work.
use crate::Identity;
use super::Outcome;

use iced::{Element, Task};
use gateway_usage_scan::{UsageJobSnapshot, UsageScanScope, UsageSourceSettingsState, UsageLastScan};

#[derive(Default)]
pub struct State {
    pub open: bool,
    navigation: Option<Navigation>,
    rebuild_job: bool,
    pub(super) scope: Option<Identity>,
    sequence: u64,
    pub(super) pending: bool,
    pub(super) paused: bool,
    pub(super) sources: Vec<UsageSourceSettingsState>,
    pub(super) last: Option<UsageLastScan>,
    pub(super) job: Option<UsageJobSnapshot>,
    pub(super) error: Option<String>,
    pub(super) confirm_rebuild: bool,
}
#[derive(Clone, Debug)]
pub enum Action { Interval(String, u32), Load, Sync, Cancel, Poll, Pause(bool), Source(String, bool), Rebuild, Rollup, Backfill }
#[derive(Clone, Debug)]
pub enum Event {
    Feedback(bool, String),
    Open, Close, ConfirmRebuild, DismissRebuild, Navigate(Navigation), Run(Action),
    ActivityDone(Result<(), String>),
    ScopedActivityDone(u64, Identity, Result<(), String>),
    Done(u64, Identity, Result<Reply, String>),
    Poll(u64, Identity),
}
/// The center/host drains this intent and opens its existing settings destination.
#[derive(Clone, Copy, Debug)]
pub enum Navigation { Routing, Display, Pricing }
pub fn take_navigation(state: &mut State) -> Option<Navigation> { state.navigation.take() }
#[derive(Clone, Debug)]
pub enum Reply {
    Settings(Vec<UsageSourceSettingsState>, UsageLastScan),
    Job(UsageJobSnapshot), Saved(UsageSourceSettingsState), Paused(bool),
}
fn message(event: Event) -> Outcome<Event> { Outcome::Event(event) }
pub(super) fn terminal(job: &UsageJobSnapshot) -> bool {
    ["phase", "status"].iter().any(|key| job.progress.get(*key).and_then(|v| v.as_str())
        .is_some_and(|s| matches!(s, "Completed" | "Cancelled" | "Failed" | "Terminal" | "completed" | "cancelled" | "failed")))
}
fn job_failed(job: &UsageJobSnapshot) -> bool {
    ["phase", "status"].iter().any(|key| job.progress.get(*key).and_then(|v| v.as_str())
        .is_some_and(|s| matches!(s, "Cancelled" | "Failed" | "cancelled" | "failed")))
        || job.progress.get("errors").and_then(|v| v.as_array()).is_some_and(|errors| !errors.is_empty())
}
fn poll(sequence: u64, scope: Identity) -> Task<Outcome<Event>> {
    Task::perform(async { tokio::time::sleep(std::time::Duration::from_secs(2)).await },
        move |_| message(Event::Poll(sequence, scope.clone())))
}
pub fn update(state: &mut State, current: Option<Identity>, client: Result<vw_gateway_client::GatewayClient, String>, event: Event) -> Task<Outcome<Event>> {
    match event {
        Event::Feedback(..) => {},
        Event::ScopedActivityDone(sequence, scope, result) => {
            if state.sequence == sequence && current.as_ref() == Some(&scope) {
                if let Err(error) = result { return Task::done(Outcome::Error(error)); }
            }
        }
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
        Event::Close => { state.open = false; state.confirm_rebuild = false; }
        Event::ConfirmRebuild => {
            if !state.pending && state.scope == current && !state.job.as_ref().is_some_and(|job| !terminal(job)) { state.confirm_rebuild = true; }
        }
        Event::DismissRebuild => state.confirm_rebuild = false,
        Event::Navigate(destination) => { state.open = false; state.navigation = Some(destination); }
        Event::Poll(sequence, scope) => {
            if state.sequence == sequence && current.as_ref() == Some(&scope) {
                if state.pending { return poll(sequence, scope); }
                return update(state, current, client, Event::Run(Action::Poll));
            }
        }
        Event::Done(sequence, scope, result) => {
            if state.sequence != sequence || state.scope.as_ref() != Some(&scope) || current.as_ref() != Some(&scope) { return Task::none(); }

            state.pending = false;
            match result {
                Err(error) => {
                    state.error = Some(error.clone());
                    return Task::done(Outcome::Error(format!("用量操作失败：{error}").into()));
                }
                Ok(Reply::Settings(sources, last)) => { state.sources = sources; state.last = Some(last);
                    if state.job.as_ref().is_some_and(|job| !terminal(job)) { return poll(sequence, scope); } }
                Ok(Reply::Saved(source)) => {
                    if let Some(old) = state.sources.iter_mut().find(|old| old.app == source.app) { *old = source; }
                    let resume = if state.job.as_ref().is_some_and(|job| !terminal(job)) { poll(sequence, scope) } else { Task::none() };
                    return Task::batch([resume, Task::done(Outcome::Success("扫描设置已保存".into()))]);
                }
                Ok(Reply::Paused(paused)) => { state.paused = paused;
                    let resume = if state.job.as_ref().is_some_and(|job| !terminal(job)) { poll(sequence, scope) } else { Task::none() };
                    return Task::batch([resume, Task::done(Outcome::Success("自动扫描设置已保存".into()))]); },
                Ok(Reply::Job(job)) => {
                    let active = !terminal(&job);
                    state.job = Some(job);
                    if active { return poll(sequence, scope); }
                    let failed = job_failed(state.job.as_ref().expect("stored job"));
                    let notice = if failed { Outcome::Error("用量任务未成功完成，请重试或查看主机任务详情".into()) }
                        else { Outcome::Success(if state.rebuild_job { "Codex 重建完成；主机未返回备份凭证" } else { "用量同步完成" }.into()) };
                    return Task::batch([Task::done(notice), Task::done(Outcome::Refresh), update(state, current, client, Event::Run(Action::Load))]);
                }
            }
        }
        Event::Run(action) => {
            let Some(scope) = current.clone() else { return Task::done(Outcome::Error("网关不可用，请检查连接".into())); };

            if state.scope.as_ref() != Some(&scope) || state.pending { return Task::none(); }
            let active = state.job.as_ref().is_some_and(|job| !terminal(job));
            if active && matches!(action, Action::Sync | Action::Rebuild | Action::Rollup | Action::Backfill) { return Task::none(); }
            if matches!(action, Action::Rebuild) && !state.confirm_rebuild { return Task::none(); }
            let job_id = state.job.as_ref().map(|j| j.task_id.clone());
            if matches!(action, Action::Cancel | Action::Poll) && !active { return Task::none(); }
            let source = if let Action::Source(ref name, _) | Action::Interval(ref name, _) = action {
                state.sources.iter().find(|s| &s.app == name).map(|s| {
                    let mut settings = s.settings.clone();
                    match action { Action::Source(_, enabled) => settings.scan_enabled = enabled,
                        Action::Interval(_, seconds) => settings.refresh_seconds = Some(seconds), _ => {} }
                    (s.app.clone(), s.revision, settings)
                })
            } else { None };
            if matches!(action, Action::Interval(_, seconds) if ![5, 10, 30, 60].contains(&seconds)) { return Task::none(); }
            if matches!(action, Action::Source(..) | Action::Interval(..)) && source.is_none() { return Task::none(); }
            if matches!(action, Action::Sync | Action::Rebuild) { state.rebuild_job = matches!(action, Action::Rebuild); }
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
                        Action::Source(..) | Action::Interval(..) => {
                            let (name, revision, settings) = source.expect("validated source");
                            let saved = client.usage_save_source_settings(&target, &name, revision, &settings).await.map_err(|e| e.to_string())?;
                            Ok(Reply::Saved(UsageSourceSettingsState { app: name, revision: saved.revision, settings: saved.settings }))
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
pub fn overlay<'a, M: Clone + 'static>(state: &'a State, current: Option<Identity>, base: Element<'a, M>, map: fn(Outcome<Event>) -> M) -> Element<'a, M> {
    super::settings_sheet::overlay(state, current, base, map)
}

/// Activity only controls the page scheduler, never the persistent source policy.
pub fn activity(state: &State, current: Option<Identity>, client: Result<vw_gateway_client::GatewayClient, String>, active: bool) -> Task<Outcome<Event>> {
    let Some(scope) = current.clone() else { return Task::none(); };
    let paused = state.scope.as_ref() == Some(&scope) && state.paused;
    let sequence = state.sequence;
    let reply_scope = scope.clone();

    Task::perform(async move {
        let client = gateway_usage_scan::GatewayUsageScanClient::new(client?.transport().clone());
        client.usage_activity(&UsageScanScope { project_id: scope.project.into(), target_host: scope.host }, active, paused)
            .await.map_err(|e| e.to_string())
    }, move |result| message(Event::ScopedActivityDone(sequence, reply_scope.clone(), result)))
}

#[cfg(test)]
#[path = "../usage_replica_06_tests.rs"]
mod usage_replica_06_tests;
