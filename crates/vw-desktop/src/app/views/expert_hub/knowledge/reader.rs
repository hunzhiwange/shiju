//! 原文只读展示：标题和操作固定，仅正文滚动；链接不产生外部动作。
use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button, settings_modal_card,
    settings_modal_overlay,
};
use crate::app::message::skill_hub::knowledge::{KnowledgeMessage, message};
use crate::app::{App, Message, i18n::I18n, skill_hub::LoadState};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, markdown, row, scrollable, text};
use iced::{Element, Font, Length};

pub(super) fn with_reader<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(reader) = &app.expert_hub.knowledge.reader else {
        return base;
    };
    let i18n = I18n::new(app.desktop_locale);
    let close = message(KnowledgeMessage::CloseReader);
    let busy = matches!(reader.status, LoadState::Loading { .. });
    // 复用公共详情的 Markdown 路径，关闭高亮并避免中文走等宽字体。
    let mut style = markdown::Style::from(&app.theme());
    style.inline_code_font = Font::default();
    style.code_block_font = Font::default();
    let body =
        markdown::view(reader.markdown_items.iter(), markdown::Settings::with_text_size(14, style))
            .map(|_| Message::None);
    let mut footer = row![
        button(text(i18n.__("关闭")))
            .padding([7, 12])
            .style(rounded_action_btn_style)
            .on_press(close.clone()),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);
    if busy {
        footer = footer.push(text(i18n.__("加载中…")));
    } else if reader.next_offset.is_some() {
        let label =
            if matches!(reader.status, LoadState::Error(_)) { "重试" } else { "继续读取" };
        footer = footer.push(
            button(text(i18n.__(label)))
                .padding([7, 12])
                .style(primary_action_btn_style)
                .on_press(message(KnowledgeMessage::ContinueRead)),
        );
    } else {
        footer = footer.push(text(i18n.__("已读完")));
    }
    if matches!(reader.status, LoadState::Error(_)) {
        footer = footer.push(text(i18n.__("加载失败，请重试")));
    }
    let title = if reader.title.is_empty() { i18n.__("原文") } else { &reader.title };
    let card = settings_modal_card(
        column![
            row![
                container(text(title).size(18)).width(Length::Fill),
                settings_close_button(close.clone())
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center),
            scrollable(container(body).padding([0, 7]).width(Length::Fill))
                .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                .height(Length::Fill),
            footer.wrap(),
        ]
        .spacing(14),
    )
    .width(Length::Fill)
    .max_width(900)
    .height(Length::Fill);
    settings_modal_overlay(
        Some(base),
        close,
        container(card).padding(20).center_x(Length::Fill).center_y(Length::Fill),
    )
}
