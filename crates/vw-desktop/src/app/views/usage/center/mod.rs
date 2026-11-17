//! SDK-backed ledger entry; current-session details remain in the parent view.
pub mod charts;
pub mod display;
pub mod groups;
pub mod logs;
pub mod models;
pub mod overview;
pub mod pricing;
pub mod scan;
pub mod state;
use crate::app::state::agent_apps_auth::Scope;
use crate::app::{App, Message, message::ViewMessage};
use iced::{Element, Task};

pub use desktop_usage::Identity;

fn identity(app: &App) -> Option<Identity> {
    let project = app.workspace_state.project_id.as_ref().filter(|p| !p.trim().is_empty())?.clone();
    let host = app.apps_catalog.host.trim().to_owned();
    if host.is_empty() {
        return None;
    }
    Some(Identity {
        endpoint: Scope::from(&crate::app::config::gateway_client_endpoint()),
        project,
        host,
    })
}

pub use desktop_usage::center::event::Event;

pub fn update(app: &mut App, event: Event) -> Task<Message> {
    let current = identity(app);
    let mut input = desktop_usage::center::update::Input {
        state: &mut app.usage_center,
        current,
        visible: app.navigation_tabs.screen == crate::app::Screen::Usage,
        client: crate::app::config::gateway_client,
        feedback: None,
    };
    let task = desktop_usage::center::update::update(&mut input, event)
        .map(|event| Message::View(ViewMessage::UsageCenter(event)));
    let feedback = input.feedback.take();
    let navigation = desktop_usage::center::scan::take_navigation(&mut app.usage_center.scan);
    let navigation = match navigation {
        Some(desktop_usage::center::scan::Navigation::Display) => update(app, Event::Display(display::Event::Open)),
        Some(desktop_usage::center::scan::Navigation::Pricing) => update(app, Event::Pricing(pricing::Event::Open)),
        Some(desktop_usage::center::scan::Navigation::Routing) => {
            app.apps_catalog.third_party_page = desktop_ui_controls::settings_navigation::ThirdPartyPage::ManagementTools;
            Task::done(Message::View(ViewMessage::OpenThirdPartyAgents))
        }
        None => Task::none(),
    };
    Task::batch([
        task,
        navigation,
        feedback.map_or_else(Task::none, |(success, message)| {
            if success { app.show_success_toast(message) } else { app.show_error_toast(message) }
        }),
    ])
}

pub fn view(app: &App) -> Element<'_, Message> {
    let known_cost = app
        .usage_center
        .result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .map(|summary| display::amount(app, &summary.known_cost_usd));
    let content =
        desktop_usage::center::view::view(&app.usage_center, display::status(app), known_cost)
            .map(|event| Message::View(ViewMessage::UsageCenter(event)));
    display::overlay(app, models::overlay(app, pricing::overlay(app, scan::overlay(app, content))))
}

/// Entry summary is a separate local-day query, never the current dashboard filter.
pub fn entry(app: &App) -> Element<'_, Message> {
    let summary = app.usage_center.today.as_ref().filter(|(scope, _)| Some(scope) == identity(app).as_ref()).map(|(_, summary)| summary);
    let label = summary.map_or_else(|| "今日金额 —".into(), |summary| format!("今日金额 {} · 未知费用 {} 请求", display::amount(app, &summary.known_cost_usd), summary.counts.unknown_cost_requests));
    iced::widget::button(iced::widget::text(format!("用量统计 ›\n{label}")).size(13))
        .padding([10, 12]).style(desktop_ui_controls::system_settings_common::rounded_action_btn_style)
        .on_press(Message::View(ViewMessage::UsageCenter(Event::Load(true)))).into()
}
