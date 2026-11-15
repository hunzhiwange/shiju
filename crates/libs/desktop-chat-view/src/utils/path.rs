//! 聊天路径兼容入口。

use std::path::Path;


pub use desktop_chat_tools::path::{normalize_file_url_to_path, normalize_file_reference_to_path};

pub fn resolve_path(project_path: Option<&str>, p: &str) -> Option<String> {
    desktop_chat_tools::path::resolve_path(project_path, p)
}

/// 生成 to project root，用于界面中显示更短的相对信息。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `None` 表示输入缺少必要字段、当前状态不匹配，或该视图片段不需要展示。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn relative_to_project_root(project_path: Option<&str>, abs: &str) -> Option<String> {
    let root = project_path?;
    let abs_path = Path::new(abs);
    let root_path = Path::new(root);
    let rel = abs_path.strip_prefix(root_path).ok()?;
    let s = rel.to_string_lossy().to_string();
    Some(s.trim_start_matches('/').to_string())
}
