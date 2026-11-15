//! 路径工具模块
//!
//! 本模块提供路径处理相关的辅助函数，用于安全策略中的路径解析和验证。
//! 主要功能包括：
//! - 用户主目录获取
//! - 路径波浪号（`~`）展开
//! - 路径格式识别

use std::path::PathBuf;

/// 获取当前用户的主目录路径
///
/// 通过读取环境变量 `HOME` 来获取用户主目录。
/// 在 Unix-like 系统上，这是标准的用户主目录位置。
///
/// # 返回值
///
/// - `Some(PathBuf)` - 如果 `HOME` 环境变量已设置，返回主目录路径
/// - `None` - 如果 `HOME` 环境变量未设置
///
/// # 示例
///
/// ```ignore
/// use vibe_window::app::agent::security::policy::path_utils::home_dir;
///
/// if let Some(home) = home_dir() {
///     println!("主目录: {:?}", home);
/// }
/// ```
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// 展开路径中的波浪号（`~`）为用户主目录
///
/// 将路径字符串中的 `~` 或 `~/` 前缀替换为实际的用户主目录路径。
/// 如果路径不以波浪号开头，则原样返回。
///
/// # 参数
///
/// - `path` - 待展开的路径字符串，可能包含 `~` 前缀
///
/// # 返回值
///
/// 返回展开后的 `PathBuf`：
/// - 如果 `path` 为 `"~"`，返回用户主目录
/// - 如果 `path` 以 `"~/"` 开头，返回主目录拼接剩余路径
/// - 否则，原样转换为 `PathBuf` 返回
///
/// # 示例
///
/// ```ignore
/// use vibe_window::app::agent::security::policy::path_utils::expand_user_path;
///
/// // 展开为 /home/username/Documents
/// let path = expand_user_path("~/Documents");
///
/// // 原样返回
/// let path = expand_user_path("/etc/config");
/// ```
pub fn expand_user_path(path: &str) -> PathBuf {
    // 特殊情况：路径恰好是 "~"，直接返回主目录
    if path == "~" {
        if let Some(home) = home_dir() {
            return home;
        }
    }

    // 如果路径以 "~/" 开头，将波浪号替换为主目录并拼接剩余部分
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(stripped);
        }
    }

    // 其他情况：路径不含波浪号，原样返回
    PathBuf::from(path)
}

/// 判断字符串是否看起来像文件系统路径
///
/// 通过启发式规则检测给定字符串是否可能是文件路径，而非普通字符串。
/// 这在安全策略中用于区分路径参数和普通字符串参数。
///
/// # 参数
///
/// - `candidate` - 待检测的候选字符串
///
/// # 返回值
///
/// 如果字符串符合以下任一条件，返回 `true`：
/// - 以 `/` 开头（绝对路径）
/// - 以 `./` 开头（当前目录相对路径）
/// - 以 `../` 开头（父目录相对路径）
/// - 以 `~` 开头（用户主目录路径）
/// - 等于 `.`（当前目录）
/// - 等于 `..`（父目录）
/// - 包含 `/`（路径分隔符）
///
/// 否则返回 `false`。
///
/// # 示例
///
/// ```ignore
/// use vibe_window::app::agent::security::policy::path_utils::looks_like_path;
///
/// assert!(looks_like_path("/etc/passwd"));      // 绝对路径
/// assert!(looks_like_path("./config.toml"));    // 相对路径
/// assert!(looks_like_path("~/Documents"));      // 主目录路径
/// assert!(looks_like_path("src/main.rs"));      // 包含分隔符
/// assert!(!looks_like_path("filename"));        // 普通文件名
/// assert!(!looks_like_path("some_text"));       // 普通字符串
/// ```
pub fn looks_like_path(candidate: &str) -> bool {
    candidate.starts_with('/')         // 绝对路径
        || candidate.starts_with("./") // 当前目录相对路径
        || candidate.starts_with("../") // 父目录相对路径
        || candidate.starts_with('~')  // 主目录路径
        || candidate == "."            // 当前目录
        || candidate == ".."           // 父目录
        || candidate.contains('/') // 包含路径分隔符
}

#[cfg(test)]
#[path = "path_utils_tests.rs"]
mod path_utils_tests;
