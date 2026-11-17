//! Usage center lifecycle and scheduling; the desktop supplies identity and feedback presentation.
use super::{
    Outcome, charts, display,
    event::{self, Event},
    groups, logs, pricing, scan,
    state::State,
};
use crate::{Identity, models};
use iced::Task;
use vw_gateway_client::GatewayClient;
pub struct Input<'a> {
    pub state: &'a mut State,
    pub current: Option<Identity>,
    pub visible: bool,
    pub client: fn() -> Result<GatewayClient, String>,
    pub feedback: Option<(bool, String)>,
}
fn display_outcome(outcome: Outcome<display::Event>) -> Event {
    match outcome {
        Outcome::Event(e) => Event::Display(e),
        Outcome::Refresh => Event::Refresh,
        Outcome::Error(e) => Event::Display(display::Event::Feedback(false, e)),
        Outcome::Success(e) => Event::Display(display::Event::Feedback(true, e)),
    }
}
fn pricing_outcome(outcome: Outcome<pricing::Event>) -> Event {
    match outcome {
        Outcome::Event(e) => Event::Pricing(e),
        Outcome::Refresh => Event::Refresh,
        Outcome::Error(e) => Event::Pricing(pricing::Event::Feedback(false, e)),
        Outcome::Success(e) => Event::Pricing(pricing::Event::Feedback(true, e)),
    }
}
fn scan_outcome(outcome: Outcome<scan::Event>) -> Event {
    match outcome {
        Outcome::Event(e) => Event::Scan(e),
        Outcome::Refresh => Event::Refresh,
        Outcome::Error(e) => Event::Scan(scan::Event::Feedback(false, e)),
        Outcome::Success(e) => Event::Scan(scan::Event::Feedback(true, e)),
    }
}
fn update_event(input: &mut Input<'_>, event: Event) -> Task<Event> {
    let state = &mut *input.state;
    let current = input.current.clone();
    let visible = input.visible;
    let client = input.client;
    match event {
        Event::Sync => { state.sync_requested = true; update(input, Event::Scan(scan::Event::Open)) }
        Event::TodayRefresh => {
            state.today = None;
            state.today_sequence = state.today_sequence.wrapping_add(1);
            let ticket = state.today_sequence;
            let Some(scope) = current else { return Task::none(); };
            Task::perform(async move {
                let result = async {
                    let range = super::range_picker::Preset::Today.resolve(chrono::Local::now())?;
                    let client = gateway_usage_read::GatewayUsageReadClient::new(client()?.transport().clone());
                    client.usage_summary(&gateway_usage_read::UsageReadScope {
                        project_id: scope.project.clone().into(), target_host: scope.host.clone(), time_zone: "UTC".into(),
                    }, &vw_api_usage::UsageQuery { range, ..super::overview::default_query() }).await.map_err(|e| e.to_string())
                }.await;
                (scope, result)
            }, move |(scope, result)| Event::TodayLoaded(ticket, scope, result))
        }
        Event::TodayLoaded(ticket, scope, result) => {
            if state.today_sequence == ticket && current.as_ref() == Some(&scope) {
                state.today = result.ok().map(|summary| (scope, summary));
            }
            Task::none()
        }
        Event::Tab(tab) => { state.tab = tab; Task::none() }
        Event::Hero(event) => { super::hero::update(&mut state.hero, event); state.charts.metric = state.hero.metric; Task::none() }
        Event::Range(event) => {
            if let Some(range) = state.range_picker.update(event, &state.query.range) {
                state.query.range = range;
                update(input, Event::Refresh)
            } else { Task::none() }
        }
        Event::Display(display::Event::Feedback(success, message))
        | Event::Pricing(pricing::Event::Feedback(success, message))
        | Event::Scan(scan::Event::Feedback(success, message)) => {
            input.feedback = Some((success, message));
            Task::none()
        }
        Event::Models(models::Event::SavedNotice) => {
            input.feedback = Some((true, "模型同步选择已保存".into()));
            Task::none()
        }
        Event::Display(event) => {
            let task = display::update(&mut state.display, current, visible, client(), event).map(display_outcome);
            state.charts.line = state.display.preferences().area;
            task
        }
        Event::Pricing(event) => {
            pricing::update(&mut state.pricing, current, visible, client(), event)
                .map(pricing_outcome)
        }
        Event::Scan(event) => {
            let synced_settings = matches!(&event, scan::Event::Done(_, scope, Ok(scan::Reply::Settings(..))) if current.as_ref() == Some(scope));
            let failed_settings = matches!(&event, scan::Event::Done(_, _, Err(_)));
            let task = scan::update(&mut state.scan, current, client(), event).map(scan_outcome);
            if failed_settings { state.sync_requested = false; }
            if synced_settings && state.sync_requested && !state.scan.pending {
                state.sync_requested = false;
                Task::batch([task, Task::done(Event::Scan(scan::Event::Run(scan::Action::Sync)))])
            } else { task }
        }
        Event::Models(event) => {
            models::update(&mut state.models, current, client, event).map(Event::Models)
        }
        Event::Charts(event) => {
            if let charts::Event::Metric(metric) = &event { state.hero.metric = *metric; }
            if let charts::Event::Line(line) = &event {
                return update(input, Event::Display(display::Event::Trend(*line)));
            }
            charts::update(&mut state.charts, current, visible, state.open, state.sequence, event)
                .map(Event::Charts)
        }
        Event::Groups(event) => {
            groups::update(&mut state.groups, current, visible, state.open, state.sequence, event)
                .map(Event::Groups)
        }
        Event::Logs(event) => logs::update(
            &mut state.logs,
            current,
            visible,
            state.open,
            state.sequence,
            client(),
            event,
        )
        .map(Event::Logs),
        Event::Filter(change) => {
            state.filter(change);
            update(input, Event::Refresh)
        }
        Event::Load(all) => {
            state.open = true;
            state.range(all);
            state.range_picker.preset = Some(if all { super::range_picker::Preset::All } else { super::range_picker::Preset::Day });
            update(input, Event::Refresh)
        }
        Event::ActivityChanged => Task::none(),
        Event::ActivitySettled(scope, active, result) => {
            state.activity_pending = false;
            if result.is_ok() { state.activity = active.then_some((scope, true)); }
            // A failure is retried on the next lifecycle notification or tick.
            Task::none()
        }
        Event::SyncStatus(ticket, scope, result) => {
            if state.sequence != ticket || state.scope.as_ref() != Some(&scope)
                || current.as_ref() != Some(&scope) { return Task::none(); }
            state.status_pending = false;
            if !state.open || !visible { return Task::none(); }
            if let Ok(last) = result {
                let previous = state.scan.last.as_ref().and_then(|last| last.last_complete_scan);
                let changed = last.last_complete_scan != previous;
                state.scan.last = Some(last);
                if changed { return update(input, Event::Refresh); }
            }
            Task::none()
        }
        Event::Tick if state.open && visible => {
            let now = std::time::Instant::now();
            let rolling = !matches!(&state.query.range,
                vw_api_usage::UsageRange::All | vw_api_usage::UsageRange::Absolute { .. } |
                vw_api_usage::UsageRange::Custom { live_end_time: false, .. });
            if rolling && !state.loading && state.last_query.is_none_or(|last|
                now.saturating_duration_since(last).as_secs() >= 60) {
                return update(input, Event::Refresh);
            }
            if state.status_pending || state.last_status.is_some_and(|last|
                now.saturating_duration_since(last).as_secs() < 5) { return Task::none(); }
            let Some(scope) = current else { return Task::none(); };
            state.status_pending = true;
            state.last_status = Some(now);
            let ticket = state.sequence;
            Task::perform(async move {
                let result = async {
                    let client = gateway_usage_scan::GatewayUsageScanClient::new(client()?.transport().clone());
                    client.usage_last_scan(&gateway_usage_scan::UsageScanScope {
                        project_id: scope.project.clone().into(), target_host: scope.host.clone(),
                    }).await.map_err(|error| error.to_string())
                }.await;
                (scope, result)
            }, move |(scope, result)| Event::SyncStatus(ticket, scope, result))
        }
        Event::Tick => Task::none(),
        Event::Close => {
            state.close();
            Task::none()
        }
        Event::Loaded(ticket, scope, result) => {
            state.accept(current.as_ref(), visible, ticket, &scope, result);
            Task::none()
        }
        Event::Refresh => {
            if !visible || !state.open { return Task::none(); }
            if let Ok(range) = state.range_picker.refresh_range(&state.query.range) { state.query.range = range; }
            state.scope = current.clone();
            state.status_pending = false;
            let query = state.query.clone();
            let ticket = state.begin();
            let logs = logs::reset(
                &mut state.logs,
                current.clone(),
                visible,
                state.open,
                state.sequence,
                client(),
                query.clone(),
            )
            .map(Event::Logs);
            let Some(scope) = current else {
                state.missing_scope();
                return logs;
            };
            let charts =
                charts::load(client(), ticket, scope.clone(), query.clone()).map(Event::Charts);
            let groups = groups::load(client(), ticket, scope.clone(), query.clone(), "UTC".into())
                .map(Event::Groups);
            let display = display::update(
                &mut state.display,
                Some(scope.clone()),
                visible,
                client(),
                display::Event::AutoLoad,
            )
            .map(display_outcome);
            Task::batch([
                display,
                logs,
                groups,
                charts,
                event::load(client(), ticket, scope, query),
            ])
        }
    }
}

