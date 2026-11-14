//! SDK-backed ledger entry; current-session details remain in the parent view.
pub mod state;
use crate::app::{App, Message, message::ViewMessage};
use iced::{Element, Task, widget::{button, column, row, text}};
use vw_gateway_client::{UsageReadScope, UsageSummary};
use vw_gateway_client::vw_api_types::usage::{UsageQuery, UsageRange, UsageFilter};
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
    Close,
    Loaded(u64, Identity, Result<UsageSummary, String>),
}

pub fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Close => app.usage_center.close(),
        Event::Loaded(ticket, scope, result) => {
            if app.usage_center.open && app.usage_center.loading
                && app.usage_center.sequence == ticket && identity(app).as_ref() == Some(&scope)
                && app.screen == crate::app::Screen::Usage {
                app.usage_center.loading = false;
                app.usage_center.result = Some(result);
            }
        }
        Event::Load(all) => {
            let ticket = app.usage_center.begin();
            let Some(scope) = identity(app) else {
                app.usage_center.loading = false;
                app.usage_center.result = Some(Err("请选择项目与执行主机".into()));
                return Task::none();
            };
            let client = crate::app::config::gateway_client();
            return Task::perform(async move {
                let result = async {
                    let client = client?;
                    let query = UsageQuery {
                        range: if all { UsageRange::All } else { UsageRange::Rolling { seconds: 86400 } },
                        filter: UsageFilter::default(), cursor: None, limit: None,
                    };
                    client.usage_summary(&UsageReadScope {
                        project_id: scope.project.clone().into(), target_host: scope.host.clone(),
                        time_zone: "UTC".into(),
                    }, &query).await.map_err(|e| e.to_string())
                }.await;
                (scope, result)
            }, move |(scope, result)| Message::View(ViewMessage::UsageCenter(Event::Loaded(ticket, scope, result))));
        }
    }
    Task::none()
}

pub fn view(app: &App) -> Element<'_, Message> {
    use crate::app::components::system_settings_common::rounded_action_btn_style;
    let action = |label: &'static str, event| button(text(label).size(13)).padding([6, 10])
        .style(rounded_action_btn_style).on_press(Message::View(ViewMessage::UsageCenter(event)));
    let mut content = column![row![
        action("全部用量 / 重试", Event::Load(true)),
        action("最近24小时（UTC）", Event::Load(false)),
        action("关闭用量中心", Event::Close),
    ].spacing(8)].spacing(8);
    if app.usage_center.open {
        let label = if app.usage_center.loading { "正在加载用量…".into() } else {
            match &app.usage_center.result {
                Some(Ok(summary)) if summary.counts.total_requests == 0 => "暂无用量".into(),
                Some(Ok(summary)) => format!("请求 {} · 已知费用 {} USD · 未知费用请求 {}",
                    summary.counts.total_requests, summary.known_cost_usd, summary.counts.unknown_cost_requests),
                Some(Err(error)) => format!("用量加载失败：{error}"),
                None => "请选择用量范围".into(),
            }
        };
        content = content.push(text(label).size(14));
    }
    content.into()
}
