pub const ASK_USER_QUESTION_TOOL_ID: &str = "AskUserQuestion";
pub const TODO_READ_TOOL_ID: &str = "TodoRead";
pub const TODO_WRITE_TOOL_ID: &str = "TodoWrite";
pub use tool_web::{WEB_FETCH_TOOL_ID, WEB_FETCH_TOOL_ALIAS};
pub use tool_web::WEB_SEARCH_TOOL_ID;
pub const BROWSER_TOOL_ID: &str = "Browser";
pub const BROWSER_OPEN_TOOL_ID: &str = "BrowserOpen";

pub const QUESTION_TOOL_ALIAS: &str = "question";
pub const TODO_READ_TOOL_ALIAS: &str = "todoread";
pub const TODO_WRITE_TOOL_ALIAS: &str = "todowrite";
pub use tool_web::WEB_SEARCH_TOOL_ALIAS;
pub const BROWSER_TOOL_ALIAS: &str = "browser";
pub const BROWSER_OPEN_TOOL_ALIAS: &str = "browser_open";
pub const ENTER_WORKTREE_TOOL_ID: &str = "enter_worktree";
pub const EXIT_WORKTREE_TOOL_ID: &str = "exit_worktree";

pub const ENTER_WORKTREE_TOOL_ALIASES: [&str; 1] = ["EnterWorktree"];
pub const EXIT_WORKTREE_TOOL_ALIASES: [&str; 1] = ["ExitWorktree"];

pub fn is_question_tool_id(id: &str) -> bool {
    matches!(id, ASK_USER_QUESTION_TOOL_ID | QUESTION_TOOL_ALIAS)
}

pub fn is_todo_read_tool_id(id: &str) -> bool {
    matches!(id, TODO_READ_TOOL_ID | TODO_READ_TOOL_ALIAS)
}

pub fn is_todo_write_tool_id(id: &str) -> bool {
    matches!(id, TODO_WRITE_TOOL_ID | TODO_WRITE_TOOL_ALIAS)
}

pub fn is_web_fetch_tool_id(id: &str) -> bool {
    matches!(id, WEB_FETCH_TOOL_ID | WEB_FETCH_TOOL_ALIAS | "webfetch")
}

pub fn is_web_search_tool_id(id: &str) -> bool {
    matches!(id, WEB_SEARCH_TOOL_ID | WEB_SEARCH_TOOL_ALIAS | "websearch" | "web_search")
}

pub fn is_browser_tool_id(id: &str) -> bool {
    matches!(id, BROWSER_TOOL_ID | BROWSER_TOOL_ALIAS)
}

pub fn is_browser_open_tool_id(id: &str) -> bool {
    matches!(id, BROWSER_OPEN_TOOL_ID | BROWSER_OPEN_TOOL_ALIAS)
}


#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
