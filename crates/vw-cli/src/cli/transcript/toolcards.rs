//! 构建 CLI 转录中的工具卡片文本。
//! 模块负责把结构化工具状态压缩成人类可扫描的终端摘要。

use super::super::theme::{TEXT_MUTED, TEXT_SUBTLE};
use super::todocards::tool_badge_cli;
use super::truncate::truncate_chars_cli;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// 执行 render_tool_card 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) fn render_tool_card(
    name: &str,
    summary: &str,
    input_raw: &str,
    _expand_tool_details: bool,
) -> Vec<Line<'static>> {
    let (_, badge_color) = tool_badge_cli(name);
    let title = tool_title_cli(name);
    let rows = tool_rows_cli(name, summary, input_raw);
    let status = tool_status_label(input_raw);
    let mut out = Vec::new();

    out.push(tool_header_line(name, badge_color, title, status.as_deref(), summary));

    let detail_rows =
        rows.iter().filter(|(label, _)| label != "概览" || summary.is_empty()).collect::<Vec<_>>();

    let primary_detail = primary_tool_detail(name, &detail_rows);

    if let Some(detail_line) = tool_detail_line(primary_detail) {
        out.push(detail_line);
    }

    out
}

/// 执行 tool_summary_cli 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) fn tool_summary_cli(tool_name: &str, input: &str) -> String {
    let trimmed = input.trim();
    let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return truncate_chars_cli(trimmed, 120);
    };
    if !v.is_object() {
        return generic_summary_cli(&v);
    }

    match tool_name {
        "read" => {
            let path = string_field(&v, &["filePath", "path"]);
            let offset = int_field(&v, &["offset"]).unwrap_or(1).max(1);
            let limit = int_field(&v, &["limit"]).unwrap_or(2000).max(1);
            let end = offset.saturating_add(limit.saturating_sub(1));
            if path.is_empty() {
                format!("line {offset}-{end}")
            } else {
                format!("{}  line {offset}-{end}", compact_cli_path(&path, 80))
            }
        }
        "grep" | "content_search" => {
            let pattern = string_field(&v, &["pattern", "input", "query"]);
            let path = string_field(&v, &["path"]);
            let mut out = String::new();
            if !path.is_empty() {
                out.push_str(&compact_cli_path(&path, 60));
            }
            if !pattern.is_empty() {
                if !out.is_empty() {
                    out.push_str("  ");
                }
                out.push_str("pattern=");
                out.push_str(&truncate_chars_cli(&pattern, 80));
            }
            if out.is_empty() { "grep".to_string() } else { out }
        }
        "glob" | "glob_search" => {
            let pattern = string_field(&v, &["pattern", "glob"]);
            let path = string_field(&v, &["path"]);
            if path.is_empty() {
                format!("pattern={}", truncate_chars_cli(&pattern, 80))
            } else {
                format!(
                    "{}  pattern={}",
                    compact_cli_path(&path, 56),
                    truncate_chars_cli(&pattern, 80)
                )
            }
        }
        "lsp" | "codesearch" => {
            let query =
                string_field(&v, &["query", "symbol", "information_request", "lineContent"]);
            let path = string_field(&v, &["filePath", "path", "includePattern"]);
            match (query.is_empty(), path.is_empty()) {
                (true, true) => "workspace search".to_string(),
                (false, true) => truncate_chars_cli(&query, 100),
                (true, false) => compact_cli_path(&path, 72),
                (false, false) => {
                    truncate_chars_cli(&format!("{}  {}", compact_cli_path(&path, 48), query), 100)
                }
            }
        }
        "bash" | "shell" => {
            let desc = string_field(&v, &["description"]);
            let workdir = string_field(&v, &["workdir"]);
            if !desc.is_empty() && !workdir.is_empty() {
                truncate_chars_cli(&format!("{desc}  {}", compact_cli_path(&workdir, 48)), 100)
            } else if !desc.is_empty() {
                truncate_chars_cli(&desc, 100)
            } else if !workdir.is_empty() {
                format!("工作目录 {}", compact_cli_path(&workdir, 72))
            } else {
                "bash command".to_string()
            }
        }
        "browser" | "browser_open" => browser_summary_cli(&v),
        "apply_patch" => summarize_apply_patch_tool(&v),
        "todowrite" => {
            let input_raw = string_field(&v, &["input"]);
            if let Ok(iv) = serde_json::from_str::<serde_json::Value>(input_raw.trim()) {
                let total =
                    iv.get("todos").and_then(|x| x.as_array()).map(std::vec::Vec::len).unwrap_or(0);
                let merge = iv.get("merge").and_then(|x| x.as_bool()).unwrap_or(false);
                let action = if merge { "merge" } else { "write" };
                return format!("todos={total} action={action}");
            }
            "todos".to_string()
        }
        "todoread" => "todo list".to_string(),
        _ => generic_summary_cli(&v),
    }
}

