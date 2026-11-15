use super::Message;
use desktop_ui_controls::system_settings_common::{
    rounded_action_btn_style, settings_checkbox_style, settings_select, settings_text_input,
};
use super::options::EditorOption;
use iced::Element;
use iced::widget::{button, checkbox, column, rule, text};
use iced_code_editor::IndentStyle;

pub fn view(editor: Option<&iced_code_editor::CodeEditor>, i18n: desktop_i18n::I18n) -> Element<'_, Message> {
    let Some(editor) = editor else { return column![].into(); };
    let mut content =
        column![text(i18n.__("当前文件·本次打开有效")).size(12), text(i18n.__("显示")).size(13)]
            .spacing(8);
    for option in EditorOption::ALL {
        if matches!(option, EditorOption::Folding) {
            content = content.push(rule::horizontal(1)).push(text(i18n.__("编辑")).size(13));
        }
        content = content.push(
            checkbox(option.enabled(editor))
                .label(i18n.__(option.label()))
                .text_size(12)
                .style(settings_checkbox_style)
                .on_toggle(move |enabled| {
                    Message::SetOption(option, enabled)
                }),
        );
        if matches!(option, EditorOption::Wrap) {
            let value = editor.wrap_column().map(|n| n.to_string()).unwrap_or_default();
            let mut input = settings_text_input(i18n.__("留空随视口"), &value);
            if editor.wrap_enabled() {
                input =
                    input.on_input(|value| Message::SetWrapColumn(value));
            }
            content = content.push(text(i18n.__("换行列宽（正整数）")).size(12)).push(input);
        }
    }
    content
        .push(text(i18n.__("缩进风格")).size(12))
        .push(settings_select(IndentStyle::ALL, Some(editor.indent_style()), |style| {
            Message::SetIndentStyle(style)
        }))
        .push(
            button(text(i18n.__("完整编辑器设置")).size(12))
                .padding([6, 10])
                .style(rounded_action_btn_style)
                .on_press(Message::ToggleSettings),
        )
        .into()
}
