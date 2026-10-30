//! UTF-8 byte-budgeted Markdown chunks. Code lines and table rows are never rewritten.

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag};

/// Source table ranges exclude code blocks and retain the original Markdown bytes.
pub(crate) fn markdown_table_ranges(text: &str) -> Vec<Range<usize>> {
    Parser::new_ext(text, Options::ENABLE_TABLES)
        .into_offset_iter()
        .filter_map(|(event, range)| matches!(event, Event::Start(Tag::Table(_))).then_some(range))
        .collect()
}

/// Protect fenced/indented blocks and inline code during platform normalization.
pub(crate) fn markdown_code_ranges(text: &str) -> Vec<Range<usize>> {
    Parser::new_ext(text, Options::ENABLE_TABLES)
        .into_offset_iter()
        .filter_map(|(event, range)| {
            matches!(event, Event::Start(Tag::CodeBlock(_)) | Event::Code(_)).then_some(range)
        })
        .collect()
}

#[derive(Clone, Copy)]
struct Fence<'a> {
    marker: u8,
    length: usize,
    opening: &'a str,
    indentation: &'a str,
}

fn fence_start(line: &str) -> Option<Fence<'_>> {
    let stripped = line.trim_start_matches(' ');
    let indent = line.len() - stripped.len();
    if indent > 3 {
        return None;
    }
    let marker = *stripped.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let length = stripped.bytes().take_while(|byte| *byte == marker).count();
    if length < 3 || (marker == b'`' && stripped[length..].contains('`')) {
        return None;
    }
    Some(Fence { marker, length, opening: line, indentation: &line[..indent] })
}

fn fence_end(line: &str, fence: Fence<'_>) -> bool {
    let stripped = line.trim_start_matches(' ');
    if line.len() - stripped.len() > 3 {
        return false;
    }
    let length = stripped.bytes().take_while(|byte| *byte == fence.marker).count();
    length >= fence.length && stripped[length..].trim().is_empty()
}

fn flush(current: &mut String, chunks: &mut Vec<String>) {
    if !current.is_empty() {
        chunks.push(std::mem::take(current));
    }
}

fn append_unit(unit: &str, budget: usize, current: &mut String, chunks: &mut Vec<String>) {
    if current.len() + unit.len() > budget {
        flush(current, chunks);
    }
    current.push_str(unit);
}

fn append_plain(
    text: &str,
    budget: usize,
    current: &mut String,
    chunks: &mut Vec<String>,
) -> anyhow::Result<()> {
    for paragraph in text.split_inclusive("\n\n") {
        if paragraph.len() <= budget {
            append_unit(paragraph, budget, current, chunks);
            continue;
        }
        for line in paragraph.split_inclusive('\n') {
            if line.len() <= budget {
                append_unit(line, budget, current, chunks);
                continue;
            }
            for character in line.chars() {
                anyhow::ensure!(
                    character.len_utf8() <= budget,
                    "Markdown byte budget cannot fit a Unicode character"
                );
                if current.len() + character.len_utf8() > budget {
                    flush(current, chunks);
                }
                current.push(character);
            }
        }
    }
    Ok(())
}

