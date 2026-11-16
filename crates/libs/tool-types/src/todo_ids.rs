//! Stable todo write tool identity.
pub const TODO_WRITE_TOOL_ID: &str = "TodoWrite";
pub const TODO_WRITE_TOOL_ALIAS: &str = "todowrite";
pub fn is_todo_write_tool_id(id: &str) -> bool {
    matches!(id, TODO_WRITE_TOOL_ID | TODO_WRITE_TOOL_ALIAS)
}

/// Stable read identity, including the public legacy alias.
pub fn is_todo_read_tool_id(id: &str) -> bool {
    matches!(id, "TodoRead" | "todoread")
}
