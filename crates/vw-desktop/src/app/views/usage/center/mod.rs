//! SDK-backed ledger entry; current-session details remain in the parent view.
pub mod display;
pub mod charts;
pub mod state;
pub mod models;
pub mod pricing;
pub mod scan;
pub mod logs;
pub mod groups;
pub mod overview;
use crate::app::{App, Message, message::ViewMessage};
use iced::{Element, Task};
use crate::app::state::agent_apps_auth::Scope;

pub use desktop_usage::Identity;

fn identity(app: &App) -> Option<Identity> {
    let project = app.project_id.as_ref().filter(|p| !p.trim().is_empty())?.clone();
    let host = app.apps_catalog.host.trim().to_owned();
    if host.is_empty() { return None; }
    Some(Identity { endpoint: Scope::from(&crate::app::config::gateway_client_endpoint()), project, host })
}

pub use desktop_usage::center::event::Event;

pub fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Display(event) => return display::update(app, event),
        Event::Scan(event) => return scan::update(app, event),
        Event::Models(event) => return models::update(app, event),
        Event::Pricing(event) => return pricing::update(app, event),
        Event::Charts(event) => return charts::update(app, event),
        Event::Logs(event) => return logs::update(app, event),
        Event::Groups(event) => return groups::update(app, event),
        Event::Filter(change) => {
            app.usage_center.filter(change);
            return update(app, Event::Refresh);
        }
        Event::Tick => {
            if app.usage_center.open && app.screen == crate::app::Screen::Usage {
                return update(app, Event::Refresh);
            }
        }
        Event::Close => {
            app.usage_center.close();
            return scan::activity(app, false);
        },
        Event::Loaded(ticket, scope, result) => {
            let current = identity(app);
            let visible = app.screen == crate::app::Screen::Usage;
            app.usage_center.accept(current.as_ref(), visible, ticket, &scope, result);
        }
        Event::Load(all) => {
            app.usage_center.range(all);
            return update(app, Event::Refresh);
        }
        Event::Refresh => {
            let query = app.usage_center.query.clone();
            let ticket = app.usage_center.begin();
            let logs_task = logs::reset(app, query.clone());
            let Some(scope) = identity(app) else {
                app.usage_center.missing_scope();
                return logs_task;
            };
            let charts_task = charts::load(ticket, scope.clone(), query.clone());
            let groups_task = groups::load(ticket, scope.clone(), query.clone(), "UTC".into());
            return Task::batch([display::update(app, display::Event::AutoLoad), scan::activity(app, true), logs_task, groups_task, charts_task, desktop_usage::center::event::load(crate::app::config::gateway_client(), ticket, scope, query)
                .map(|event| Message::View(ViewMessage::UsageCenter(event)))]);
        }
    }
    Task::none()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let known_cost = app.usage_center.result.as_ref().and_then(|result| result.as_ref().ok())
        .map(|summary| display::amount(app, &summary.known_cost_usd));
    let content = desktop_usage::center::view::view(&app.usage_center, display::status(app), known_cost)
        .map(|event| Message::View(ViewMessage::UsageCenter(event)));
    display::overlay(app, models::overlay(app, pricing::overlay(app, scan::overlay(app, content))))
}
