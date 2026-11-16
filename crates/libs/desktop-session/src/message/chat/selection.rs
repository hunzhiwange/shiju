//! 会话工具上下文选择，直接复用工具领域状态。
use desktop_chat_tools::state::{SessionToolBucket, SessionToolGroup, SessionToolSelectorState};
use desktop_settings_skills::types::SkillsDirectoryScope;
pub enum SelectionMessage {
    Bucket(SessionToolBucket),
    Search(String),
    Scope(SkillsDirectoryScope),
    Collapse(SessionToolGroup),
    Group(SessionToolGroup),
    All,
    Invert,
    Tool(String),
    ManualTool(String),
    Skill(String),
    Reset,
}
pub fn update(
    selector: &mut SessionToolSelectorState,
    base_tools: &[String],
    message: SelectionMessage,
) -> Option<&'static str> {
    match message {
        SelectionMessage::Bucket(bucket) => {
            if !selector.toggle_bucket(bucket) {
                return Some("至少保留一个工具分桶");
            }
            selector.reconcile_tools(base_tools);
        }
        SelectionMessage::Search(query) => selector.set_query(query),
        SelectionMessage::Scope(scope) => selector.select_skill_directory_scope(scope),
        SelectionMessage::Collapse(group) => selector.toggle_group_collapsed(group),
        SelectionMessage::Group(group) => {
            if !selector.toggle_group_tools(base_tools, group) {
                return Some("至少保留一个会话工具");
            }
        }
        SelectionMessage::All => selector.select_all_tools(base_tools),
        SelectionMessage::Invert => {
            if !selector.invert_tools(base_tools) {
                return Some("当前工具范围无法直接反选");
            }
        }
        SelectionMessage::Tool(tool_id) => {
            if !selector.toggle_tool(base_tools, &tool_id) {
                return Some("至少保留一个会话工具");
            }
        }
        SelectionMessage::ManualTool(tool_id) => selector.toggle_manual_tool(&tool_id),
        SelectionMessage::Skill(skill_id) => selector.toggle_manual_skill(&skill_id),
        SelectionMessage::Reset => {
            selector.reset();
            selector.reconcile_tools(base_tools);
        }
    }
    None
}
