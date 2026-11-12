use crate::app::components::system_settings_common::{
    danger_action_btn_style, rounded_action_btn_style, settings_close_button, settings_modal_card,
    settings_modal_header, settings_modal_overlay_with_escape, settings_muted_text_style,
};
use crate::app::i18n::I18n;
use crate::app::message::ProjectMessage;
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Element, Length};

pub(crate) fn with_file_tree_delete_confirm<'a>(
    app: &'a App,
    root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some((_, paths)) = &app.file_tree_pending_delete else {
        return root_content;
    };
    let i18n = I18n::new(app.desktop_locale);
    let cancel = Message::Project(ProjectMessage::FileTreeDeleteCanceled);
    let header = settings_modal_header(
        text(i18n.__("确认删除")).size(20),
        settings_close_button(cancel.clone()),
    );
    let actions = row![
        Space::new().width(Length::Fill),
        button(text(i18n.__("取消")).size(13))
            .on_press(cancel.clone())
            .padding([9, 14])
            .style(rounded_action_btn_style),
        button(text(i18n.__("删除")).size(13))
            .on_press(Message::Project(ProjectMessage::FileTreeDeleteConfirmed))
            .padding([9, 14])
            .style(danger_action_btn_style),
    ]
    .spacing(10);
    let path = column(
        paths.iter().map(|path| text(path.as_str()).wrapping(text::Wrapping::WordOrGlyph).into()),
    )
    .spacing(4);
    let path = container(
        scrollable(container(path).padding([0, 7]).width(Length::Fill))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .width(Length::Fill),
    )
    .max_height(96);
    let card = settings_modal_card(
        column![
            header,
            path,
            text(i18n.__("删除后无法恢复，请确认当前选择无误。"))
                .size(13)
                .style(settings_muted_text_style),
            actions,
        ]
        .spacing(14)
        .width(Length::Fill),
    )
    .width(Length::Fixed(480.0));

    settings_modal_overlay_with_escape(root_content, cancel, container(card).padding(12))
}
