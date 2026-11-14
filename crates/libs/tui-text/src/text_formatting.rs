//! 提供面向终端展示的文本、JSON、路径和英文列表格式化辅助函数。

use unicode_segmentation::UnicodeSegmentation;

use crate::width::display_width;

pub fn capitalize_first(input: &str) -> String {
    let mut chars = input.chars();
    match chars.next() {
        Some(first) => {
            let mut capitalized = first.to_uppercase().collect::<String>();
            capitalized.push_str(chars.as_str());
            capitalized
        }
        None => String::new(),
    }
}

/// 将工具结果截断到给定高宽；若文本是有效 JSON，则先紧凑格式化再截断。
/// 这是尽力而为的近似：一个字素可能占用多个终端单元格。
/// Truncate a tool result to fit within the given height and width. If the text is valid JSON, we format it in a compact way before truncating.
/// This is a best-effort approach that may not work perfectly for text where 1 grapheme is rendered as multiple terminal cells.
pub fn format_and_truncate_tool_result(
    text: &str,
    max_lines: usize,
    line_width: usize,
) -> String {
    // 估算结果最多可展示的字素数；字素与终端单元格不保证一一对应，
    // 因此每行减一作为余量。该近似也不会随未来终端缩放重算。
    // Work out the maximum number of graphemes we can display for a result.
    // It's not guaranteed that 1 grapheme = 1 cell, so we subtract 1 per line as a fudge factor.
    // It also won't handle future terminal resizes properly, but it's an OK approximation for now.
    let max_graphemes = (max_lines * line_width).saturating_sub(max_lines);

    if let Some(formatted_json) = format_json_compact(text) {
        truncate_text(&formatted_json, max_graphemes)
    } else {
        truncate_text(text, max_graphemes)
    }
}

/// Formats JSON on one line with spaces after separators for readability and natural wrap points.
/// 将 JSON 格式化为单行，并在分隔符后保留空格以便阅读和自然换行。
///
/// 极致紧凑的 JSON 难以浏览，而美化 JSON 会占用过多终端行。
/// 有效输入返回格式化字符串，无效输入返回 `None`。
/// 示例：`{"a":"b","c":["d","e"]}` 变为 `{"a": "b", "c": ["d", "e"]}`。
/// Compact JSON is hard to scan, while pretty-printed JSON consumes unnecessary terminal rows.
/// Returns the formatted JSON string for valid input, or `None` otherwise.
///
/// Example: `{"a":"b","c":["d","e"]}` becomes `{"a": "b", "c": ["d", "e"]}`.
pub fn format_json_compact(text: &str) -> Option<String> {
    let json = serde_json::from_str::<serde_json::Value>(text).ok()?;
    let json_pretty = serde_json::to_string_pretty(&json).unwrap_or_else(|_| json.to_string());

    // 删除换行和多余空白，把多行美化 JSON 转为可读的紧凑单行。
    // Convert multi-line pretty JSON to compact single-line format by removing newlines and excess whitespace
    let mut result = String::new();
    let mut chars = json_pretty.chars().peekable();
    let mut in_string = false;
    let mut escape_next = false;

    // 逐字符遍历 JSON，只在字符串之外为冒号和逗号补一个空格。
    // Iterate over the characters in the JSON string, adding spaces after : and , but only when not in a string
    while let Some(ch) = chars.next() {
        match ch {
            '"' if !escape_next => {
                in_string = !in_string;
                result.push(ch);
            }
            '\\' if in_string => {
                escape_next = !escape_next;
                result.push(ch);
            }
            '\n' | '\r' if !in_string => {
                // 跳过字符串之外的换行。
                // Skip newlines when not in a string
            }
            ' ' | '\t' if !in_string => {
                // 仅在字符串之外、且后面不是闭合括号时补空格。
                // Add a space after : and , but only when not in a string
                if let Some(&next_ch) = chars.peek()
                    && let Some(last_ch) = result.chars().last()
                    && (last_ch == ':' || last_ch == ',')
                    && !matches!(next_ch, '}' | ']')
                {
                    result.push(' ');
                }
            }
            _ => {
                if escape_next && in_string {
                    escape_next = false;
                }
                result.push(ch);
            }
        }
    }

    Some(result)
}

