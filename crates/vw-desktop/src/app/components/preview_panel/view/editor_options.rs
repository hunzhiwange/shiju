use super::{App, Message};
use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_checkbox_style, settings_select,
};
use crate::app::message::editor::{EditorMessage, EditorOption};
use iced::Element;
use iced::widget::{button, checkbox, column, rule, text};
use iced_code_editor::IndentStyle;

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let Some(tab) =
        app.preview_tabs.iter().find(|tab| Some(&tab.path) == app.active_preview_path.as_ref())
    else {
        return column![].into();
    };
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let mut content =
        column![text(i18n.__("当前文件·本次打开有效")).size(12), text(i18n.__("显示")).size(13)]
            .spacing(8);
    for option in EditorOption::ALL {
        if matches!(option, EditorOption::Folding) {
            content = content.push(rule::horizontal(1)).push(text(i18n.__("编辑")).size(13));
        }
        content = content.push(
            checkbox(option.enabled(&tab.editor.inner))
                .label(i18n.__(option.label()))
                .text_size(12)
                .style(settings_checkbox_style)
                .on_toggle(move |enabled| {
                    Message::Editor(EditorMessage::SetOption(option, enabled))
                }),
        );
    }
    content
        .push(text(i18n.__("缩进风格")).size(12))
        .push(settings_select(IndentStyle::ALL, Some(tab.editor.inner.indent_style()), |style| {
            Message::Editor(EditorMessage::SetIndentStyle(style))
        }))
        .push(
            button(text(i18n.__("完整编辑器设置")).size(12))
                .padding([6, 10])
                .style(rounded_action_btn_style)
                .on_press(Message::Editor(EditorMessage::ToggleSettings)),
        )
        .into()
}
