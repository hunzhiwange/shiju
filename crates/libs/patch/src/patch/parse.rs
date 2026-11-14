//! 补丁解析模块，负责从文本中识别 apply_patch 片段并转换为结构化 hunk。

use super::{Error, Hunk, ParseResult, UpdateFileChunk};

fn strip_heredoc(input: &str) -> String {
    let trimmed = input.trim();

    let Some(idx) = trimmed.find("<<") else {
        return trimmed.to_string();
    };

    let after = &trimmed[idx + 2..];
    let after = after.trim_start();

    let (marker, rest) = if after.starts_with('\'') || after.starts_with('"') {
        let quote = after.chars().next().unwrap();
        let after_quote = &after[1..];
        let Some(end_quote) = after_quote.find(quote) else {
            return trimmed.to_string();
        };
        (&after_quote[..end_quote], &after_quote[end_quote + 1..])
    } else {
        let mut end = 0;
        for (i, ch) in after.char_indices() {
            if ch.is_whitespace() {
                break;
            }
            end = i + ch.len_utf8();
        }
        (&after[..end], &after[end..])
    };

    let rest = rest.trim_start();
    if !rest.starts_with('\n') {
        return trimmed.to_string();
    }

    let body = &rest[1..];
    let end_marker = format!("\n{}\n", marker);
    if let Some(end_idx) = body.rfind(&end_marker) {
        return body[..end_idx].to_string();
    }

    trimmed.to_string()
}

fn is_begin_marker(line: &str) -> bool {
    let s = line.trim();
    s == "*** Begin Patch" || s == "*** 开始补丁"
}

fn is_end_marker(line: &str) -> bool {
    let s = line.trim();
    s == "*** End Patch" || s == "*** 结束补丁"
}

fn parse_header(line: &str) -> Option<(&'static str, String)> {
    let s = line.trim();
    let parse = |prefix_en: &str, prefix_zh: &str| -> Option<String> {
        if let Some(rest) = s.strip_prefix(prefix_en) {
            return Some(rest.trim().trim_start_matches(':').trim().to_string());
        }
        if let Some(rest) = s.strip_prefix(prefix_zh) {
            let rest = rest.trim().trim_start_matches('：').trim_start_matches(':').trim();
            return Some(rest.to_string());
        }
        None
    };

    if let Some(path) = parse("*** Add File", "*** 添加文件") {
        return Some(("add", path));
    }
    if let Some(path) = parse("*** Delete File", "*** 删除文件") {
        return Some(("delete", path));
    }
    if let Some(path) = parse("*** Update File", "*** 更新文件") {
        return Some(("update", path));
    }
    if let Some(path) = parse("*** Move to", "*** 移动到") {
        return Some(("move", path));
    }

    None
}

fn parse_add_file_content(lines: &[String], mut index: usize) -> (String, usize) {
    let mut content = String::new();

    while index < lines.len() {
        let line = &lines[index];
        if line.trim_start().starts_with("***") {
            break;
        }
        if let Some(rest) = line.strip_prefix('+') {
            content.push_str(rest);
            content.push('\n');
        }
        index += 1;
    }

    if content.ends_with('\n') {
        content.pop();
    }

    (content, index)
}

fn parse_update_file_chunks(lines: &[String], mut index: usize) -> (Vec<UpdateFileChunk>, usize) {
    let mut chunks = Vec::new();

    while index < lines.len() {
        let line = &lines[index];
        if line.trim_start().starts_with("***") {
            break;
        }
        if !line.starts_with("@@") {
            index += 1;
            continue;
        }

        let context = line.trim_start_matches('@').trim();
        index += 1;

        let mut old_lines = Vec::new();
        let mut new_lines = Vec::new();
        let mut is_end_of_file = false;

        while index < lines.len() {
            let line = &lines[index];
            if line.trim() == "*** End of File" {
                is_end_of_file = true;
                index += 1;
                break;
            }
            if line.starts_with("@@") || line.trim_start().starts_with("***") {
                break;
            }

            if let Some(rest) = line.strip_prefix(' ') {
                old_lines.push(rest.to_string());
                new_lines.push(rest.to_string());
            } else if let Some(rest) = line.strip_prefix('-') {
                old_lines.push(rest.to_string());
            } else if let Some(rest) = line.strip_prefix('+') {
                new_lines.push(rest.to_string());
            }
            index += 1;
        }

        chunks.push(UpdateFileChunk {
            old_lines,
            new_lines,
            change_context: if context.is_empty() { None } else { Some(context.to_string()) },
            is_end_of_file: is_end_of_file.then_some(true),
        });
    }

    (chunks, index)
}

/// 解析 patch 输入。
///
/// 返回结构化结果；无法识别的输入会显式返回错误，便于调用方定位补丁格式问题。
pub fn parse_patch(patch_text: &str) -> Result<ParseResult, Error> {
    let cleaned = strip_heredoc(patch_text);
    let cleaned = cleaned.trim();
    let lines = cleaned.lines().map(|line| line.to_string()).collect::<Vec<_>>();

    let begin_idx = lines.iter().position(|line| is_begin_marker(line));
    let end_idx = lines.iter().position(|line| is_end_marker(line));

    let (Some(begin_idx), Some(end_idx)) = (begin_idx, end_idx) else {
        return Err(Error::Parse("Invalid patch format: missing Begin/End markers".to_string()));
    };

    if begin_idx >= end_idx {
        return Err(Error::Parse(
            "Invalid patch format: Begin marker after End marker".to_string(),
        ));
    }

    let mut hunks = Vec::new();
    let mut index = begin_idx + 1;

    while index < end_idx {
        let Some((kind, path)) = parse_header(&lines[index]) else {
            index += 1;
            continue;
        };

        match kind {
            "add" => {
                let (contents, next_index) = parse_add_file_content(&lines, index + 1);
                hunks.push(Hunk::Add { path, contents });
                index = next_index;
            }
            "delete" => {
                hunks.push(Hunk::Delete { path });
                index += 1;
            }
            "update" => {
                let mut move_path = None;
                let mut next = index + 1;

                if next < end_idx
                    && let Some(("move", destination)) = parse_header(&lines[next])
                {
                    move_path = Some(destination);
                    next += 1;
                }

                let (chunks, next_index) = parse_update_file_chunks(&lines, next);
                hunks.push(Hunk::Update { path, move_path, chunks });
                index = next_index;
            }
            _ => {
                index += 1;
            }
        }
    }

    Ok(ParseResult { hunks })
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod parse_tests;
