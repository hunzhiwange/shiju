//! Scan controls send intents to the authorized host; closing the dialog never cancels work.
use super::Outcome;
use crate::Identity;
use gateway_usage_scan::{UsageJobSnapshot, UsageLastScan, UsageScanPreferences, UsageScanScope};
use iced::{Element, Task};

#[derive(Default)]
pub struct State {
    pub open: bool,
    navigation: Option<Navigation>,
    pub(super) scope: Option<Identity>,
    sequence: u64,
    pub(super) pending: bool,
    pub(super) preferences: Option<UsageScanPreferences>,
    pub(super) last: Option<UsageLastScan>,
    pub(super) job: Option<UsageJobSnapshot>,
    pub(super) error: Option<String>,
}
impl State {
    pub(super) fn set_scope(&mut self, scope: Option<Identity>) {
        if self.scope != scope {
            *self = Self { sequence: self.sequence.wrapping_add(1), scope, ..Self::default() };
        }
    }

    pub(super) fn last_sync_time(&self) -> Option<chrono::DateTime<chrono::Local>> {
        let elapsed = self.last.as_ref()?.last_complete_scan?
            .duration_since(std::time::UNIX_EPOCH).ok()?;
        let time = chrono::DateTime::from_timestamp(
            elapsed.as_secs().try_into().ok()?,
            elapsed.subsec_nanos(),
        )?;
        Some(time.with_timezone(&chrono::Local))
    }
}
#[derive(Clone, Debug)]
pub enum Action { Save(UsageScanPreferences), Load, Sync, Cancel, Poll, Rollup, Backfill }
#[derive(Clone, Debug)]
pub enum Event {
    Feedback(bool, String),
    Completed(Identity, bool, String),
    RefreshHead,
    Open, Close, Navigate(Navigation), Run(Action),
    Done(u64, Identity, Result<Reply, String>),
    Poll(u64, Identity),
}
/// The center/host drains this intent and opens its existing settings destination.
#[derive(Clone, Copy, Debug)]
pub enum Navigation { Routing, Display, Pricing }
pub fn take_navigation(state: &mut State) -> Option<Navigation> { state.navigation.take() }
#[derive(Clone, Debug)]
pub enum Reply {
    Settings(UsageScanPreferences, UsageLastScan),
    Job(UsageJobSnapshot), Saved(UsageScanPreferences),
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

fn scan_summary(job: &UsageJobSnapshot) -> Option<(bool, String)> {
    let phase = job.progress.get("phase")?.as_str()?;
    if !matches!(phase, "Completed" | "completed" | "Failed" | "failed") {
        return None;
    }
    let counts = job.progress.get("counts")?;
    let imported = counts.get("imported")?.as_u64()?;
    let files = counts.get("files_scanned")?.as_u64()?;
    let errors = job.progress.get("errors")?.as_array()?.len();
    let success = matches!(phase, "Completed" | "completed") && errors == 0;
    let title = if success { "同步完成" } else { "同步失败" };
    Some((success, format!("{title}：新增 {imported} 条，扫描 {files} 个文件，错误 {errors}")))
}

fn poll(sequence: u64, scope: Identity) -> Task<Outcome<Event>> {
    Task::perform(async { tokio::time::sleep(std::time::Duration::from_secs(2)).await },
        move |_| message(Event::Poll(sequence, scope.clone())))
}
pub fn update(state: &mut State, current: Option<Identity>, client: Result<vw_gateway_client::GatewayClient, String>, event: Event) -> Task<Outcome<Event>> {
    match event {
        Event::Feedback(..) | Event::Completed(..) | Event::RefreshHead => {},
        Event::Open => {
            state.set_scope(current.clone());
            state.open = true;
            return update(state, current, client, Event::Run(Action::Load));
        }
        Event::Close => state.open = false,
        Event::Navigate(destination) => {
            if matches!(destination, Navigation::Routing) { state.open = false; }
            state.navigation = Some(destination);
        }
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
                Ok(Reply::Settings(preferences, last)) => { state.preferences = Some(preferences); state.last = Some(last);
                    if state.job.as_ref().is_some_and(|job| !terminal(job)) { return poll(sequence, scope); } }
                Ok(Reply::Saved(preferences)) => {
                    state.preferences = Some(preferences);
                    let resume = if state.job.as_ref().is_some_and(|job| !terminal(job)) { poll(sequence, scope) } else { Task::none() };
                    return Task::batch([resume, Task::done(Outcome::Success("自动扫描设置已保存".into()))]);
                }
                Ok(Reply::Job(job)) => {
                    let active = !terminal(&job);
                    let refresh = if job.progress.contains_key("counts") {
                        message(Event::RefreshHead)
                    } else {
                        Outcome::Refresh
                    };
                    let notice = if active {
                        None
                    } else if let Some((success, summary)) = scan_summary(&job) {
                        Some(message(Event::Completed(scope.clone(), success, summary)))
                    } else if job_failed(&job) {
                        Some(Outcome::Error("用量任务未成功完成，请重试或查看主机任务详情".into()))
                    } else {
                        Some(Outcome::Success("用量同步完成".into()))
                    };
                    state.job = Some(job);
                    let Some(notice) = notice else { return poll(sequence, scope); };
                    return Task::batch([Task::done(notice), Task::done(refresh), update(state, current, client, Event::Run(Action::Load))]);
                }
            }
        }
        Event::Run(action) => {
            let Some(scope) = current.clone() else { return Task::done(Outcome::Error("网关不可用，请检查连接".into())); };

            if state.scope.as_ref() != Some(&scope) || state.pending { return Task::none(); }
            let active = state.job.as_ref().is_some_and(|job| !terminal(job));
            if active && matches!(action, Action::Sync | Action::Rollup | Action::Backfill) { return Task::none(); }
            let job_id = state.job.as_ref().map(|j| j.task_id.clone());
            if matches!(action, Action::Cancel | Action::Poll) && !active { return Task::none(); }
            state.pending = true;
            state.sequence = state.sequence.wrapping_add(1);
            state.error = None;
            let sequence = state.sequence;

            let target = UsageScanScope { project_id: scope.project.clone().into(), target_host: scope.host.clone() };
            return Task::perform(async move {
                let result = async {
                    let client = gateway_usage_scan::GatewayUsageScanClient::new(client?.transport().clone());
                    let result = match action {
                        Action::Load => Ok(Reply::Settings(client.usage_scan_preferences(&target).await.map_err(|e| e.to_string())?, client.usage_last_scan(&target).await.map_err(|e| e.to_string())?)),
                        Action::Sync => client.usage_start_scan(&target).await.map(Reply::Job),
                        Action::Cancel => client.usage_cancel_job(&target, job_id.as_deref().unwrap_or_default()).await.map(Reply::Job),
                        Action::Poll => client.usage_job(&target, job_id.as_deref().unwrap_or_default()).await.map(Reply::Job),
                        Action::Save(preferences) => client.usage_save_scan_preferences(&target, preferences).await.map(Reply::Saved),
                        Action::Rollup => client.usage_rollup(&target).await.map(Reply::Job),
                        Action::Backfill => client.usage_backfill(&target).await.map(Reply::Job),
                    };
                    result.map_err(|e: gateway_usage_scan::UsageScanError| e.to_string())
                }.await;
                (scope, result)
            }, move |(scope, result)| message(Event::Done(sequence, scope, result)));
        }
    }
    Task::none()
}
pub fn overlay<'a, M: Clone + 'static>(
    state: &'a State,
    current: Option<Identity>,
    base: Element<'a, M>,
    display: Element<'a, M>,
    map: fn(Outcome<Event>) -> M,
) -> Element<'a, M> {
    super::settings_sheet::overlay(state, current, base, display, map)
}

#[cfg(test)]
#[path = "../usage_replica_06_tests.rs"]
mod usage_replica_06_tests;
