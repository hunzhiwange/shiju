//! FileReadTool 的宿主类型特化。
pub type FileReadTool = tool_files::file_read::FileReadTool<super::context::AgentContextHost>;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