/// 将 `text` 截断到 `max_graphemes` 个字素，避免切开由多个码点组成的字符。
/// Truncate `text` to `max_graphemes` graphemes. Using graphemes to avoid accidentally truncating in the middle of a multi-codepoint character.
pub fn truncate_text(text: &str, max_graphemes: usize) -> String {
    let mut graphemes = text.grapheme_indices(true);

    // 若 `max_graphemes` 位置仍有字素，说明原文确实溢出。
    // Check if there's a grapheme at position max_graphemes (meaning there are more than max_graphemes total)
    if let Some((byte_index, _)) = graphemes.nth(max_graphemes) {
        // 超出限制时才进入截断路径。
        // There are more than max_graphemes, so we need to truncate
        if max_graphemes >= 3 {
            // 预留三个位置追加 `...`，使总字素数仍不超过限制。
            // Truncate to max_graphemes - 3 and add "..." to stay within limit
            let mut truncate_graphemes = text.grapheme_indices(true);
            if let Some((truncate_byte_index, _)) = truncate_graphemes.nth(max_graphemes - 3) {
                let truncated = &text[..truncate_byte_index];
                format!("{truncated}...")
            } else {
                text.to_string()
            }
        } else {
            // 限制少于三时无法容纳省略号，直接返回前若干字素。
            // max_graphemes < 3, so just return first max_graphemes without "..."
            let truncated = &text[..byte_index];
            truncated.to_string()
        }
    } else {
        // 未溢出时原样返回，避免无谓分配和展示变化。
        // There are max_graphemes or fewer graphemes, return original text
        text.to_string()
    }
}

