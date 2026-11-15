//! 详情弹窗的只读投影；编辑历史与会话状态仍由会话域持有。
use iced::{widget::{Id, text_editor}, mouse};
#[derive(Clone, Copy)]
pub struct DialogInput<'a> {
 pub title: &'a str,
 pub editor: &'a text_editor::Content,
 pub editor_id: &'a Id,
 pub scroll_top_line: f32,
 pub context_menu_open: bool,
 pub context_menu_pos: Option<(f32, f32)>,
}
#[derive(Clone, Copy)]
pub struct DetailInput<'a> {
 pub dialog: Option<DialogInput<'a>>,
 pub i18n: desktop_i18n::I18n,
 pub line_height: f32,
}
#[derive(Debug, Clone)]
pub enum Message {
 CloseToolDetail,
 ToolDetailEditorAction(text_editor::Action),
 ToolDetailEditorWheelScrolled {delta: mouse::ScrollDelta, viewport_height: f32},
 ToolDetailScrollbarChanged {top_line: f32, viewport_height: f32},
 ToolDetailOpenContextMenu {x: f32, y: f32},
 ToolDetailUndo, ToolDetailRedo, ToolDetailCloseContextMenu,
 ToolDetailContextMenuCopy, ToolDetailContextMenuCut, ToolDetailContextMenuPaste, ToolDetailContextMenuDelete,
}
