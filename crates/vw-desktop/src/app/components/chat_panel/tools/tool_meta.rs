use crate::app::Message;
use iced::Element;
use desktop_chat_tools::tools::tool_meta as domain;
pub use domain::{ToolVisual, ToolVisualState, ToolVisualTone, tool_visual_state, tool_verb, tool_inline_summary, tool_inline_summary_unbounded, localized_tool_inline_summary, localized_tool_inline_summary_unbounded, tool_icon, tool_header_label};
pub fn tool_icon_badge<'a>(tool_name: &str, visual: ToolVisual) -> Element<'a, Message> { domain::tool_icon_badge(tool_name, visual) }
pub fn tool_header_title<'a>(
    tool_name: &str,
    // title 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    title: impl Into<String>,
    visual: ToolVisual,
) -> Element<'a, Message> { domain::tool_header_title(tool_name, title, visual) }
pub fn tool_status_badge<'a>(visual: ToolVisual) -> Element<'a, Message> { domain::tool_status_badge(visual) }
pub fn tool_activity_header<'a>(
    tool_name: &str,
    title: impl Into<String>,
    summary: impl Into<String>,
    visual: ToolVisual,
    action: Element<'a, Message>,
) -> iced::widget::Row<'a, Message> { domain::tool_activity_header(tool_name, title, summary, visual, action) }
