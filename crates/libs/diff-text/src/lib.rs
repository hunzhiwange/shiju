//! 工具 diff 预览解析工具。
//!
//! 本模块从工具输入、输出和 apply_patch 摘要中提取可展示的文件预览与增删行统计。

pub fn looks_like_unified_diff(s: &str) -> bool {
    let t = s.trim();
    !t.is_empty() && t.starts_with("--- ") && t.contains("\n+++ ") && t.contains("\n@@")
}

/// 从原始输入中提取 diff block。
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
pub fn extract_diff_block(s: &str) -> Option<String> {
    let start = s.find("<diff>")? + "<diff>".len();
    let end = start + s[start..].find("</diff>")?;
    if end <= start {
        return None;
    }
    Some(s[start..end].trim_matches('\n').to_string())
}

/// 解析 apply patch summary 的输入文本，返回后续视图可以直接消费的结构化结果。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回集合保持输入顺序或界面展示顺序，空集合表示没有可展示项。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn parse_apply_patch_summary(output: &str) -> Vec<(char, String)> {
    let mut out = Vec::new();
    let mut in_updated_list = false;
    for line in output.lines() {
        let l = line.trim();
        if l.starts_with("The following files have been updated") {
            in_updated_list = true;
            continue;
        }
        if in_updated_list {
            if l.is_empty() {
                in_updated_list = false;
                continue;
            }
            if let Some(path) = l.strip_prefix("- ") {
                let path = path.trim();
                if is_likely_file_path(path) {
                    out.push(('M', path.to_string()));
                }
                continue;
            }
        }
        // 过滤掉不是状态行的文本，避免把普通输出误判成文件变更摘要。
        if l.len() < 2 || l.as_bytes().get(1).is_none_or(|b| *b != b' ') {
            continue;
        }
        let kind = l.chars().next().unwrap_or('\0');
        if !matches!(kind, 'A' | 'M' | 'D') {
            return Vec::new();
        }
        let rest = l.get(1..).unwrap_or("").trim();
        if !is_likely_file_path(rest) {
            continue;
        }
        out.push((kind, rest.to_string()));
    }
    out
}

/// 解析 apply patch line changes 的输入文本，返回后续视图可以直接消费的结构化结果。
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
pub fn parse_apply_patch_line_changes(output: &str) -> (usize, usize) {
    let mut adds = 0usize;
    let mut dels = 0usize;

    for line in output.lines() {
        let l = line.trim();
        // 过滤掉不是状态行的文本，避免把普通输出误判成文件变更摘要。
        if l.len() < 2 || l.as_bytes().get(1).is_none_or(|b| *b != b' ') {
            continue;
        }
        let kind = l.chars().next().unwrap_or('\0');
        if !matches!(kind, 'A' | 'M' | 'D') {
            continue;
        }
        for t in l.split_whitespace() {
            if let Some(t) = t.strip_prefix('+') {
                if let Some((a, d)) = t.split_once('-') {
                    if let Ok(a) = a.parse::<usize>() {
                        adds = adds.saturating_add(a);
                    }
                    if let Ok(d) = d.parse::<usize>() {
                        dels = dels.saturating_add(d);
                    }
                } else if let Ok(a) = t.parse::<usize>() {
                    adds = adds.saturating_add(a);
                }
            } else if let Some(t) = t.strip_prefix('-')
                && let Ok(d) = t.parse::<usize>()
            {
                dels = dels.saturating_add(d);
            }
        }
    }

    (adds, dels)
}

/// 统计 unified diff changes 中的增删或数量信息。
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
pub fn count_unified_diff_changes(diff: &str) -> (usize, usize) {
    let mut adds = 0usize;
    let mut dels = 0usize;
    for line in diff.lines() {
        if let Some(c) = line.chars().next() {
            match c {
                '+' => {
                    if !line.starts_with("+++") {
                        adds += 1;
                    }
                }
                '-' => {
                    if !line.starts_with("---") {
                        dels += 1;
                    }
                }
                _ => {}
            }
        }
    }
    (adds, dels)
}

/// 统计 apply patch format changes 中的增删或数量信息。
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
pub fn count_apply_patch_format_changes(patch: &str) -> (usize, usize) {
    let mut adds = 0usize;
    let mut dels = 0usize;
    for line in patch.lines() {
        if line.starts_with("***") {
            continue;
        }
        if let Some(c) = line.chars().next() {
            match c {
                '+' => adds += 1,
                '-' => dels += 1,
                _ => {}
            }
        }
    }
    (adds, dels)
}

/// 处理 file preview 对应的局部职责。
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
pub fn file_preview(tool_name: &str, input: &str, output: &str) -> Option<String> {
    if tool_name == "apply_patch" {
        if let Some(diff) = extract_diff_block(output) {
            let t = diff.trim();
            return if t.is_empty() { None } else { Some(t.to_string()) };
        }
        let t = input.trim();
        return if t.is_empty() { None } else { Some(t.to_string()) };
    }

    if let Some(diff) = extract_diff_block(output) {
        let t = diff.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    if looks_like_unified_diff(output) {
        return Some(output.trim().to_string());
    }

    if !input.trim_start().starts_with('{') {
        return None;
    }
    let vv = serde_json::from_str::<serde_json::Value>(input.trim()).ok()?;
    match tool_name {
        "write" | "file_write" => vv
            .get("content")
            .and_then(|v| v.as_str())
            .filter(|s| !is_omitted_placeholder(s))
            .map(|s| s.to_string()),
        "file_edit" => vv
            .get("new_string")
            .or_else(|| vv.get("newString"))
            .and_then(|v| v.as_str())
            .filter(|s| !is_omitted_placeholder(s))
            .map(|s| s.to_string()),
        "notebook_edit" => {
            vv.get("new_code").or_else(|| vv.get("newCode")).and_then(string_or_string_array)
        }
        _ => None,
    }
}

fn is_omitted_placeholder(value: &str) -> bool {
    let Some(rest) = value.trim().strip_prefix("<omitted ") else {
        return false;
    };
    let Some(count) = rest.strip_suffix(" chars>") else {
        return false;
    };
    !count.is_empty() && count.chars().all(|ch| ch.is_ascii_digit())
}

/// 处理 string or string array 对应的局部职责。
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
fn string_or_string_array(value: &serde_json::Value) -> Option<String> {
    match value {
        // serde_json 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        serde_json::Value::String(text) => Some(text.clone()),
        // serde_json 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        serde_json::Value::Array(items) => {
            let lines = items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            if lines.is_empty() { None } else { Some(lines.join("\n")) }
        }
        _ => None,
    }
}

/// 处理 is likely file path 对应的局部职责。
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
fn is_likely_file_path(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() || t.contains('\n') || t.contains('\r') {
        return false;
    }
    if t.starts_with("http://") || t.starts_with("https://") {
        return false;
    }
    if t.chars().any(|c| c == '<' || c == '>' || c == '`' || c == '|') {
        return false;
    }
    if t.contains("  ") {
        return false;
    }
    let has_path_hint = t.contains('/')
        || t.contains('\\')
        || t.contains('.')
        || t.starts_with("./")
        || t.starts_with("../");
    let has_space = t.contains(' ') || t.contains('\t');
    has_path_hint && !has_space
}

#[cfg(test)]
mod tests;
