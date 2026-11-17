//! 文件编辑工具的宿主类型特化。
#[cfg(test)]
pub(crate) mod types { pub use tool_files::file_edit::types::{FileDescriptor, StructuredPatch}; }
pub type FileEditTool = tool_files::file_edit::FileEditTool<super::context::AgentContextHost>;
