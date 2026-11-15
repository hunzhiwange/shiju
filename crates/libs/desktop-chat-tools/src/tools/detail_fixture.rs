#![allow(dead_code)]
use iced::widget::{Id, text_editor};
pub struct ToolDetailDialog {
    pub msg_idx: usize,
    pub tool_idx: usize,
    pub title: String,
    pub content: String,
    pub editor: text_editor::Content,
    pub editor_id: Id,
    pub undo_stack: Vec<String>,
    pub redo_stack: Vec<String>,
    pub context_menu_open: bool,
    pub context_menu_pos: Option<(f32, f32)>,
    pub scroll_top_line: f32,
    pub scroll_remainder: f32,
    pub viewport_height: f32,
}
#[derive(Default)]
pub struct TestApp {pub tool_detail_dialog: Option<ToolDetailDialog>, pub current_line_height: f32}
impl TestApp {
 pub fn new() -> (Self, ()) { (Self::default(), ()) }
 pub fn input(&self) -> super::detail_input::DetailInput<'_> {
  super::detail_input::DetailInput {
   i18n: desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese), line_height: self.current_line_height,
   dialog: self.tool_detail_dialog.as_ref().map(|d| super::detail_input::DialogInput {title: &d.title, editor: &d.editor, editor_id: &d.editor_id, scroll_top_line: d.scroll_top_line, context_menu_open: d.context_menu_open, context_menu_pos: d.context_menu_pos})
  }
 }
}
