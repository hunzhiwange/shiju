//! 文件搜索、访问边界与精确编辑工具。
pub mod content_search;
pub mod external_directory;
pub mod file_edit;
pub mod file_read;
pub mod file_write;
pub mod glob;
pub mod glob_search;
pub mod grep;
pub mod image_asset_read;
pub mod ls;
pub mod pdf_read;

/// 宿主技能目录状态；每次检查时读取当前真实状态。
pub trait FileToolHost: tool_context::host::ContextHost {
    fn session_skill_allows_read(path: &std::path::Path) -> bool;
    fn skill_directories() -> impl std::future::Future<Output = Vec<String>> + tool_context::host::HostFuture;
}