/// 执行 compact_cli_path 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) fn compact_cli_path(path: &str, max_chars: usize) -> String {
    if path.chars().count() <= max_chars {
        return path.to_string();
    }

    let separators: &[_] = &['/', '\\'];
    let mut parts: Vec<&str> = path.split(separators).filter(|part| !part.is_empty()).collect();
    if parts.is_empty() {
        return truncate_chars_cli(path, max_chars);
    }

    let mut suffix = parts.pop().unwrap_or_default().to_string();
    while let Some(part) = parts.pop() {
        let candidate = format!("{part}/{suffix}");
        if candidate.chars().count().saturating_add(4) > max_chars {
            break;
        }
        suffix = candidate;
    }

    truncate_chars_cli(&format!(".../{suffix}"), max_chars)
}

fn tool_title_cli(name: &str) -> &'static str {
    match name {
        "read" => "读取",
        "write" => "写入",
        "apply_patch" => "补丁",
        "grep" | "content_search" | "glob" | "glob_search" | "lsp" | "codesearch" => "搜索",
        "bash" | "shell" => "运行",
        "browser" | "browser_open" => "浏览器",
        "question" => "提问",
        "task" => "任务",
        "webfetch" => "抓取",
        _ => "工具",
    }
}

fn tool_rows_cli(name: &str, summary: &str, input_raw: &str) -> Vec<(String, String)> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(input_raw.trim()) else {
        return if summary.is_empty() {
            Vec::new()
        } else {
            vec![("概览".to_string(), truncate_chars_cli(summary, 96))]
        };
    };

    let mut rows = Vec::new();
    if !summary.is_empty() {
        rows.push(("概览".to_string(), truncate_chars_cli(summary, 96)));
    }

    match name {
        "read" => {
            let path = string_field(&v, &["filePath", "path"]);
            let offset = int_field(&v, &["offset"]).unwrap_or(1).max(1);
            let limit = int_field(&v, &["limit"]).unwrap_or(2000).max(1);
            let end = offset.saturating_add(limit.saturating_sub(1));
            push_row_if_value(
                &mut rows,
                "路径",
                if path.is_empty() { String::new() } else { compact_cli_path(&path, 96) },
            );
            rows.push(("范围".to_string(), format!("line {offset}-{end}")));
        }
        "write" => {
            let path = string_field(&v, &["filePath", "path"]);
            push_row_if_value(
                &mut rows,
                "路径",
                if path.is_empty() { String::new() } else { compact_cli_path(&path, 96) },
            );
        }
        "apply_patch" => rows.extend(apply_patch_rows(&v)),
        "grep" | "content_search" => {
            push_path_row(&mut rows, "目录", &string_field(&v, &["path"]), 96);
            push_row_if_value(
                &mut rows,
                "文件",
                truncate_chars_cli(&string_field(&v, &["include"]), 96),
            );
            push_row_if_value(
                &mut rows,
                "模式",
                truncate_chars_cli(&string_field(&v, &["pattern", "input", "query"]), 96),
            );
        }
        "glob" | "glob_search" => {
            push_path_row(&mut rows, "目录", &string_field(&v, &["path"]), 96);
            push_row_if_value(
                &mut rows,
                "模式",
                truncate_chars_cli(&string_field(&v, &["pattern", "glob"]), 96),
            );
        }
        "lsp" | "codesearch" => {
            push_row_if_value(
                &mut rows,
                "查询",
                truncate_chars_cli(
                    &string_field(&v, &["query", "symbol", "information_request", "lineContent"]),
                    96,
                ),
            );
            push_path_row(
                &mut rows,
                "路径",
                &string_field(&v, &["filePath", "path", "includePattern"]),
                96,
            );
        }
        "bash" | "shell" => {
            push_row_if_value(
                &mut rows,
                "说明",
                truncate_chars_cli(&string_field(&v, &["description"]), 96),
            );
            push_row_if_value(
                &mut rows,
                "命令",
                truncate_chars_cli(&string_field(&v, &["command", "cmd"]), 96),
            );
            push_path_row(&mut rows, "目录", &string_field(&v, &["workdir"]), 96);
            if let Some(timeout) = int_field(&v, &["timeout"]) {
                rows.push(("超时".to_string(), format!("{timeout} ms")));
            }
        }
        "browser" | "browser_open" => {
            push_row_if_value(
                &mut rows,
                "动作",
                truncate_chars_cli(&string_field(&v, &["action"]), 96),
            );
            push_row_if_value(&mut rows, "目标", truncate_chars_cli(&browser_target_cli(&v), 96));
        }
        "question" => rows.extend(question_rows(&v)),
        "task" => rows.extend(task_rows(&v)),
        "webfetch" => {
            push_row_if_value(
                &mut rows,
                "链接",
                truncate_chars_cli(&string_field(&v, &["url"]), 96),
            );
            push_row_if_value(
                &mut rows,
                "格式",
                truncate_chars_cli(&string_field(&v, &["format"]), 96),
            );
        }
        _ => rows.extend(generic_rows(&v)),
    }

    dedup_rows(rows)
}