/// 将路径样字符串截断到指定显示宽度，尽量保留首尾片段并在中间插入一个 Unicode 省略号。
/// 单个片段仍无法容纳时，从前方截断并添加省略号。
/// Truncate a path-like string to the given display width, keeping leading and trailing segments
/// where possible and inserting a single Unicode ellipsis between them. If an individual segment
/// cannot fit, it is front-truncated with an ellipsis.
pub fn center_truncate_path(path: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(path) <= max_width {
        return path.to_string();
    }

    let sep = std::path::MAIN_SEPARATOR;
    let has_leading_sep = path.starts_with(sep);
    let has_trailing_sep = path.ends_with(sep);
    let mut raw_segments: Vec<&str> = path.split(sep).collect();
    if has_leading_sep && !raw_segments.is_empty() && raw_segments[0].is_empty() {
        raw_segments.remove(0);
    }
    if has_trailing_sep
        && !raw_segments.is_empty()
        && raw_segments.last().is_some_and(|last| last.is_empty())
    {
        raw_segments.pop();
    }

    if raw_segments.is_empty() {
        if has_leading_sep {
            let root = sep.to_string();
            if display_width(root.as_str()) <= max_width {
                return root;
            }
        }
        return "…".to_string();
    }

    struct Segment<'a> {
        original: &'a str,
        text: String,
        truncatable: bool,
        is_suffix: bool,
    }

    let assemble = |leading: bool, segments: &[Segment<'_>]| -> String {
        let mut result = String::new();
        if leading {
            result.push(sep);
        }
        for segment in segments {
            if !result.is_empty() && !result.ends_with(sep) {
                result.push(sep);
            }
            result.push_str(segment.text.as_str());
        }
        result
    };

    let front_truncate = |original: &str, allowed_width: usize| -> String {
        if allowed_width == 0 {
            return String::new();
        }
        if display_width(original) <= allowed_width {
            return original.to_string();
        }
        if allowed_width == 1 {
            return "…".to_string();
        }

        let mut kept = Vec::new();
        // 先为前导省略号预留一列，再从片段末尾按字素收集。
        let mut used_width = 1; // reserve space for leading ellipsis
        for grapheme in original.graphemes(/*is_extended*/ true).rev() {
            let grapheme_width = display_width(grapheme);
            if used_width + grapheme_width > allowed_width {
                break;
            }
            used_width += grapheme_width;
            kept.push(grapheme);
        }
        kept.reverse();
        let mut truncated = String::from("…");
        for grapheme in kept {
            truncated.push_str(grapheme);
        }
        truncated
    };

    let mut combos: Vec<(usize, usize)> = Vec::new();
    let segment_count = raw_segments.len();
    for left in 1..=segment_count {
        let min_right = if left == segment_count { 0 } else { 1 };
        for right in min_right..=(segment_count - left) {
            combos.push((left, right));
        }
    }
    let desired_suffix = if segment_count > 1 { std::cmp::min(2, segment_count - 1) } else { 0 };
    let mut prioritized: Vec<(usize, usize)> = Vec::new();
    let mut fallback: Vec<(usize, usize)> = Vec::new();
    for combo in combos {
        if combo.1 >= desired_suffix {
            prioritized.push(combo);
        } else {
            fallback.push(combo);
        }
    }
    let sort_combos = |items: &mut Vec<(usize, usize)>| {
        items.sort_by(|(left_a, right_a), (left_b, right_b)| {
            left_b
                .cmp(left_a)
                .then_with(|| right_b.cmp(right_a))
                .then_with(|| (left_b + right_b).cmp(&(left_a + right_a)))
        });
    };
    sort_combos(&mut prioritized);
    sort_combos(&mut fallback);

    let fit_segments =
        |segments: &mut Vec<Segment<'_>>, allow_front_truncate: bool| -> Option<String> {
            loop {
                let candidate = assemble(has_leading_sep, segments);
                let width = display_width(candidate.as_str());
                if width <= max_width {
                    return Some(candidate);
                }

                if !allow_front_truncate {
                    return None;
                }

                let mut indices: Vec<usize> = Vec::new();
                for (idx, seg) in segments.iter().enumerate().rev() {
                    if seg.truncatable && seg.is_suffix {
                        indices.push(idx);
                    }
                }
                for (idx, seg) in segments.iter().enumerate().rev() {
                    if seg.truncatable && !seg.is_suffix {
                        indices.push(idx);
                    }
                }

                if indices.is_empty() {
                    return None;
                }

                let mut changed = false;
                for idx in indices {
                    let original_width = display_width(segments[idx].original);
                    if original_width <= max_width && segment_count > 2 {
                        continue;
                    }
                    let seg_width = display_width(segments[idx].text.as_str());
                    let other_width = width.saturating_sub(seg_width);
                    let allowed_width = max_width.saturating_sub(other_width).max(1);
                    let new_text = front_truncate(segments[idx].original, allowed_width);
                    if new_text != segments[idx].text {
                        segments[idx].text = new_text;
                        changed = true;
                        break;
                    }
                }

                if !changed {
                    return None;
                }
            }
        };

    for (left_count, right_count) in prioritized.into_iter().chain(fallback) {
        let mut segments: Vec<Segment<'_>> = raw_segments[..left_count]
            .iter()
            .map(|seg| Segment {
                original: seg,
                text: (*seg).to_string(),
                truncatable: true,
                is_suffix: false,
            })
            .collect();

        let need_ellipsis = left_count + right_count < segment_count;
        if need_ellipsis {
            segments.push(Segment {
                original: "…",
                text: "…".to_string(),
                truncatable: false,
                is_suffix: false,
            });
        }

        if right_count > 0 {
            segments.extend(raw_segments[segment_count - right_count..].iter().map(|seg| {
                Segment {
                    original: seg,
                    text: (*seg).to_string(),
                    truncatable: true,
                    is_suffix: true,
                }
            }));
        }

        let allow_front_truncate = need_ellipsis || segment_count <= 2;
        if let Some(candidate) = fit_segments(&mut segments, allow_front_truncate) {
            return candidate;
        }
    }

    front_truncate(path, max_width)
}

/// 使用恰当的英文标点连接字符串列表。
/// 示例分别覆盖空、单项、双项以及三项列表。
/// Join a list of strings with proper English punctuation.
/// Examples:
/// - [] -> ""
/// - ["apple"] -> "apple"
/// - ["apple", "banana"] -> "apple and banana"
/// - ["apple", "banana", "cherry"] -> "apple, banana and cherry"
pub(crate) fn proper_join<T: AsRef<str>>(items: &[T]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].as_ref().to_string(),
        2 => format!("{} and {}", items[0].as_ref(), items[1].as_ref()),
        _ => {
            let last = items[items.len() - 1].as_ref();
            let mut result = String::new();

            for (i, item) in items.iter().take(items.len() - 1).enumerate() {
                if i > 0 {
                    result.push_str(", ");
                }
                result.push_str(item.as_ref());
            }

            format!("{result} and {last}")
        }
    }
}

#[cfg(test)]
#[path = "text_formatting_tests.rs"]
mod tests;
