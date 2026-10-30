//! 跨平台出站消息格式规整，行为对齐 QwenPaw 对应通道。

use super::markdown_chunk::{markdown_code_ranges, markdown_table_ranges};

/// 规整钉钉容易误解析的列表与缩进代码块。
pub(crate) fn normalize_chat_markdown(text: &str) -> String {
    dedent_fenced_code_blocks(&ensure_numbered_list_spacing(text))
}

/// 飞书 post Markdown 要求围栏前换行，否则可能无法识别代码块。
pub(crate) fn normalize_feishu_markdown(text: &str) -> String {
    let protected = markdown_code_ranges(text);
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    for (offset, _) in text.match_indices("```") {
        if protected.iter().any(|range| range.contains(&offset)) {
            continue;
        }
        // Only repair an opening fence attached to prose, never part of a
        // longer marker or inline-code span.
        if text[..offset].ends_with('`') || text[offset + 3..].starts_with('`') {
            continue;
        }
        let rest = text[offset + 3..].split('\n').next().unwrap_or_default();
        if rest.contains('`') {
            continue;
        }
        output.push_str(&text[cursor..offset]);
        if offset > 0 && !text[..offset].ends_with('\n') {
            output.push('\n');
        }
        output.push_str("```");
        cursor = offset + 3;
    }
    output.push_str(&text[cursor..]);
    output
}

/// 按企业微信的要求对齐 GFM 表格列，代码围栏内保持原样。
pub(crate) fn format_wecom_markdown_tables(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    for range in markdown_table_ranges(text) {
        output.push_str(&text[cursor..range.start]);
        let source = &text[range.clone()];
        let lines = source.trim_end_matches(['\r', '\n']).split('\n').collect::<Vec<_>>();
        output.push_str(&format_wecom_table(&lines).join("\n"));
        output.push_str(&source[source.trim_end_matches(['\r', '\n']).len()..]);
        cursor = range.end;
    }
    output.push_str(&text[cursor..]);
    output
}

/// Pipes escaped in source or inside a code span belong to their cell.
fn table_cells(line: &str) -> Vec<&str> {
    let mut cells = Vec::new();
    let mut start = 0;
    let mut code_ticks = 0;
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'`' => {
                let count = bytes[index..].iter().take_while(|byte| **byte == b'`').count();
                if code_ticks == 0 {
                    code_ticks = count;
                } else if code_ticks == count {
                    code_ticks = 0;
                }
                index += count;
            }
            b'|' if code_ticks == 0 => {
                cells.push(line[start..index].trim());
                index += 1;
                start = index;
            }
            _ => index += 1,
        }
    }
    cells.push(line[start..].trim());
    if cells.first() == Some(&"") {
        cells.remove(0);
    }
    if cells.last() == Some(&"") {
        cells.pop();
    }
    cells
}

fn format_wecom_table(lines: &[&str]) -> Vec<String> {
    let has_separator = lines.get(1).is_some_and(|line| {
        !line.is_empty()
            && line
                .chars()
                .all(|character| character.is_whitespace() || matches!(character, '-' | ':' | '|'))
    });
    let rows = lines
        .iter()
        .enumerate()
        .filter(|(index, _)| !has_separator || *index != 1)
        .filter_map(|(_, line)| {
            let cells = table_cells(line);
            (!cells.is_empty()).then_some(cells)
        })
        .collect::<Vec<_>>();
    let column_count = rows.iter().map(Vec::len).max().unwrap_or_default();
    let mut widths = vec![0; column_count];
    for row in &rows {
        for (index, width) in widths.iter_mut().enumerate() {
            *width = (*width).max(row.get(index).map_or(0, |cell| cell.chars().count()));
        }
    }
    let mut formatted = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        let cells = widths
            .iter()
            .enumerate()
            .map(|(index, width)| {
                let cell = row.get(index).copied().unwrap_or("");
                format!("{cell}{}", " ".repeat(width.saturating_sub(cell.chars().count())))
            })
            .collect::<Vec<_>>();
        formatted.push(format!("| {} |", cells.join(" | ")));
        if row_index == 0 {
            let separators = widths
                .iter()
                .enumerate()
                .map(|(index, width)| {
                    let alignment = lines.get(1).map(|line| table_cells(line)).unwrap_or_default();
                    let marker = alignment.get(index).copied().unwrap_or_default();
                    let mut separator = "-".repeat((*width).max(3));
                    if marker.starts_with(':') {
                        separator.replace_range(..1, ":");
                    }
                    if marker.ends_with(':') {
                        separator.pop();
                        separator.push(':');
                    }
                    separator
                })
                .collect::<Vec<_>>();
            formatted.push(format!("| {} |", separators.join(" | ")));
        }
    }
    formatted
}

fn ensure_numbered_list_spacing(text: &str) -> String {
    let lines = text.split('\n').collect::<Vec<_>>();
    let protected = markdown_code_ranges(text);
    let mut offset = 0;
    let mut output = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        let is_numbered =
            is_numbered_list_item(line) && !protected.iter().any(|range| range.contains(&offset));
        if is_numbered && index > 0 {
            let previous = lines[index - 1];
            if !previous.trim().is_empty() && !is_numbered_list_item(previous) {
                output.push("");
            }
        }
        output.push(*line);
        offset += line.len() + 1;
    }
    output.join("\n")
}

fn is_numbered_list_item(line: &str) -> bool {
    let trimmed = line.trim();
    let Some((number, rest)) = trimmed.split_once('.') else {
        return false;
    };
    !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit())
        && rest.chars().next().is_some_and(char::is_whitespace)
}

fn dedent_fenced_code_blocks(text: &str) -> String {
    let mut output = Vec::new();
    let mut fence_indent: Option<(String, u8, usize)> = None;
    for line in text.split('\n') {
        if let Some((indent, marker, length)) = fence_indent.as_ref() {
            let dedented = line.strip_prefix(indent).unwrap_or(line);
            output.push(dedented);
            let trimmed = dedented.trim_start();
            let closing_length = trimmed.bytes().take_while(|byte| byte == marker).count();
            if closing_length >= *length && trimmed[closing_length..].trim().is_empty() {
                fence_indent = None;
            }
            continue;
        }
        let trimmed = line.trim_start_matches([' ', '\t']);
        let marker = trimmed.as_bytes().first().copied().unwrap_or_default();
        let length = trimmed.bytes().take_while(|byte| *byte == marker).count();
        if matches!(marker, b'`' | b'~') && length >= 3 {
            let indent_len = line.len().saturating_sub(trimmed.len());
            let indent = &line[..indent_len];
            output.push(trimmed);
            fence_indent = Some((indent.to_string(), marker, length));
        } else {
            output.push(line);
        }
    }
    output.join("\n")
}