fn apply_patch_rows(v: &serde_json::Value) -> Vec<(String, String)> {
    let patch = string_field(v, &["patchText"]);
    let mut added = 0usize;
    let mut updated = 0usize;
    let mut deleted = 0usize;
    let mut files = Vec::new();

    for line in patch.lines() {
        let trimmed = line.trim();
        if let Some(path) = trimmed.strip_prefix("*** Add File: ") {
            added = added.saturating_add(1);
            files.push(("新增", compact_cli_path(path.trim(), 92)));
        } else if let Some(path) = trimmed.strip_prefix("*** Update File: ") {
            updated = updated.saturating_add(1);
            files.push(("更新", compact_cli_path(path.trim(), 92)));
        } else if let Some(path) = trimmed.strip_prefix("*** Delete File: ") {
            deleted = deleted.saturating_add(1);
            files.push(("删除", compact_cli_path(path.trim(), 92)));
        }
    }

    let mut rows = Vec::new();
    if !files.is_empty() {
        rows.push((
            "变更".to_string(),
            format!("{} 项 · 新增 {added} · 更新 {updated} · 删除 {deleted}", files.len()),
        ));
        rows.extend(files.into_iter().take(6).map(|(label, value)| (label.to_string(), value)));
    }
    rows
}

fn question_rows(v: &serde_json::Value) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    let Some(questions) = v.get("questions").and_then(|x| x.as_array()) else {
        return rows;
    };

    rows.push(("问题数".to_string(), format!("{} 项", questions.len())));
    if let Some(first) = questions.first() {
        push_row_if_value(
            &mut rows,
            "标题",
            truncate_chars_cli(&string_field(first, &["header"]), 96),
        );
        push_row_if_value(
            &mut rows,
            "问题",
            truncate_chars_cli(&string_field(first, &["question"]), 96),
        );
        let option_count =
            first.get("options").and_then(|x| x.as_array()).map(std::vec::Vec::len).unwrap_or(0);
        if option_count > 0 {
            rows.push(("选项".to_string(), format!("{} 个", option_count)));
        }
    }
    rows
}

