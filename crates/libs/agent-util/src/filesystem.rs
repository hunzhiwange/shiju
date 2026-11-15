//! 文件系统工具模块
//!
//! 本模块提供了一系列用于文件和目录操作的实用工具函数，主要包括：
//!
//! - 路径存在性和类型检查
//! - 路径规范化处理（跨平台支持）
//! - 路径包含关系判断
//! - 向上查找特定文件或目录
//! - 基于通配符模式向上搜索文件
//!
//! 这些工具函数简化了常见的文件系统操作，并提供了跨平台的兼容性处理。

use std::path::{Path, PathBuf};

/// 检查指定路径是否存在
///
/// 该函数尝试获取路径的元数据来判断文件或目录是否存在。
/// 如果能够成功获取元数据，则认为路径存在。
///
/// # 参数
///
/// - `p`: 要检查的路径，可以是任何实现了 `AsRef<Path>` trait 的类型
///
/// # 返回值
///
/// 如果路径存在返回 `true`，否则返回 `false`
///
/// # 示例
///
/// ```ignore
/// use crate::filesystem::exists;
///
/// if exists("/path/to/file") {
///     println!("文件存在");
/// }
/// ```
pub fn exists(p: impl AsRef<Path>) -> bool {
    std::fs::metadata(p).is_ok()
}

/// 检查指定路径是否为目录
///
/// 该函数获取路径的元数据并检查其是否为目录类型。
/// 如果获取元数据失败或路径不是目录，则返回 `false`。
///
/// # 参数
///
/// - `p`: 要检查的路径，可以是任何实现了 `AsRef<Path>` trait 的类型
///
/// # 返回值
///
/// 如果路径存在且为目录返回 `true`，否则返回 `false`
///
/// # 示例
///
/// ```ignore
/// use crate::filesystem::is_dir;
///
/// if is_dir("/path/to/directory") {
///     println!("这是一个目录");
/// }
/// ```
pub fn is_dir(p: impl AsRef<Path>) -> bool {
    std::fs::metadata(p).map(|m| m.is_dir()).unwrap_or(false)
}

/// 规范化路径
///
/// 根据操作系统对路径进行规范化处理：
/// - 在 Windows 系统上，使用 `canonicalize` 解析符号链接并获取绝对路径
/// - 在其他系统上，直接返回路径的 `PathBuf` 表示
///
/// # 参数
///
/// - `p`: 要规范化的路径，可以是任何实现了 `AsRef<Path>` trait 的类型
///
/// # 返回值
///
/// 规范化后的 `PathBuf`
///
/// # 注意
///
/// 在 Windows 上，如果 `canonicalize` 失败（例如路径不存在），
/// 则返回原始路径的 `PathBuf` 表示。
///
/// # 示例
///
/// ```ignore
/// use crate::filesystem::normalize_path;
///
/// let normalized = normalize_path("./some/path");
/// ```
pub fn normalize_path(p: impl AsRef<Path>) -> PathBuf {
    let p = p.as_ref();
    if cfg!(windows) {
        std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
    } else {
        p.to_path_buf()
    }
}

/// 检查子路径是否包含在父路径下
///
/// 判断 `child` 路径是否位于 `parent` 路径的目录树中。
/// 首先尝试规范化两个路径后进行比较，如果规范化失败则直接比较原始路径。
///
/// # 参数
///
/// - `parent`: 父路径，可以是任何实现了 `AsRef<Path>` trait 的类型
/// - `child`: 子路径，可以是任何实现了 `AsRef<Path>` trait 的类型
///
/// # 返回值
///
/// 如果 `child` 在 `parent` 目录树下返回 `true`，否则返回 `false`
///
/// # 示例
///
/// ```ignore
/// use crate::filesystem::contains;
///
/// // 返回 true
/// assert!(contains("/home/user", "/home/user/documents"));
///
/// // 返回 false
/// assert!(!contains("/home/user", "/etc/config"));
/// ```
pub fn contains(parent: impl AsRef<Path>, child: impl AsRef<Path>) -> bool {
    let parent = parent.as_ref();
    let child = child.as_ref();
    if let (Ok(p), Ok(c)) = (std::fs::canonicalize(parent), std::fs::canonicalize(child)) {
        return c.starts_with(&p);
    }
    child.starts_with(parent)
}

/// 检查两个路径是否存在重叠（包含关系）
///
/// 判断两个路径中是否有任何一个包含另一个。
/// 即判断 `a` 是否包含 `b`，或者 `b` 是否包含 `a`。
///
/// # 参数
///
/// - `a`: 第一个路径，可以是任何实现了 `AsRef<Path>` trait 的类型
/// - `b`: 第二个路径，可以是任何实现了 `AsRef<Path>` trait 的类型
///
/// # 返回值
///
/// 如果两个路径存在重叠（一个包含另一个）返回 `true`，否则返回 `false`
///
/// # 示例
///
/// ```ignore
/// use crate::filesystem::overlaps;
///
/// // 返回 true，因为 /home/user 包含 /home/user/docs
/// assert!(overlaps("/home/user", "/home/user/docs"));
///
/// // 返回 false，因为两个路径互不包含
/// assert!(!overlaps("/home/alice", "/home/bob"));
/// ```
pub fn overlaps(a: impl AsRef<Path>, b: impl AsRef<Path>) -> bool {
    contains(a.as_ref(), b.as_ref()) || contains(b.as_ref(), a.as_ref())
}

