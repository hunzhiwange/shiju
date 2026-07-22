//! 聊天面板通用辅助函数。
//!
//! 本模块提供状态、路径、文本、主题、时间或菜单相关的小型工具，供聊天面板视图复用。

use once_cell::sync::Lazy;
/// 重新导出 use std::borrow::Cow，让上层模块通过稳定路径访问。
use std::borrow::Cow;
/// 重新导出 use std::collections::HashMap，让上层模块通过稳定路径访问。
use std::collections::HashMap;
/// 重新导出 use std::hash::{Hash, Hasher}，让上层模块通过稳定路径访问。
use std::hash::{Hash, Hasher};
/// 重新导出 use std::sync::Mutex，让上层模块通过稳定路径访问。
use std::sync::Mutex;

/// 重新导出 use super::super::tool_names::{is_compact_tool_call_trace, is_known_tool_name}，让上层模块通过稳定路径访问。
use super::super::tool_names::{is_compact_tool_call_trace, is_known_tool_name};

/// 处理 truncate chars 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回字符串已经按界面展示或比较需求做过必要整理。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i >= max_chars {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

/// 处理 truncate lines middle 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回字符串已经按界面展示或比较需求做过必要整理。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn truncate_lines_middle(s: &str, max_lines: usize, per_line_max_chars: usize) -> String {
    let mut lines: Vec<String> = s.lines().map(|l| truncate_chars(l, per_line_max_chars)).collect();
    if lines.len() <= max_lines {
        return lines.join("\n");
    }
    let head = max_lines / 2;
    let tail = max_lines - head;
    let mut out = Vec::with_capacity(max_lines + 1);
    out.extend(lines.drain(..head));
    out.push("…".to_string());
    out.extend(lines.drain(lines.len().saturating_sub(tail)..));
    out.join("\n")
}

/// 归一化 display text，让后续路径或文本比较保持确定性。
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
pub fn normalize_display_text<'a>(s: &'a str) -> Cow<'a, str> {
    if s.is_empty()
        || (!s.contains('\r')
            && !s.contains("\n\n\n")
            && !s.contains(" \n")
            && !s.contains("\t\n")
            && !s.contains("\n \n")
            && !s.contains("\n\t\n"))
    {
        return Cow::Borrowed(s);
    }

    let mut out = String::with_capacity(s.len());
    let mut prev_blank = false;
    let mut in_fence = false;
    for raw_line in s.split('\n') {
        let line = raw_line.trim_end_matches('\r');
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        }
        if in_fence {
            out.push_str(line);
            out.push('\n');
            prev_blank = false;
            continue;
        }

        let line = line.trim_end_matches([' ', '\t']);
        if line.is_empty() {
            if prev_blank {
                continue;
            }
            out.push('\n');
            prev_blank = true;
        } else {
            out.push_str(line);
            out.push('\n');
            prev_blank = false;
        }
    }
    if out.ends_with('\n') {
        out.pop();
    }
    Cow::Owned(out)
}

/// 处理 fold fullwidth ascii 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回字符串已经按界面展示或比较需求做过必要整理。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
fn fold_fullwidth_ascii(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ('\u{FF01}'..='\u{FF5E}').contains(&ch) {
                // char 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                char::from_u32((ch as u32).saturating_sub(0xFEE0)).unwrap_or(ch)
            } else if ch == '\u{3000}' {
                ' '
            } else {
                ch
            }
        })
        .collect::<String>()
}

/// 处理 is tool trace hint line 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `true` 表示当前输入满足该辅助函数描述的条件。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
fn is_tool_trace_hint_line(line: &str) -> bool {
    let folded = fold_fullwidth_ascii(line);
    let t_lower = folded.trim().to_ascii_lowercase();

    t_lower == "tool"
        || t_lower.starts_with("tool ")
        || is_known_tool_name(&t_lower)
        || is_compact_tool_call_trace(&t_lower)
}

