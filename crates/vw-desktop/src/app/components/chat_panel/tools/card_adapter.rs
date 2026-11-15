use crate::app::{App, Message, message};
use desktop_chat_tools::card_input::{CardInput, Message as Local};
pub(super) fn input(app: &App) -> CardInput<'_> {
 CardInput { i18n: crate::app::i18n::I18n::new(app.desktop_locale), hovered: app.chat_tool_hovered_idx,
 expanded: &app.chat_tool_expanded, file_expanded: &app.chat_tool_file_expanded,
 project_path: app.project_path.as_deref(), text: super::super::tool_text_support::text_state(app) }
}
pub(super) fn map(local: Local<Message>) -> Message { match local {
 Local::Diff(value) => value,
 Local::OpenInlineYaml {workflow_yaml,focus_node_id} => Message::WorkflowTool(crate::apps::workflow::WorkflowMessage::OpenInlineYaml {workflow_yaml,focus_node_id}),
 Local::ToolFilesFilterChanged(v) => Message::Chat(message::ChatMessage::ToolFilesFilterChanged(v)),
 Local::ToolFileHover(v) => Message::Chat(message::ChatMessage::ToolFileHover(v)),
 Local::ToolFileHoverLeave => Message::Chat(message::ChatMessage::ToolFileHoverLeave),
 Local::ToggleExploreSummary(m,t) => Message::Chat(message::ChatMessage::ToggleExploreSummary(m,t)),
 Local::OpenToolDetail(m,t,v) => Message::Chat(message::ChatMessage::OpenToolDetail(m,t,v)),
 Local::ToolHover(m,t) => Message::Chat(message::ChatMessage::ToolHover(m,t)),
 Local::ToolHoverLeave => Message::Chat(message::ChatMessage::ToolHoverLeave),
 Local::ToggleTool(m,t) => Message::Chat(message::ChatMessage::ToggleTool(m,t)),
 Local::ToggleToolFile(m,t,p) => Message::Chat(message::ChatMessage::ToggleToolFile(m,t,p)),
 Local::OpenMessageContextMenu {target,x,y,text} => Message::Chat(message::ChatMessage::OpenMessageContextMenu {target,x,y,text}),
 Local::OpenPreview(path) => Message::Preview(message::PreviewMessage::Open(path)),
 Local::OpenChatTextDiff {title,file,before,after} => Message::Git(message::GitMessage::OpenChatTextDiff {title,file,before,after}),
 Local::Text(value) => super::super::tool_text_support::map_message(value),
} }
pub(super) fn map_card(local: Local) -> Message { match local {
 Local::Diff(never) => match never {},
 Local::OpenInlineYaml {workflow_yaml,focus_node_id} => map(Local::OpenInlineYaml {workflow_yaml,focus_node_id}),
 Local::ToolFilesFilterChanged(v) => Message::Chat(message::ChatMessage::ToolFilesFilterChanged(v)),
 Local::ToolFileHover(v) => Message::Chat(message::ChatMessage::ToolFileHover(v)),
 Local::ToolFileHoverLeave => Message::Chat(message::ChatMessage::ToolFileHoverLeave),
 Local::ToggleExploreSummary(m,t) => Message::Chat(message::ChatMessage::ToggleExploreSummary(m,t)),
 Local::OpenToolDetail(m,t,v) => map(Local::OpenToolDetail(m,t,v)),
 Local::ToolHover(m,t) => map(Local::ToolHover(m,t)),
 Local::ToolHoverLeave => map(Local::ToolHoverLeave),
 Local::ToggleTool(m,t) => map(Local::ToggleTool(m,t)),
 Local::ToggleToolFile(m,t,p) => map(Local::ToggleToolFile(m,t,p)),
 Local::OpenMessageContextMenu {target,x,y,text} => map(Local::OpenMessageContextMenu {target,x,y,text}),
 Local::OpenPreview(path) => map(Local::OpenPreview(path)),
 Local::OpenChatTextDiff {title,file,before,after} => map(Local::OpenChatTextDiff {title,file,before,after}),
 Local::Text(value) => map(Local::Text(value)),
} }