/// 从起始目录向上查找指定的目标文件或目录
///
/// 从 `start` 目录开始，逐级向上搜索名为 `target` 的文件或目录，
/// 直到到达 `stop` 目录或文件系统根目录为止。
///
/// # 参数
///
/// - `target`: 要查找的目标文件或目录名称
/// - `start`: 搜索的起始目录
/// - `stop`: 可选的停止目录，到达此目录时停止搜索（不包含此目录的检查）
///
/// # 返回值
///
/// 找到的所有匹配路径的向量，按从近到远的顺序排列
///
/// # 异步
///
/// 此函数为异步函数，适合在异步上下文中使用
///
/// # 示例
///
/// ```ignore
/// use crate::filesystem::find_up;
///
/// // 从当前目录向上查找所有名为 "Cargo.toml" 的文件
/// let cargo_files = find_up("Cargo.toml", ".", None).await;
///
/// // 查找到 /home/user/project/Cargo.toml 时停止
/// let result = find_up("config.json", "/home/user/project/src", Some("/home/user")).await;
/// ```
pub async fn find_up<P: AsRef<Path>, S: AsRef<Path>>(
    target: &str,
    start: P,
    stop: Option<S>,
) -> Vec<PathBuf> {
    let mut current = start.as_ref().to_path_buf();
    let stop = stop.map(|p| p.as_ref().to_path_buf());
    let mut result = Vec::new();

    loop {
        let search = current.join(target);
        if exists(&search) {
            result.push(search);
        }
        if stop.as_ref().is_some_and(|s| *s == current) {
            break;
        }
        let Some(parent) = current.parent().map(|p| p.to_path_buf()) else { break };
        if parent == current {
            break;
        }
        current = parent;
    }
    result
}

/// 从起始目录向上查找多个目标文件或目录
///
/// 从 `start` 目录开始，逐级向上搜索 `targets` 中指定的所有文件或目录，
/// 直到到达 `stop` 目录或文件系统根目录为止。
///
/// # 参数
///
/// - `targets`: 要查找的目标文件或目录名称列表
/// - `start`: 搜索的起始目录
/// - `stop`: 可选的停止目录，到达此目录时停止搜索
///
/// # 返回值
///
/// 找到的所有匹配路径的向量
///
/// # 示例
///
/// ```ignore
/// use crate::filesystem::up;
///
/// // 从当前目录向上查找配置文件
/// let configs = up(&["config.json", "config.yaml", ".env"], ".", None);
/// ```
pub fn up<P: AsRef<Path>, S: AsRef<Path>>(
    targets: &[&str],
    start: P,
    stop: Option<S>,
) -> Vec<PathBuf> {
    let mut current = start.as_ref().to_path_buf();
    let stop = stop.map(|p| p.as_ref().to_path_buf());
    let mut result = Vec::new();

    loop {
        for target in targets {
            let search = current.join(target);
            if exists(&search) {
                result.push(search);
            }
        }
        if stop.as_ref().is_some_and(|s| *s == current) {
            break;
        }
        let Some(parent) = current.parent().map(|p| p.to_path_buf()) else { break };
        if parent == current {
            break;
        }
        current = parent;
    }
    result
}

/// 从起始目录向上使用通配符模式搜索文件
///
/// 从 `start` 目录开始，逐级向上使用 glob 通配符模式搜索匹配的文件，
/// 直到到达 `stop` 目录或文件系统根目录为止。
///
/// # 参数
///
/// - `pattern`: glob 通配符模式（如 `"*.toml"`、`"config.*"` 等）
/// - `start`: 搜索的起始目录
/// - `stop`: 可选的停止目录，到达此目录时停止搜索
///
/// # 返回值
///
/// 找到的所有匹配文件的路径向量（仅包含文件，不包含目录）
///
/// # 示例
///
/// ```ignore
/// use crate::filesystem::glob_up;
///
/// // 从当前目录向上查找所有 .toml 配置文件
/// let toml_files = glob_up("*.toml", ".", None);
///
/// // 查找所有以 "config" 开头的文件
/// let config_files = glob_up("config.*", "/home/user/project/src", Some("/home/user"));
/// ```
pub fn glob_up<P: AsRef<Path>, S: AsRef<Path>>(
    pattern: &str,
    start: P,
    stop: Option<S>,
) -> Vec<PathBuf> {
    let mut current = start.as_ref().to_path_buf();
    let stop = stop.map(|p| p.as_ref().to_path_buf());
    let mut result = Vec::new();

    loop {
        let pat = current.join(pattern).to_string_lossy().to_string();
        if let Ok(paths) = glob::glob(&pat) {
            for p in paths.flatten() {
                if p.is_file() {
                    result.push(p);
                }
            }
        }
        if stop.as_ref().is_some_and(|s| *s == current) {
            break;
        }
        let Some(parent) = current.parent().map(|p| p.to_path_buf()) else { break };
        if parent == current {
            break;
        }
        current = parent;
    }
    result
}
