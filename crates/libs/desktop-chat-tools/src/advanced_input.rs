pub struct AdvancedInput<'a> {
 pub i18n: desktop_i18n::I18n,
 pub hovered: Option<u64>,
 pub text: crate::tool_text_support::ToolTextState<'a>,
}
#[derive(Debug, Clone)]
pub enum Message {
 OpenToolDetail(usize, usize, String),
 ToolHover(usize, usize),
 ToolHoverLeave,
 OpenMessageContextMenu { target: u64, x: f32, y: f32, text: String },
 Text(crate::tool_text_support::Message),
}