fn task_rows(v: &serde_json::Value) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    push_row_if_value(
        &mut rows,
        "任务",
        truncate_chars_cli(&string_field(v, &["description"]), 96),
    );
    push_row_if_value(
        &mut rows,
        "代理",
        truncate_chars_cli(&string_field(v, &["subagent_type"]), 96),
    );
    push_row_if_value(&mut rows, "命令", truncate_chars_cli(&string_field(v, &["command"]), 96));
    let prompt = first_line(&string_field(v, &["prompt"]));
    push_row_if_value(&mut rows, "提示", truncate_chars_cli(&prompt, 96));
    rows
}

fn generic_rows(v: &serde_json::Value) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    push_path_row(&mut rows, "路径", &string_field(v, &["filePath", "path"]), 96);
    push_path_row(&mut rows, "目录", &string_field(v, &["workdir"]), 96);

    if let Some(obj) = v.as_object() {
        for (key, value) in obj {
            if matches!(
                key.as_str(),
                "filePath"
                    | "path"
                    | "workdir"
                    | "patchText"
                    | "command"
                    | "prompt"
                    | "input"
                    | "operations"
            ) {
                continue;
            }
            if let Some(text) = scalar_value_summary(value) {
                rows.push((key.to_string(), truncate_chars_cli(&text, 96)));
            }
            if rows.len() >= 6 {
                break;
            }
        }
    }
    rows
}

fn dedup_rows(rows: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (label, value) in rows {
        if value.trim().is_empty() {
            continue;
        }
        if out.iter().any(|(existing_label, existing_value)| {
            existing_label == &label && existing_value == &value
        }) {
            continue;
        }
        out.push((label, value));
    }
    out
}

fn tool_header_line(
    tool_name: &str,
    badge_color: Color,
    title: &str,
    status: Option<&str>,
    summary: &str,
) -> Line<'static> {
    let emoji = tool_emoji_cli(tool_name);
    let mut spans = vec![
        Span::raw(format!("{emoji} ")),
        Span::styled(
            title.to_string(),
            Style::default().fg(badge_color).add_modifier(Modifier::BOLD),
        ),
    ];

    if let Some(status) = status.filter(|status| !status.is_empty()) {
        spans.push(Span::styled(status.to_string(), Style::default().fg(TEXT_SUBTLE)));
    }

    if !summary.trim().is_empty() {
        spans.push(Span::styled(" · ", Style::default().fg(TEXT_SUBTLE)));
        spans.push(Span::styled(truncate_chars_cli(summary, 84), Style::default().fg(TEXT_MUTED)));
    }

    Line::from(spans)
}

fn tool_detail_line(row: Option<&(String, String)>) -> Option<Line<'static>> {
    let (label, value) = row?;

    Some(Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{label} {value}"), Style::default().fg(TEXT_MUTED)),
    ]))
}

fn primary_tool_detail<'a>(
    tool_name: &str,
    rows: &'a [&'a (String, String)],
) -> Option<&'a (String, String)> {
    let priorities: &[&str] = match tool_name {
        "read" | "write" | "apply_patch" => &["路径", "范围", "变更"],
        "bash" | "shell" => &["命令", "目录", "说明"],
        "browser" | "browser_open" => &["目标", "动作"],
        "grep" | "content_search" | "glob" | "glob_search" => &["模式", "目录", "路径"],
        "lsp" | "codesearch" => &["查询", "路径", "目录"],
        "webfetch" => &["链接", "格式"],
        "question" => &["问题", "标题"],
        "task" => &["任务", "命令", "提示"],
        _ => &["路径", "命令", "范围", "目录"],
    };

    for wanted in priorities {
        if let Some(row) = rows.iter().find(|(label, _)| label == wanted) {
            return Some(row);
        }
    }

    rows.first().copied()
}

