//! Skill 审计共享的路径、扩展名和 URL 文本辅助函数。
//!
//! 这些函数只做局部、确定性的判断，供 manifest、Markdown 和扫描器复用。
//! 安全相关判断保持保守：遇到脚本后缀或 shell shebang 时标记为不支持，
//! 是否产生 finding 由调用方的显式策略决定。

use std::fs;
use std::path::Path;

/// 返回 `path` 相对 `root` 的展示字符串。
///
/// # 参数
///
/// - `root`: skill 根目录。
/// - `path`: 要展示的路径。
///
/// # 返回值
///
/// 若 `path` 在 `root` 内，返回相对路径；根目录自身显示为 `"."`。
/// 若无法剥离前缀，则退回到完整路径展示。
pub(super) fn relative_display(root: &Path, path: &Path) -> String {
    if let Ok(rel) = path.strip_prefix(root) {
        if rel.as_os_str().is_empty() {
            return ".".to_string();
        }
        return rel.display().to_string();
    }
    path.display().to_string()
}

/// 判断路径是否指向 Markdown 文件。
///
/// 支持 `.md` 与 `.markdown`，扩展名比较不区分大小写。
pub(super) fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown"))
}

/// 判断路径是否指向 TOML 文件。
///
/// 扩展名比较不区分大小写。
pub(super) fn is_toml_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
}

/// 判断路径是否是当前 skill 安全策略不支持的脚本文件。
///
/// 检查同时覆盖脚本后缀和 shebang。后者用于捕获无扩展名但可执行的
/// shell/powershell 脚本。
pub(super) fn is_unsupported_script_file(path: &Path) -> bool {
    has_script_suffix(path.to_string_lossy().as_ref()) || has_shell_shebang(path)
}

/// 判断原始路径文本是否以已知脚本后缀结尾。
pub(super) fn has_script_suffix(raw: &str) -> bool {
    let lowered = raw.to_ascii_lowercase();
    let script_suffixes = [".sh", ".bash", ".zsh", ".ksh", ".fish", ".ps1", ".bat", ".cmd"];
    script_suffixes.iter().any(|suffix| lowered.ends_with(suffix))
}

fn has_shell_shebang(path: &Path) -> bool {
    let Ok(content) = fs::read(path) else {
        return false;
    };
    // 只检查前 128 字节；读取失败由实际加载路径报告，不转化为策略 finding。
    let prefix = &content[..content.len().min(128)];
    let shebang = String::from_utf8_lossy(prefix).to_ascii_lowercase();
    shebang.starts_with("#!")
        && (shebang.contains("sh")
            || shebang.contains("bash")
            || shebang.contains("zsh")
            || shebang.contains("pwsh")
            || shebang.contains("powershell"))
}

/// 去除链接目标中的查询串和 fragment。
///
/// 返回值是 `input` 的切片，不分配新字符串。
pub(super) fn strip_query_and_fragment(input: &str) -> &str {
    let mut end = input.len();
    if let Some(idx) = input.find('#') {
        end = end.min(idx);
    }
    if let Some(idx) = input.find('?') {
        end = end.min(idx);
    }
    &input[..end]
}

/// 提取 URL scheme，并过滤明显无效的 scheme 文本。
///
/// 返回 `None` 表示输入没有合法 scheme。
pub(super) fn url_scheme(target: &str) -> Option<&str> {
    let (scheme, rest) = target.split_once(':')?;
    if scheme.is_empty() || rest.is_empty() {
        return None;
    }
    if !scheme.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')) {
        return None;
    }
    Some(scheme)
}

/// 判断链接目标是否看起来像绝对路径。
///
/// 同时识别 Unix、Windows 盘符路径和 `~/` 形式，用于审计 Markdown 中
/// 可能越过 skill 目录的本地引用。
pub(super) fn looks_like_absolute_path(target: &str) -> bool {
    let path = Path::new(target);
    if path.is_absolute() {
        return true;
    }

    let bytes = target.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return true;
    }

    target.starts_with("~/")
}

/// 判断链接目标是否带 Markdown 后缀。
pub(super) fn has_markdown_suffix(target: &str) -> bool {
    let lowered = target.to_ascii_lowercase();
    lowered.ends_with(".md") || lowered.ends_with(".markdown")
}
#[cfg(test)]
#[path = "support_tests.rs"]
mod support_tests;
