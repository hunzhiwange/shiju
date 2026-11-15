//! Public editor actions; events use the existing preview save and undo pipeline.
use super::Message;
use desktop_ui_controls::system_settings_common::rounded_action_btn_style;

use iced::widget::{Space, button, column, row, rule, text};
use iced::{Element, Length};
use iced_code_editor::Message as Event;

pub fn actions(read_only: bool, vim: bool, folding: bool) -> Vec<Event> {
    let mut items = vec![Event::SelectAll];
    if !vim {
        items.extend([Event::SelectNextOccurrence, Event::AddCursorAbove, Event::AddCursorBelow]);
    }
    items.extend([Event::OpenGotoLine, Event::OpenCommandPalette]);
    if !read_only {
        items.extend([
            Event::MoveLineUp,
            Event::MoveLineDown,
            Event::DuplicateLineUp,
            Event::DuplicateLineDown,
            Event::ToggleComment,
        ]);
    }
    if folding {
        items.extend([Event::ToggleFoldAtCursor, Event::FoldAll, Event::UnfoldAll]);
    }
    items
}

pub fn view(editor: Option<&iced_code_editor::CodeEditor>, i18n: desktop_i18n::I18n, read_only: bool) -> Element<'_, Message> {
    let Some(editor) = editor else { return column![].into(); };
    let command = if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" };
    let mut content = column![].spacing(2);
    let mut previous = 0;
    for event in
        actions(read_only, editor.vim_enabled(), editor.folding_enabled())
    {
        let (group, label, shortcut) = match &event {
            Event::SelectAll => (1, "全选", format!("{command}+A")),
            Event::SelectNextOccurrence => (1, "选择下一个相同项", format!("{command}+D")),
            Event::AddCursorAbove => (1, "向上添加光标", "Ctrl+Alt+↑".into()),
            Event::AddCursorBelow => (1, "向下添加光标", "Ctrl+Alt+↓".into()),
            Event::OpenGotoLine => (2, "跳转到行", format!("{command}+G")),
            Event::OpenCommandPalette => (2, "命令面板", format!("{command}+Shift+P")),
            Event::MoveLineUp => (3, "向上移动行", "Alt+↑".into()),
            Event::MoveLineDown => (3, "向下移动行", "Alt+↓".into()),
            Event::DuplicateLineUp => (3, "向上复制行", "Alt+Shift+↑".into()),
            Event::DuplicateLineDown => (3, "向下复制行", "Alt+Shift+↓".into()),
            Event::ToggleComment => (3, "切换行注释", format!("{command}+/")),
            Event::ToggleFoldAtCursor => (4, "切换折叠", "Ctrl+.".into()),
            Event::FoldAll => (4, "全部折叠", "Ctrl+K".into()),
            Event::UnfoldAll => (4, "全部展开", "Ctrl+J".into()),
            _ => unreachable!("actions only contains supported menu events"),
        };
        if previous != 0 && previous != group {
            content = content.push(rule::horizontal(1));
        }
        previous = group;
        content = content.push(
            button(
                row![
                    text(i18n.__(label)).size(12),
                    Space::new().width(Length::Fill),
                    text(shortcut).size(11)
                ]
                .spacing(12)
                .align_y(iced::Alignment::Center),
            )
            .padding([6, 10])
            .width(Length::Fill)
            .style(rounded_action_btn_style)
            .on_press(Message::EditorEvent(event)),
        );
    }
    content.into()
}
