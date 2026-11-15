//! 工具卡片实际使用的只读输入与交互。
use std::collections::HashSet;
#[derive(Clone)]
pub struct CardInput<'a> {
    pub i18n: desktop_i18n::I18n,
    pub hovered: Option<u64>,
    pub expanded: &'a HashSet<u64>,
    pub file_expanded: &'a HashSet<String>,
    pub project_path: Option<&'a str>,
    pub text: crate::tool_text_support::ToolTextState<'a>,
}
#[derive(Debug, Clone)]
pub enum Message<Host = std::convert::Infallible> {
    OpenToolDetail(usize, usize, String),
    ToolHover(usize, usize),
    ToolHoverLeave,
    ToolFilesFilterChanged(String),
    ToolFileHover(String),
    ToolFileHoverLeave,
    ToggleExploreSummary(usize, usize),
    ToggleTool(usize, usize),
    ToggleToolFile(usize, usize, String),
    OpenMessageContextMenu { target: u64, x: f32, y: f32, text: String },
    OpenPreview(String),
    OpenChatTextDiff { title: String, file: String, before: String, after: String },
    Text(crate::tool_text_support::Message),
    Diff(Host),
}
