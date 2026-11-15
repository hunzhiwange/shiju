//! 聊天面板通用辅助函数。
//!
//! 本模块提供状态、路径、文本、主题、时间或菜单相关的小型工具，供聊天面板视图复用。

use std::path::{Path, PathBuf};

/// 归一化 file url to path，让后续路径或文本比较保持确定性。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn normalize_file_url_to_path(s: &str) -> &str {
    s.strip_prefix("file:///").or_else(|| s.strip_prefix("file://")).unwrap_or(s)
}

/// 归一化 file reference to path，让后续路径或文本比较保持确定性。
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
pub fn normalize_file_reference_to_path(s: &str) -> Option<String> {
    let mut value = s.trim();
    if value.is_empty() {
        return None;
    }

    if value.starts_with('[')
        && value.ends_with(')')
        && let Some((_, target)) = value.rsplit_once("](")
    {
        value = target.trim_end_matches(')');
    }

    value = value.trim().trim_matches('`').trim_matches('"').trim_matches('\'').trim();

    if value.is_empty() {
        return None;
    }

    value = normalize_file_url_to_path(value).trim();

    if let Some((path, _)) = value.split_once("#L") {
        value = path;
    } else if let Some((path, _)) = value.split_once("#line-") {
        value = path;
    }

    let normalized = value.trim();
    if normalized.is_empty() { None } else { Some(normalized.to_string()) }
}

/// 处理 resolve path 对应的局部职责。
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
pub fn resolve_path(project_root: Option<&str>, p: &str) -> Option<String> {
    let normalized = normalize_file_reference_to_path(p)?;
    let p = normalized.as_str();
    if Path::new(p).is_absolute() {
        return Some(p.to_string());
    }
    if let Some(root) = project_root {
        return Some(PathBuf::from(root).join(p).to_string_lossy().to_string());
    }
    std::env::current_dir().ok().map(|cwd| cwd.join(p).to_string_lossy().to_string())
}
