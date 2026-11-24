use desktop_ui_controls::system_settings_common::*;
type Message = Event;
use vw_api_app_failover::*;
use vw_api_transport::ProviderId;
#[derive(Clone, Debug)]
pub enum Operation {
    Refresh,
    Add(ProviderId),
    Remove(ProviderId),
    Move(usize, usize),
    Enabled(bool),
}
#[derive(Clone, Debug)]
pub enum Event {
    Close,
    Run(Operation),
}
pub struct State<'a> {
    pub busy: bool,
    pub error: &'a Option<String>,
    pub queue: &'a Option<FailoverQueueResponse>,
    pub candidates: &'a [FailoverQueueEntry],
}
use iced::{
    Element, Length,
    widget::{button, column, container, row, text},
};

fn action(i18n: desktop_i18n::I18n, label: &'static str, event: Event, enabled: bool) -> Element<'static, Message> {
    button(text(i18n.__(label)))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then(|| event))
        .into()
}
pub fn overlay<'a>(i18n: desktop_i18n::I18n, state: State<'a>, base: Element<'a, Message>) -> Element<'a, Message> {
    let close = Event::Close;
    let header = settings_modal_header(
        text(i18n.__("备用队列")).size(20),
        Some(settings_close_button(close.clone())),
    );
    let mut body = column![
        text(i18n.__("队列资格与顺序需经验证。官方账号无法加入备用队列。")),
        action(i18n, i18n.__("刷新"), Event::Run(Operation::Refresh), !state.busy),
    ]
    .spacing(12);
    if state.busy {
        body = body.push(text(i18n.__("正在等待确认…")));
    }
    if let Some(error) = state.error {
        body = body.push(settings_error_banner(error));
    }
    if let Some(snapshot) = state.queue {
        body = body.push(action(i18n,
            if snapshot.enabled { i18n.__("停用故障转移") } else { i18n.__("启用故障转移") },
            Event::Run(Operation::Enabled(!snapshot.enabled)),
            !state.busy,
        ));
        if snapshot.entries.is_empty() {
            body = body.push(text(i18n.__("备用队列为空")));
        }
        for (index, entry) in snapshot.entries.iter().enumerate() {
            body = body.push(settings_panel(
                column![
                    text(format!("{}. {}", index + 1, entry.name)),
                    row![
                        action(i18n,
                            i18n.__("上移"),
                            Event::Run(Operation::Move(index, index.saturating_sub(1))),
                            !state.busy && index > 0
                        ),
                        action(i18n,
                            i18n.__("下移"),
                            Event::Run(Operation::Move(index, index + 1)),
                            !state.busy && index + 1 < snapshot.entries.len()
                        ),
                        action(i18n,
                            i18n.__("移除"),
                            Event::Run(Operation::Remove(entry.provider_id.clone())),
                            !state.busy
                        ),
                    ]
                    .spacing(8)
                    .wrap(),
                ]
                .spacing(8),
            ));
        }
        body = body.push(text(i18n.__("可加入的供应商")).size(16));
        for entry in state
            .candidates
            .iter()
            .filter(|c| !snapshot.entries.iter().any(|e| e.provider_id == c.provider_id))
        {
            body = body.push(
                row![
                    text(&entry.name),
                    action(i18n,
                        i18n.__("加入"),
                        Event::Run(Operation::Add(entry.provider_id.clone())),
                        !state.busy
                    )
                ]
                .spacing(8),
            );
        }
    }
    let footer = row![action(i18n, i18n.__("关闭"), Event::Close, true)];
    let card = settings_modal_layout(header, body, Some(footer.into())).width(760).height(640);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