/// Reconcile activity independently of query requests. The host owns scan timers
/// and single-flight jobs; hiding a page or sheet never cancels those jobs.
pub fn update(input: &mut Input<'_>, event: Event) -> Task<Event> {
    let failed_activity = matches!(&event, Event::ActivitySettled(_, _, Err(_)));
    let scope_changed = input.state.scope != input.current;
    if scope_changed {
        let reopen = input.state.open;
        input.state.close();
        input.state.today = None;
        input.state.today_sequence = input.state.today_sequence.wrapping_add(1);
        input.state.scope = input.current.clone();
        input.state.scan = scan::State::default();
        input.state.logs = logs::State::default();
        input.state.status_pending = false;
        input.state.last_status = None;
        input.state.open = reopen;
    }
    let resumed = matches!(&event, Event::ActivityChanged) && input.visible && input.state.open;
    let reload_scope = scope_changed && input.state.open && input.visible
        && !matches!(&event, Event::Close | Event::Load(_) | Event::Refresh | Event::Filter(_));
    let task = if resumed { update_event(input, Event::Refresh) }
        else { update_event(input, event) };
    let task = if reload_scope { Task::batch([task, update_event(input, Event::Refresh)]) }
        else { task };
    if input.state.activity_pending || failed_activity { return task; }
    let desired = if input.state.open && input.visible { input.current.clone() } else { None };
    let previous = input.state.activity.as_ref().map(|(scope, _)| scope.clone());
    if previous == desired { return task; }
    // Retain the original authenticated transport when retiring an old host.
    let (scope, active) = if let Some(old) = previous { (old, false) }
        else if let Some(scope) = desired { (scope, true) }
        else { return task; };
    input.state.activity_pending = true;
    let paused = input.state.scan.scope.as_ref() == Some(&scope) && input.state.scan.paused;
    let client = if active {
        let client = (input.client)();
        if let Ok(client) = &client { input.state.activity_client = Some(client.clone()); }
        client
    } else {
        input.state.activity_client.clone().ok_or_else(|| "页面扫描连接已失效".to_owned())
    };
    let activity = Task::perform(async move {
        let result = async {
            let client = gateway_usage_scan::GatewayUsageScanClient::new(client?.transport().clone());
            client.usage_activity(&gateway_usage_scan::UsageScanScope {
                project_id: scope.project.clone().into(), target_host: scope.host.clone(),
            }, active, paused).await.map_err(|error| error.to_string())
        }.await;
        (scope, result)
    }, move |(scope, result)| Event::ActivitySettled(scope, active, result));
    Task::batch([task, activity])
}
