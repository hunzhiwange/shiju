//! 文件遍历忽略规则。
//!
//! 本模块集中维护代码搜索和文件列表使用的默认忽略集合，包括常见依赖目录、
//! 构建产物、缓存和临时文件。规则默认保守地排除高噪声路径，调用方可通过额外
//! glob 和白名单做局部调整。

use glob::Pattern;
use std::path::MAIN_SEPARATOR;
use std::sync::LazyLock;

#[cfg(test)]
#[path = "ignore_tests.rs"]
mod ignore_tests;

static FOLDERS: LazyLock<std::collections::HashSet<&'static str>> = LazyLock::new(|| {
    [
        "node_modules",
        "bower_components",
        ".pnpm-store",
        "vendor",
        ".npm",
        "dist",
        "build",
        "out",
        ".next",
        "target",
        "bin",
        "obj",
        ".git",
        ".svn",
        ".hg",
        ".vscode",
        ".idea",
        ".turbo",
        ".output",
        "desktop",
        ".sst",
        ".cache",
        ".webkit-cache",
        "__pycache__",
        ".pytest_cache",
        "mypy_cache",
        ".history",
        ".gradle",
    ]
    .into_iter()
    .collect()
});

static FILES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec![
        "**/*.swp",
        "**/*.swo",
        "**/*.pyc",
        "**/.DS_Store",
        "**/Thumbs.db",
        "**/logs/**",
        "**/tmp/**",
        "**/temp/**",
        "**/*.log",
        "**/coverage/**",
        "**/.nyc_output/**",
    ]
});

static FILE_GLOBS: LazyLock<Vec<Pattern>> =
    LazyLock::new(|| FILES.iter().filter_map(|p| Pattern::new(p).ok()).collect());

pub static PATTERNS: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut out = Vec::new();
    out.extend(FILES.iter().map(|s| s.to_string()));
    out.extend(FOLDERS.iter().map(|s| s.to_string()));
    out
});

/// 按平台分隔符和常见跨平台分隔符拆分路径。
///
/// 参数：
/// - `path`：待拆分的路径文本。
///
/// 返回值：
/// 返回路径片段迭代器。额外支持 `/` 与 `\`，是为了让测试、Windows 路径和
/// 归一化后的相对路径共用同一套匹配逻辑。
fn split_path(path: &str) -> impl Iterator<Item = &str> {
    path.split(MAIN_SEPARATOR).flat_map(|p| p.split('/')).flat_map(|p| p.split('\\'))
}

/// 判断文件路径是否命中忽略规则。
///
/// 参数：
/// - `filepath`：待检查的相对或规范化路径。
/// - `extra`：调用方附加的忽略 glob。
/// - `whitelist`：优先级最高的白名单 glob，命中后强制不忽略。
///
/// 返回值：
/// 命中忽略规则时返回 `true`，否则返回 `false`。
///
/// 安全说明：
/// 白名单先于默认规则执行，调用方应只传入明确可信的白名单；默认规则不会扩大
/// 文件访问权限，只影响遍历结果是否展示。
pub fn matches(filepath: &str, extra: Option<&[Pattern]>, whitelist: Option<&[Pattern]>) -> bool {
    if let Some(list) = whitelist {
        for glob in list {
            if glob.matches(filepath) {
                return false;
            }
        }
    }

    for part in split_path(filepath) {
        if FOLDERS.contains(part) {
            return true;
        }
    }

    if let Some(extra) = extra {
        for glob in extra {
            if glob.matches(filepath) {
                return true;
            }
        }
    }

    for glob in FILE_GLOBS.iter() {
        if glob.matches(filepath) {
            return true;
        }
    }

    false
}
