//! 跨平台出站消息格式规整，行为对齐 QwenPaw 对应通道。

/// 规整钉钉容易误解析的列表与缩进代码块。
pub(crate) fn normalize_chat_markdown(text: &str) -> String {
    dedent_fenced_code_blocks(&ensure_numbered_list_spacing(text))
}

/// 飞书 post Markdown 要求围栏前换行，否则可能无法识别代码块。
pub(crate) fn normalize_feishu_markdown(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    for (offset, _) in text.match_indices("```") {
        output.push_str(&text[cursor..offset]);
        if offset > 0 && text[..offset].chars().next_back() != Some('\n') {
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
    let lines = text.split('\n').collect::<Vec<_>>();
    let mut output = Vec::new();
    let mut index = 0;
    let mut in_code_fence = false;
    while index < lines.len() {
        let line = lines[index];
        if line.trim().starts_with("```") {
            in_code_fence = !in_code_fence;
            output.push(line.to_string());
            index += 1;
            continue;
        }
        if in_code_fence || !line.contains('|') {
            output.push(line.to_string());
            index += 1;
            continue;
        }
        let start = index;
        while index < lines.len()
            && lines[index].contains('|')
            && !lines[index].trim().starts_with("```")
        {
            index += 1;
        }
        output.extend(format_wecom_table(&lines[start..index]));
    }
    output.join("\n")
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
            let mut cells = line.split('|').map(str::trim).collect::<Vec<_>>();
            if cells.first().is_some_and(|cell| cell.is_empty()) {
                cells.remove(0);
            }
            if cells.last().is_some_and(|cell| cell.is_empty()) {
                cells.pop();
            }
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
            let separators =
                widths.iter().map(|width| "-".repeat((*width).max(3))).collect::<Vec<_>>();
            formatted.push(format!("| {} |", separators.join(" | ")));
        }
    }
    formatted
}

fn ensure_numbered_list_spacing(text: &str) -> String {
    let lines = text.split('\n').collect::<Vec<_>>();
    let mut output = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        let is_numbered = is_numbered_list_item(line);
        if is_numbered && index > 0 {
            let previous = lines[index - 1];
            if !previous.trim().is_empty() && !is_numbered_list_item(previous) {
                output.push("");
            }
        }
        output.push(*line);
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
    let mut fence_indent: Option<String> = None;
    for line in text.split('\n') {
        if let Some(indent) = fence_indent.as_ref() {
            let dedented = line.strip_prefix(indent).unwrap_or(line);
            output.push(dedented);
            if dedented.trim_start().starts_with("```") {
                fence_indent = None;
            }
            continue;
        }
        let trimmed = line.trim_start_matches([' ', '\t']);
        if trimmed.starts_with("```") {
            let indent_len = line.len().saturating_sub(trimmed.len());
            let indent = &line[..indent_len];
            output.push(trimmed);
            fence_indent = Some(indent.to_string());
        } else {
            output.push(line);
        }
    }
    output.join("\n")
}
