//! 补丁替换推导模块，负责根据上下文块计算新文件内容和统一 diff 预览。

use std::fs;
use std::path::Path;

use super::{ApplyPatchFileUpdate, Error, UpdateFileChunk};

fn normalize_unicode(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => '\'',
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => '"',
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}' => '-',
            '\u{2026}' => '.',
            '\u{00A0}' => ' ',
            _ => ch,
        })
        .collect()
}

fn try_match(
    lines: &[String],
    pattern: &[String],
    start_index: usize,
    compare: impl Fn(&str, &str) -> bool,
    eof: bool,
) -> Option<usize> {
    if pattern.is_empty() || lines.len() < pattern.len() {
        return None;
    }

    if eof && lines.len() >= pattern.len() {
        let from_end = lines.len() - pattern.len();
        if from_end >= start_index
            && pattern
                .iter()
                .enumerate()
                .all(|(offset, expected)| compare(&lines[from_end + offset], expected.as_str()))
        {
            return Some(from_end);
        }
    }

    let last_start = lines.len() - pattern.len();
    if start_index > last_start {
        return None;
    }

    for index in start_index..=last_start {
        let ok = pattern
            .iter()
            .enumerate()
            .all(|(offset, expected)| compare(&lines[index + offset], expected.as_str()));
        if ok {
            return Some(index);
        }
    }

    None
}

fn seek_sequence(
    lines: &[String],
    pattern: &[String],
    start_index: usize,
    eof: bool,
) -> Option<usize> {
    let exact = try_match(lines, pattern, start_index, |left, right| left == right, eof);
    if exact.is_some() {
        return exact;
    }

    let rstrip = try_match(
        lines,
        pattern,
        start_index,
        |left, right| left.trim_end() == right.trim_end(),
        eof,
    );
    if rstrip.is_some() {
        return rstrip;
    }

    let trim =
        try_match(lines, pattern, start_index, |left, right| left.trim() == right.trim(), eof);
    if trim.is_some() {
        return trim;
    }

    try_match(
        lines,
        pattern,
        start_index,
        |left, right| normalize_unicode(left.trim()) == normalize_unicode(right.trim()),
        eof,
    )
}

fn compute_replacements(
    original_lines: &[String],
    file_path: &Path,
    chunks: &[UpdateFileChunk],
) -> Result<Vec<(usize, usize, Vec<String>)>, Error> {
    let mut replacements = Vec::new();
    let mut line_index = 0usize;

    for chunk in chunks {
        if let Some(context) = &chunk.change_context {
            let context_pattern = vec![context.to_string()];
            if let Some(index) = seek_sequence(original_lines, &context_pattern, line_index, false)
            {
                line_index = index + 1;
            }
        }

        if chunk.old_lines.is_empty() {
            let insertion_idx = std::cmp::min(line_index, original_lines.len());
            replacements.push((insertion_idx, 0, chunk.new_lines.clone()));
            continue;
        }

        let mut pattern = chunk.old_lines.clone();
        let mut new_slice = chunk.new_lines.clone();
        let eof = chunk.is_end_of_file.unwrap_or(false);

        let mut found = seek_sequence(original_lines, &pattern, line_index, eof);
        if found.is_none()
            && let Some(last) = pattern.last()
            && last.is_empty()
        {
            pattern.pop();
            if let Some(last_new) = new_slice.last()
                && last_new.is_empty()
            {
                new_slice.pop();
            }
            found = seek_sequence(original_lines, &pattern, line_index, eof);
        }

        let Some(found) = found else {
            return Err(Error::ComputeReplacements(format!(
                "Failed to find expected lines in {}.\n这通常表示文件内容已变化，或补丁上下文不够唯一。\n建议：先重新读取该文件，再在补丁里加入更多未修改的上下文行（以空格开头的行），或使用更具体的 @@ 头来定位。\n期望匹配的旧内容：\n{}",
                file_path.display(),
                chunk.old_lines.join("\n")
            )));
        };

        replacements.push((found, pattern.len(), new_slice));
        line_index = found + pattern.len();
    }

    replacements.sort_by_key(|(index, _, _)| *index);
    Ok(replacements)
}

fn apply_replacements(
    lines: &[String],
    replacements: &[(usize, usize, Vec<String>)],
) -> Vec<String> {
    let mut result = lines.to_vec();

    for (start_idx, old_len, new_segment) in replacements.iter().rev() {
        result.splice(*start_idx..*start_idx + *old_len, new_segment.clone());
    }

    result
}

fn generate_unified_diff(old_content: &str, new_content: &str) -> String {
    let old_lines = old_content.split('\n').collect::<Vec<_>>();
    let new_lines = new_content.split('\n').collect::<Vec<_>>();

    let mut diff = String::from("@@ -1 +1 @@\n");
    let mut has_changes = false;

    for index in 0..old_lines.len().max(new_lines.len()) {
        let old_line = old_lines.get(index).copied().unwrap_or("");
        let new_line = new_lines.get(index).copied().unwrap_or("");

        if old_line != new_line {
            if !old_line.is_empty() {
                diff.push('-');
                diff.push_str(old_line);
                diff.push('\n');
            }
            if !new_line.is_empty() {
                diff.push('+');
                diff.push_str(new_line);
                diff.push('\n');
            }
            has_changes = true;
        } else if !old_line.is_empty() {
            diff.push(' ');
            diff.push_str(old_line);
            diff.push('\n');
        }
    }

    if has_changes { diff } else { String::new() }
}

/// 提供 derive new contents from chunks 功能。
///
/// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
pub fn derive_new_contents_from_chunks(
    file_path: &Path,
    chunks: &[UpdateFileChunk],
) -> Result<ApplyPatchFileUpdate, Error> {
    let original_content = fs::read_to_string(file_path)?;
    let original_lines = original_content.lines().map(|line| line.to_string()).collect::<Vec<_>>();

    let replacements = compute_replacements(&original_lines, file_path, chunks)?;
    let mut new_lines = apply_replacements(&original_lines, &replacements);

    if new_lines.last().map(|line| !line.is_empty()).unwrap_or(true) {
        new_lines.push(String::new());
    }

    let new_content = new_lines.join("\n");
    let unified_diff = generate_unified_diff(&original_content, &new_content);

    Ok(ApplyPatchFileUpdate { unified_diff, content: new_content })
}

#[cfg(test)]
#[path = "replace_tests.rs"]
mod replace_tests;