/// 处理 strip internal tool trace 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回字符串已经按界面展示或比较需求做过必要整理。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn strip_internal_tool_trace(s: &str) -> String {
    static STRIPPED_TOOL_TRACE_CACHE: Lazy<Mutex<HashMap<u64, String>>> =
        Lazy::new(|| Mutex::new(HashMap::new()));
    /// STRIPPED_TOOL_TRACE_CACHE_LIMIT 是当前模块共享的固定参数。
    const STRIPPED_TOOL_TRACE_CACHE_LIMIT: usize = 512;

    if s.is_empty()
        || (!s.contains("Called the ")
            && !s.contains("tool\n")
            && !s.contains("tool ")
            && !s.contains("<path>")
            && !s.contains("<content>")
            && !s.contains("<file>")
            && !s.contains("toolread(")
            && !s.contains("toolwrite(")
            && !s.contains("tooledit(")
            && !s.contains("toolbash(")
            && !s.contains("todoread")
            && !s.contains("todowrite")
            && !s.contains("metadata")
            && !s.contains("outputPath")
            && !s.contains("missing required field")
            && !s.contains("expected schema")
            && !s.lines().any(is_tool_trace_hint_line))
    {
        return s.to_string();
    }

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    let cache_key = hasher.finish();

    if let Ok(cache) = STRIPPED_TOOL_TRACE_CACHE.lock()
        && let Some(cached) = cache.get(&cache_key)
    {
        return cached.clone();
    }

    /// 处理 looks like tool leak line 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// `true` 表示当前输入满足该辅助函数描述的条件。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn looks_like_tool_leak_line(t_lower: &str) -> bool {
        t_lower.starts_with("tool ")
            || is_known_tool_name(t_lower)
            || is_compact_tool_call_trace(t_lower)
            || t_lower.starts_with("(\"input\"")
            || t_lower.starts_with("(\"output\"")
            || t_lower.contains("outputpath")
            || t_lower.contains("tool_calls")
            || t_lower.contains("missing required field")
            || t_lower.contains("invalid arguments")
            || t_lower.contains("expected schema")
            || t_lower.contains("status\":\"completed")
            || t_lower.contains("title\":\"todoread")
            || t_lower.contains("metadata")
            || t_lower.contains("truncated")
    }

    let mut out = String::with_capacity(s.len());
    let mut in_content_dump = false;
    let mut in_file_dump = false;
    let mut in_tool_leak_dump = false;
    let mut in_tool_input_dump = false;

    for line in s.lines() {
        let folded = fold_fullwidth_ascii(line);
        let t = folded.trim();
        let t_lower = t.to_ascii_lowercase();

        if in_tool_input_dump {
            if t.is_empty() {
                in_tool_input_dump = false;
            }
            continue;
        }

        if in_tool_leak_dump {
            if t.is_empty() {
                in_tool_leak_dump = false;
                continue;
            }
            if looks_like_tool_leak_line(&t_lower) {
                continue;
            }
            in_tool_leak_dump = false;
        }

        if in_content_dump || in_file_dump {
            if t_lower.contains("</content>") {
                in_content_dump = false;
            }
            if t_lower.contains("</file>") {
                in_file_dump = false;
            }
            continue;
        }

        if t_lower.contains("called the ")
            && (t_lower.contains(" tool with the following input:")
                || t_lower.ends_with(" tool")
                || t_lower.contains(" tool "))
        {
            in_tool_input_dump = t_lower.contains(" tool with the following input:");
            in_tool_leak_dump = !in_tool_input_dump;
            continue;
        }

        if t_lower == "tool" || looks_like_tool_leak_line(&t_lower) {
            in_tool_leak_dump = true;
            continue;
        }

        if t_lower.contains("<content>") {
            in_content_dump = true;
            continue;
        }
        if t_lower.contains("<file>") {
            in_file_dump = true;
            continue;
        }

        if matches!(
            t_lower.as_str(),
            "<path>"
                | "</path>"
                | "<type>"
                | "</type>"
                | "<content>"
                | "</content>"
                | "<file>"
                | "</file>"
                | "<file_link>"
                | "</file_link>"
        ) || t_lower.contains("<path>")
            || t_lower.contains("<type>")
            || t_lower.contains("<file_link>")
            || t_lower.starts_with("(output capped at")
            || t_lower.starts_with("(end of file")
            || t_lower.starts_with("(showing lines")
            || is_compact_tool_call_trace(&t_lower)
            || t_lower.contains("toolread(")
            || t_lower.contains("toolwrite(")
            || t_lower.contains("tooledit(")
            || t_lower.contains("toolbash(")
            || t_lower.contains("tooltodowrite")
            || t_lower.contains("tooltodoread")
        {
            continue;
        }

        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(line);
    }

    if let Ok(mut cache) = STRIPPED_TOOL_TRACE_CACHE.lock() {
        if cache.len() >= STRIPPED_TOOL_TRACE_CACHE_LIMIT
            && let Some(oldest_key) = cache.keys().next().copied()
        {
            cache.remove(&oldest_key);
        }
        cache.insert(cache_key, out.clone());
    }

    out
}
