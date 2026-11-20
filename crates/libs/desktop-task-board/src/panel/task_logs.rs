//! 任务详情的只读执行日志。
use crate::message::TaskBoardMessage;
use desktop_ui_controls::text_editor_context_menu::{
    TextEditorContextMenuMessages, TextEditorContextMenuState, wrap_with_context_menu,
};
use desktop_ui_controls::text_editor_scroll_panel::{
    TextEditorScrollPanelMetrics, text_editor_scroll_panel,
};
use iced::widget::{container, responsive, text_editor};
use iced::{Background, Border, Color, Element, Length, Size, Theme};

use super::{PanelView, PanelMessage as Message};

pub fn build_task_logs<'a>(app: PanelView<'a>) -> Element<'a, Message> {
    let panel = responsive(move |size| build_task_logs_panel(app, size)).height(Length::Fill);

    container(panel).width(Length::Fill).height(Length::Fill).into()
}

fn build_task_logs_panel<'a>(app: PanelView<'a>, size: Size) -> Element<'a, Message> {
    let editor = text_editor(app.task_board_logs_editor)
        .id((*app.task_board_logs_editor_id).clone())
        .on_action(|action| Message::TaskBoard(TaskBoardMessage::LogsViewerEditorAction(action)))
        .size(11)
        .padding(0)
        .height(Length::Shrink)
        .style(|theme: &Theme, _status| {
            let p = theme.extended_palette();
            text_editor::Style {
                background: Background::Color(Color::TRANSPARENT),
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
                value: theme.palette().text,
                selection: theme.palette().primary.scale_alpha(0.30),
                placeholder: p.secondary.strong.color.scale_alpha(0.55),
            }
        });

    let editor = wrap_with_context_menu(
        desktop_i18n::I18n::new(app.desktop_locale),
        editor,
        TextEditorContextMenuState {
            open: app.task_board_logs_context_menu_open,
            position: app.task_board_logs_context_menu_pos,
        },
        |point| {
            Message::TaskBoard(TaskBoardMessage::LogsViewerOpenContextMenu {
                x: point.x,
                y: point.y,
            })
        },
        TextEditorContextMenuMessages {
            undo: None,
            redo: None,
            close: Message::TaskBoard(TaskBoardMessage::LogsViewerCloseContextMenu),
            copy: Message::TaskBoard(TaskBoardMessage::LogsViewerContextMenuCopy),
            cut: Message::TaskBoard(TaskBoardMessage::LogsViewerContextMenuCut),
            paste: Message::TaskBoard(TaskBoardMessage::LogsViewerContextMenuPaste),
            delete: Message::TaskBoard(TaskBoardMessage::LogsViewerContextMenuDelete),
        },
    );

    text_editor_scroll_panel(
        editor,
        size,
        TextEditorScrollPanelMetrics {
            viewport_padding: 24.0,
            line_height: app.current_line_height,
            line_count: app.task_board_logs_editor.line_count(),
            scroll_top_line: app.task_board_logs_scroll_top_line,
        },
        |delta, viewport_height| {
            Message::TaskBoard(TaskBoardMessage::LogsViewerEditorWheelScrolled {
                delta,
                viewport_height,
            })
        },
        |top_line, viewport_height| {
            Message::TaskBoard(TaskBoardMessage::LogsViewerScrollbarChanged {
                top_line,
                viewport_height,
            })
        },
    )
}

#[cfg(test)]
#[path = "task_logs_tests.rs"]
mod task_logs_tests;
