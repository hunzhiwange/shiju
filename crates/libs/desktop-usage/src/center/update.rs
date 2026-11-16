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
pub fn update(input: &mut Input<'_>, event: Event) -> Task<Event> {
    let state = &mut *input.state;
    let current = input.current.clone();
    let visible = input.visible;
    let client = input.client;
    match event {
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
            display::update(&mut state.display, current, visible, client(), event)
                .map(display_outcome)
        }
        Event::Pricing(event) => {
            pricing::update(&mut state.pricing, current, visible, client(), event)
                .map(pricing_outcome)
        }
        Event::Scan(event) => {
            scan::update(&mut state.scan, current, client(), event).map(scan_outcome)
        }
        Event::Models(event) => {
            models::update(&mut state.models, current, client, event).map(Event::Models)
        }
        Event::Charts(event) => {
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
            state.range(all);
            update(input, Event::Refresh)
        }
        Event::Tick if state.open && visible => update(input, Event::Refresh),
        Event::Tick => Task::none(),
        Event::Close => {
            state.close();
            scan::activity(&state.scan, current, client(), false).map(scan_outcome)
        }
        Event::Loaded(ticket, scope, result) => {
            state.accept(current.as_ref(), visible, ticket, &scope, result);
            Task::none()
        }
        Event::Refresh => {
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
            let scan =
                scan::activity(&state.scan, Some(scope.clone()), client(), true).map(scan_outcome);
            Task::batch([
                display,
                scan,
                logs,
                groups,
                charts,
                event::load(client(), ticket, scope, query),
            ])
        }
    }
}
