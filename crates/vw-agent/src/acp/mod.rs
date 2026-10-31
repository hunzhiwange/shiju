//! VibeWindow 原生运行时的 ACP Agent 后端适配。

mod approval;
mod backend;
mod mcp;

pub use backend::VibeWindowBackend;

// 只分类已知工具身份。未知插件或标题中的 read/search 字样不能扩大自动授权。
fn tool_kind(name: &str) -> agent_client_protocol::ToolKind {
    use agent_client_protocol::ToolKind;
    match name {
        "file_read" | "read" | "pdf_read" | "image_asset_read" | "ls" => ToolKind::Read,
        "glob" | "grep" => ToolKind::Search,
        "file_write" | "write" | "file_edit" | "edit" | "apply_patch" => ToolKind::Edit,
        "bash" | "shell" | "bash_wait" | "bash_cancel" => ToolKind::Execute,
        _ => ToolKind::Other,
    }
}

#[cfg(test)]
#[path = "backend_tests.rs"]
mod backend_tests;

#[cfg(test)]
#[path = "approval_tests.rs"]
mod approval_tests;

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod mcp_tests;
