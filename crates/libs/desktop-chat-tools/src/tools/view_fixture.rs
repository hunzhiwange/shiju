//! 领域视图测试的最小存储；不构造桌面 App。
use std::collections::{HashMap, HashSet};
use iced::widget::text_editor::Content;
#[derive(Default)]
pub struct TestApp {
 pub tool_files_filter: String,
 pub chat_tool_hovered_idx: Option<u64>,
 pub project_path: Option<String>, pub desktop_locale: config_ui::DesktopLocale,
 pub chat_tool_text_editors: HashMap<u128, Content>,
 pub chat_explore_summary_animations: HashMap<u128, super::explore_input::ExploreSummaryAnimationState>,
 pub chat_explore_expanded: HashSet<u64>, pub chat_tool_file_expanded: HashSet<String>,
 pub chat_tool_file_hovered: Option<String>, pub chat_context_menu: ContextMenu,
 empty: HashMap<u64, Content>, wide: HashMap<u128, Content>,
}
#[derive(Default)]
pub struct ContextMenu { pub target: Option<u64>, pub position: Option<(f32,f32)> }
impl TestApp {
 pub fn new() -> (Self, ()) { (Self::default(), ()) }
 fn text(&self) -> crate::tool_text_support::ToolTextState<'_> {
  crate::tool_text_support::ToolTextState { chat_message_editors: &[], chat_special_text_editors: &self.empty,
   chat_tool_text_editors: &self.chat_tool_text_editors, chat_think_editors: &self.empty,
   chat_markdown_block_editors: &self.wide, markdown_selection: None, theme: iced::Theme::Light }
 }
 pub fn files(&self) -> super::files_input::FilesInput<'_> {
  super::files_input::FilesInput {card: crate::card_input::CardInput {
   i18n: desktop_i18n::I18n::new(self.desktop_locale),hovered: self.chat_tool_hovered_idx,expanded: &self.chat_explore_expanded,
   file_expanded: &self.chat_tool_file_expanded,project_path: self.project_path.as_deref(),text: self.text()
  },filter: &self.tool_files_filter,file_hovered: self.chat_tool_file_hovered.as_deref()}
 }
 pub fn explore(&self) -> super::explore_input::ExploreInput<'_> {
  super::explore_input::ExploreInput {i18n: desktop_i18n::I18n::new(self.desktop_locale),hovered: self.chat_tool_hovered_idx,
   chat_explore_expanded: &self.chat_explore_expanded,chat_explore_summary_animations: &self.chat_explore_summary_animations,
   text: self.text(),now_ms: 1,status_animation_frame: 0}
 }
}
pub fn diff<'a>(_: String, _: Option<String>, _: String, _: String) -> iced::Element<'a, ()> { iced::widget::Space::new().into() }