fn tool_status_label(input_raw: &str) -> Option<String> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(input_raw.trim()) else {
        return None;
    };

    let status = v.get("status").and_then(|x| x.as_str()).unwrap_or("").trim();
    match status {
        "completed" => Some("已完成".to_string()),
        "running" | "in_progress" => Some("进行中".to_string()),
        "error" => Some("失败".to_string()),
        "denied" => Some("已拒绝".to_string()),
        _ => None,
    }
}

fn tool_emoji_cli(tool_name: &str) -> &'static str {
    match tool_name {
        "read" => "📖",
        "write" => "✏️",
        "apply_patch" => "📝",
        "grep" | "content_search" | "glob" | "glob_search" | "lsp" | "codesearch" => "🔍",
        "bash" | "shell" => "⚡",
        "webfetch" => "🌐",
        "question" => "❓",
        "task" => "📋",
        "todowrite" | "todoread" => "✅",
        _ => "🔧",
    }
}

fn push_row_if_value(rows: &mut Vec<(String, String)>, label: &str, value: String) {
    if !value.trim().is_empty() {
        rows.push((label.to_string(), value));
    }
}

fn push_path_row(rows: &mut Vec<(String, String)>, label: &str, path: &str, max_chars: usize) {
    if !path.trim().is_empty() {
        rows.push((label.to_string(), compact_cli_path(path, max_chars)));
    }
}

fn string_field(v: &serde_json::Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(value) =
            v.get(*key).and_then(|x| x.as_str()).map(str::trim).filter(|s| !s.is_empty())
        {
            return value.to_string();
        }
    }
    String::new()
}

fn int_field(v: &serde_json::Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| v.get(*key).and_then(|x| x.as_i64()))
}

fn scalar_value_summary(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
        }
        serde_json::Value::Bool(v) => Some(if *v { "true" } else { "false" }.to_string()),
        serde_json::Value::Number(v) => Some(v.to_string()),
        serde_json::Value::Array(v) => Some(format!("{} 项", v.len())),
        serde_json::Value::Object(v) => Some(format!("{} 个字段", v.len())),
        serde_json::Value::Null => None,
    }
}

fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or("").trim().to_string()
}

fn summarize_apply_patch_tool(v: &serde_json::Value) -> String {
    let rows = apply_patch_rows(v);
    if let Some((_, value)) = rows.first() { value.clone() } else { "patch".to_string() }
}

fn browser_summary_cli(v: &serde_json::Value) -> String {
    let action = string_field(v, &["action"]);
    let target = browser_target_cli(v);
    match (action.is_empty(), target.is_empty()) {
        (false, false) => truncate_chars_cli(&format!("{action} {target}"), 100),
        (false, true) => truncate_chars_cli(&action, 100),
        (true, false) => truncate_chars_cli(&target, 100),
        (true, true) => "browser".to_string(),
    }
}

fn generic_summary_cli(v: &serde_json::Value) -> String {
    for key in ["selector", "target", "url", "title", "description", "action", "key"] {
        let value = string_field(v, &[key]);
        if !value.is_empty() {
            return truncate_chars_cli(&value, 100);
        }
    }

    let path = string_field(v, &["filePath", "path"]);
    if !path.is_empty() {
        return compact_cli_path(&path, 88);
    }

    if let Some(obj) = v.as_object() {
        return format!("{} 项参数", obj.len());
    }

    "工具".to_string()
}

fn browser_target_cli(v: &serde_json::Value) -> String {
    for key in ["selector", "url", "title", "target", "text"] {
        let value = string_field(v, &[key]);
        if !value.is_empty() {
            return value;
        }
    }
    String::new()
}
