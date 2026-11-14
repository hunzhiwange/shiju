//! One remembered query drives every ledger panel; calendar bounds belong to the service.
use vw_gateway_client::vw_api_types::usage::{UsageFilter, UsageQuery, UsageRange, UsageCalendarPeriod, UsageSourceKind, UsageProvider};
use crate::app::{Message, message::ViewMessage};
use iced::{Element, widget::{button, column, row, text, text_input}};
use super::Event;

pub fn default_query() -> UsageQuery {
    UsageQuery { range: UsageRange::All, filter: UsageFilter::default(), cursor: None, limit: None }
}

#[derive(Clone, Debug)]
pub enum Change {
    App(String), Provider(String), Model(String), Source(Option<UsageSourceKind>), Range(UsageRange),
}

pub fn apply(query: &mut UsageQuery, change: Change) {
    let optional = |value: String| if value.trim().is_empty() { None } else { Some(value.trim().to_owned()) };
    match change {
        Change::App(value) => query.filter.app = optional(value),
        Change::Provider(value) => query.filter.provider = optional(value).map(|provider_id| UsageProvider::Known { provider_id }),
        Change::Model(value) => query.filter.request_model = optional(value),
        Change::Source(value) => query.filter.source_kind = value,
        Change::Range(value) => query.range = value,
    }
    query.cursor = None;
}

pub fn view(query: &UsageQuery) -> Element<'_, Message> {
    use crate::app::components::system_settings_common::{settings_text_input_style, SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, rounded_action_btn_style};
    let send = |change| Message::View(ViewMessage::UsageCenter(Event::Filter(change)));
    let action = |label: &'static str, change| button(text(label).size(13)).padding([6, 10]).style(rounded_action_btn_style).on_press(send(change));
    let provider = match &query.filter.provider { Some(UsageProvider::Known { provider_id }) => provider_id.as_str(), _ => "" };
    let filters = row![
        text_input("应用 ID（空为全部）", query.filter.app.as_deref().unwrap_or("")).style(settings_text_input_style).padding(SETTINGS_CONTROL_PADDING).size(SETTINGS_CONTROL_TEXT_SIZE).on_input(move |v| send(Change::App(v))),
        text_input("供应商 ID（空为全部）", provider).style(settings_text_input_style).padding(SETTINGS_CONTROL_PADDING).size(SETTINGS_CONTROL_TEXT_SIZE).on_input(move |v| send(Change::Provider(v))),
        text_input("请求模型（空为全部）", query.filter.request_model.as_deref().unwrap_or("")).style(settings_text_input_style).padding(SETTINGS_CONTROL_PADDING).size(SETTINGS_CONTROL_TEXT_SIZE).on_input(move |v| send(Change::Model(v))),
    ].spacing(8);
    let calendar = |period| Change::Range(UsageRange::Calendar { period, time_zone: "UTC".into() });
    column![filters, row![
        action("全部来源", Change::Source(None)), action("原生日志", Change::Source(Some(UsageSourceKind::Native))),
        action("路由", Change::Source(Some(UsageSourceKind::Relay))), action("识矩", Change::Source(Some(UsageSourceKind::Shiju))),
    ].spacing(8), row![
        action("全部时间", Change::Range(UsageRange::All)), action("今天（UTC）", calendar(UsageCalendarPeriod::Today)),
        action("最近7天", calendar(UsageCalendarPeriod::Last7Days)), action("最近30天", calendar(UsageCalendarPeriod::Last30Days)),
        action("滚动24小时", Change::Range(UsageRange::Rolling { seconds: 86400 })),
    ].spacing(8), text(format!("当前范围：{:?} · 来源：{:?}", query.range, query.filter.source_kind)).size(12)].spacing(8).into()
}
