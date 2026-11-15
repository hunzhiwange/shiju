//! 渲染应用视图中的模态窗口。
//! 本模块只描述模态内容和交互消息，不直接承担持久化策略。


#[derive(Clone, Debug)]
pub enum Event { SessionRenameChanged(String), SessionRenameSave, SessionRenameCancel }

use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_close_button, settings_field_label, settings_modal_card,
    settings_modal_header, settings_modal_overlay_with_escape, settings_muted_text_style,
    settings_text_input_style,
};
use desktop_i18n::I18n;

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Space, TextInput, button, column, container, row, scrollable, text, text_input,
};
use iced::{Alignment, Element, Length};

pub fn session_rename_old_title<'a>(sessions: &'a [session_info::Info], project_sessions: &'a std::collections::HashMap<String, Vec<session_info::Info>>, session_id: &str) -> &'a str {
    sessions
        .iter()
        .find(|session| session.id.as_str() == session_id)
        .map(|session| session.title.as_str())
        .or_else(|| {
            project_sessions
                .values()
                .find_map(|sessions| {
                    sessions.iter().find(|session| session.id.as_str() == session_id)
                })
                .map(|session| session.title.as_str())
        })
        .unwrap_or("")
}

pub fn with_session_rename<'a, Message: Clone + 'a>(
    session_id: Option<&str>, value: &'a str, sessions: &'a [session_info::Info], project_sessions: &'a std::collections::HashMap<String, Vec<session_info::Info>>, locale: config_ui::DesktopLocale,
    emit: impl Fn(Event) -> Message + Copy + 'a,
    root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(session_id) = session_id else {
        return root_content;
    };
    let i18n = I18n::new(locale);
    let input = text_input(i18n.__("输入新名称"), value)
        .on_input(move |value| emit(Event::SessionRenameChanged(value)));

    rename_dialog(
        i18n,
        i18n.__("会话重命名"),
        session_rename_old_title(sessions, project_sessions, session_id),
        input,
        emit(Event::SessionRenameSave),
        emit(Event::SessionRenameCancel),
        root_content,
    )
}

fn rename_dialog<'a, Message: Clone + 'a>(
    i18n: I18n,
    title: &'a str,
    old_name: &'a str,
    input: TextInput<'a, Message>,
    save: Message,
    cancel: Message,
    root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    let input = input
        .on_submit(save.clone())
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .width(Length::Fill);
    let mut content =
        column![
            settings_modal_header(text(title).size(20), settings_close_button(cancel.clone()),)
        ]
        .spacing(14);
    if !old_name.is_empty() {
        let old_name = text(old_name)
            .size(13)
            .style(settings_muted_text_style)
            .wrapping(text::Wrapping::WordOrGlyph);
        content = content.push(
            container(
                scrollable(container(old_name).padding([0, 7]).width(Length::Fill))
                    .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
                    .width(Length::Fill),
            )
            .max_height(96),
        );
    }

    let actions = row![
        Space::new().width(Length::Fill),
        button(text(i18n.__("取消")).size(13))
            .on_press(cancel.clone())
            .padding([9, 14])
            .style(rounded_action_btn_style),
        button(text(i18n.__("确定")).size(13))
            .on_press(save)
            .padding([9, 14])
            .style(primary_action_btn_style),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let card = settings_modal_card(
        content
            .push(column![settings_field_label(i18n.__("名称")), input].spacing(6))
            .push(actions),
    )
    .width(Length::Fixed(480.0));
    settings_modal_overlay_with_escape(root_content, cancel, container(card).padding(12))
}
