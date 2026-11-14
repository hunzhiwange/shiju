//! SDK-backed ledger entry; current-session details remain in the parent view.
pub mod display;
pub mod charts;
pub mod state;
pub mod pricing;
pub mod scan;
pub mod logs;
pub mod groups;
pub mod overview;
use crate::app::{App, Message, message::ViewMessage};
use iced::{Element, Task, widget::{button, column, row, text}};
use vw_gateway_client::{UsageReadScope, UsageSummary};
use vw_gateway_client::vw_api_types::usage::UsageRange;
use crate::app::state::agent_apps_auth::Scope;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    endpoint: Scope,
    project: String,
    host: String,
}

fn identity(app: &App) -> Option<Identity> {
    let project = app.project_id.as_ref().filter(|p| !p.trim().is_empty())?.clone();
    let host = app.apps_catalog.host.trim().to_owned();
    if host.is_empty() { return None; }
    Some(Identity { endpoint: Scope::from(&crate::app::config::gateway_client_endpoint()), project, host })
}

#[derive(Clone, Debug)]
pub enum Event {
    Load(bool),
    Refresh,
    Display(display::Event),
    Pricing(pricing::Event),
    Charts(charts::Event),
    Scan(scan::Event),
    Logs(logs::Event),
    Groups(groups::Event),
    Tick,
    Filter(overview::Change),
    Close,
    Loaded(u64, Identity, Result<UsageSummary, String>),
}

pub fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Display(event) => return display::update(app, event),
        Event::Scan(event) => return scan::update(app, event),
        Event::Pricing(event) => return pricing::update(app, event),
        Event::Charts(event) => return charts::update(app, event),
        Event::Logs(event) => return logs::update(app, event),
        Event::Groups(event) => return groups::update(app, event),
        Event::Filter(change) => {
            overview::apply(&mut app.usage_center.query, change);
            return update(app, Event::Refresh);
        }
        Event::Tick => {
            if app.usage_center.open && app.screen == crate::app::Screen::Usage {
                return update(app, Event::Refresh);
            }
        }
        Event::Close => {
            app.usage_center.close();
            app.usage_center.scan.open = false;
            return scan::activity(app, false);
        },
        Event::Loaded(ticket, scope, result) => {
            if app.usage_center.open && app.usage_center.loading
                && app.usage_center.sequence == ticket && identity(app).as_ref() == Some(&scope)
                && app.screen == crate::app::Screen::Usage {
                app.usage_center.loading = false;
                app.usage_center.result = Some(result);
            }
        }
        Event::Load(all) => {
            app.usage_center.query.range = if all { UsageRange::All } else { UsageRange::Rolling { seconds: 86400 } };
            return update(app, Event::Refresh);
        }
        Event::Refresh => {
            let query = app.usage_center.query.clone();
            let ticket = app.usage_center.begin();
            let logs_task = logs::reset(app, query.clone());
            let Some(scope) = identity(app) else {
                app.usage_center.charts.loading = false;
                app.usage_center.charts.result = Some(Err("请选择项目与执行主机".into()));
                app.usage_center.groups.loading = false;
                app.usage_center.groups.result = Some(Err("请选择项目与执行主机".into()));
                app.usage_center.loading = false;
                app.usage_center.result = Some(Err("请选择项目与执行主机".into()));
                return logs_task;
            };
            let charts_task = charts::load(ticket, scope.clone(), query.clone());
            let groups_task = groups::load(ticket, scope.clone(), query.clone(), "UTC".into());
            let client = crate::app::config::gateway_client();
            return Task::batch([display::update(app, display::Event::AutoLoad), scan::activity(app, true), logs_task, groups_task, charts_task, Task::perform(async move {
                let result = async {
                    let client = client?;
                    client.usage_summary(&UsageReadScope {
                        project_id: scope.project.clone().into(), target_host: scope.host.clone(),
                        time_zone: "UTC".into(),
                    }, &query).await.map_err(|e| e.to_string())
                }.await;
                (scope, result)
            }, move |(scope, result)| Message::View(ViewMessage::UsageCenter(Event::Loaded(ticket, scope, result))))]);
        }
    }
    Task::none()
}

pub fn view(app: &App) -> Element<'_, Message> {
    use crate::app::components::system_settings_common::rounded_action_btn_style;
    let action = |label: &'static str, event| button(text(label).size(13)).padding([6, 10])
        .style(rounded_action_btn_style).on_press(Message::View(ViewMessage::UsageCenter(event)));
    let mut content = column![row![
        action("刷新用量", Event::Refresh),
        action("扫描与修复", Event::Scan(scan::Event::Open)),
        action("币种与显示", Event::Display(display::Event::Open)),
        action("模型定价", Event::Pricing(pricing::Event::Open)),
        action("最近24小时（UTC）", Event::Load(false)),
        action("关闭用量中心", Event::Close),
    ].spacing(8)].spacing(8);
    if app.usage_center.open {
        content = content.push(overview::view(&app.usage_center.query));
        let label = if app.usage_center.loading { "正在加载用量…".into() } else {
            match &app.usage_center.result {
                Some(Ok(summary)) if summary.counts.total_requests == 0 => "暂无用量".into(),
                Some(Ok(summary)) => format!("请求 {} · 计费请求 {} · 成功 {} · 失败 {} · 已知费用 {} · 未知费用请求 {}",
                    summary.counts.total_requests, summary.counts.billable_requests, summary.succeeded_requests, summary.failed_requests, display::amount(app, &summary.known_cost_usd), summary.counts.unknown_cost_requests),
                Some(Err(error)) => format!("用量加载失败：{error}"),
                None => "请选择用量范围".into(),
            }
        };
        content = content.push(text(display::status(app)).size(13)).push(text(label).size(14)).push(charts::view(app)).push(groups::view(app)).push(logs::view(app));
    }
    display::overlay(app, pricing::overlay(app, scan::overlay(app, content.into())))
}
