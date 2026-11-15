use desktop_ui_controls::system_settings_common::*;
use vw_api_mcp_management::{McpCatalogResponse, McpRecordId};
#[derive(Clone, Debug)]
pub enum Event<Choice> {
    Select(Choice),
    Start(Vec<McpRecordId>),
    Cancel,
    Refresh,
}
pub struct Panel<'a, Choice> {
    pub visible: bool,
    pub active: bool,
    pub cancelling: bool,
    pub choices: Vec<Choice>,
    pub app: Option<Choice>,
    pub catalog: Option<&'a McpCatalogResponse>,
    pub labels: Vec<(McpRecordId, &'static str)>,
    pub error: &'a Option<String>,
}
use iced::{
    Element,
    widget::{button, column, row, text},
};

pub fn view<'a, Choice: Clone + PartialEq + std::fmt::Display + 'static, M: Clone + 'a>(
    state: Panel<'a, Choice>,
    message: impl Fn(Event<Choice>) -> M + Copy + 'a,
    results: Element<'a, M>,
) -> Element<'a, M> {
    let visible = state.visible;
    let active = visible && state.active;
    let control = |label: &'static str, event: Option<Event<Choice>>| {
        button(text(label))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe(event.map(message))
    };
    let mut body = column![
        text("MCP 连通检测").size(16),
        text("仅点击才检查连接并发现工具，不执行工具；结果属于当前连接修订。")
            .size(12)
            .style(settings_muted_text_style),
        settings_select(state.choices, state.app, move |choice| message(Event::Select(choice)))
            .disabled(active),
    ]
    .spacing(8);
    if let Some(catalog) = state.catalog {
        body = body.push(control(
            "检测全部",
            (!active && !catalog.records.is_empty()).then(|| {
                Event::Start(catalog.records.iter().map(|r| r.record.id.clone()).collect())
            }),
        ));
        for record in &catalog.records {
            body = body.push(
                row![
                    text(format!(
                        "{} · {} · {}",
                        record.name,
                        record.record.id.0,
                        if visible {
                            state
                                .labels
                                .iter()
                                .find(|(id, _)| *id == record.record.id)
                                .map(|(_, label)| *label)
                                .unwrap_or("unchecked · 未检查")
                        } else {
                            "unchecked · 未检查"
                        }
                    )),
                    control(
                        "检测",
                        (!active).then(|| Event::Start(vec![record.record.id.clone()]))
                    )
                ]
                .spacing(8),
            );
        }
    }
    if visible {
        body = body.push(results);

        if active {
            body = body.push(
                row![
                    control("取消检测", (!state.cancelling).then_some(Event::Cancel)),
                    control("刷新检测", state.error.as_ref().map(|_| Event::Refresh))
                ]
                .spacing(8),
            );
        }
        if let Some(error) = state.error {
            body = body.push(settings_error_banner(error.clone()));
        }
    }
    body.into()
}