fn append_code(
    lines: &[&str],
    fence: Fence<'_>,
    closed: bool,
    budget: usize,
    current: &mut String,
    chunks: &mut Vec<String>,
) -> anyhow::Result<()> {
    let marker = (fence.marker as char).to_string().repeat(fence.length);
    let closing = format!("{}{marker}", fence.indentation);
    let mut whole = lines.concat();
    if !closed {
        if !whole.ends_with('\n') {
            whole.push('\n');
        }
        whole.push_str(&closing);
    }
    if whole.len() <= budget {
        append_unit(&whole, budget, current, chunks);
        return Ok(());
    }
    flush(current, chunks);
    let mut opening = fence.opening.to_string();
    if !opening.ends_with('\n') {
        opening.push('\n');
    }
    let end = lines.len() - usize::from(closed);
    let mut part = opening.clone();
    for line in &lines[1..end] {
        let newline = usize::from(!line.ends_with('\n'));
        anyhow::ensure!(
            opening.len() + line.len() + newline + closing.len() <= budget,
            "Markdown code line exceeds byte budget"
        );
        if part.len() + line.len() + newline + closing.len() > budget {
            part.push_str(&closing);
            chunks.push(part);
            part = opening.clone();
        }
        part.push_str(line);
    }
    if !part.ends_with('\n') {
        part.push('\n');
    }
    let final_closing = if closed { lines[lines.len() - 1] } else { &closing };
    if part.len() + final_closing.len() <= budget {
        part.push_str(final_closing);
    } else {
        // A source closing fence may be longer than the synthetic one.
        part.push_str(&closing);
        chunks.push(part);
        part = format!("{opening}{final_closing}");
    }
    anyhow::ensure!(part.len() <= budget, "Markdown fence exceeds byte budget");
    append_unit(&part, budget, current, chunks);
    Ok(())
}

fn append_table(
    table: &str,
    budget: usize,
    current: &mut String,
    chunks: &mut Vec<String>,
) -> anyhow::Result<()> {
    if table.len() <= budget {
        append_unit(table, budget, current, chunks);
        return Ok(());
    }
    let lines = table.split_inclusive('\n').collect::<Vec<_>>();
    anyhow::ensure!(lines.len() >= 2, "Markdown table header is incomplete");
    let header = lines[..2].concat();
    anyhow::ensure!(header.len() <= budget, "Markdown table header exceeds byte budget");
    flush(current, chunks);
    let mut part = header.clone();
    for row in &lines[2..] {
        anyhow::ensure!(
            header.len() + row.len() <= budget,
            "Markdown table row exceeds byte budget"
        );
        if part.len() + row.len() > budget {
            chunks.push(part);
            part = header.clone();
        }
        part.push_str(row);
    }
    append_unit(&part, budget, current, chunks);
    Ok(())
}

/// Split the complete normalized reply before sending any chunk to a platform.
/// Callers must build and preflight every final payload (including JSON escaping)
/// before the first network request; this budget covers only Markdown bytes.
pub(crate) fn split_markdown(text: &str, max_bytes: usize) -> anyhow::Result<Vec<String>> {
    anyhow::ensure!(max_bytes > 0, "Markdown byte budget must be positive");
    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    let tables = markdown_table_ranges(text);
    let mut offset = 0;
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut index = 0;
    while index < lines.len() {
        if let Some(table) = tables.iter().find(|range| range.start == offset) {
            append_table(&text[table.clone()], max_bytes, &mut current, &mut chunks)?;
            while index < lines.len() && offset < table.end {
                offset += lines[index].len();
                index += 1;
            }
            continue;
        }
        let section_start = index;
        if let Some(fence) = fence_start(lines[index]) {
            let start = index;
            index += 1;
            while index < lines.len() && !fence_end(lines[index], fence) {
                index += 1;
            }
            let closed = index < lines.len();
            if closed {
                index += 1;
            }
            append_code(&lines[start..index], fence, closed, max_bytes, &mut current, &mut chunks)?;
        } else {
            let start = index;
            index += 1;
            let mut next_offset = offset + lines[start].len();
            while index < lines.len()
                && fence_start(lines[index]).is_none()
                && !tables.iter().any(|range| range.start == next_offset)
            {
                next_offset += lines[index].len();
                index += 1;
            }
            append_plain(&lines[start..index].concat(), max_bytes, &mut current, &mut chunks)?;
        }
        offset += lines[section_start..index].iter().map(|line| line.len()).sum::<usize>();
    }
    flush(&mut current, &mut chunks);
    Ok(chunks)
}

#[cfg(test)]
#[path = "markdown_chunk_tests.rs"]
mod tests;
