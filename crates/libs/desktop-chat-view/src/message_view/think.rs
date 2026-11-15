//! 思考与可见文本的既有分离规则。
pub fn split_think(raw: &str) -> (Vec<String>, String, bool) {
    let mut thinks = Vec::new();
    let mut visible = String::new();
    let mut rest = raw;
    let mut thinking_open = false;

    fn normalize_visible_text(s: &str) -> String {
        if !s.contains('\r')
            && !s.contains("\n\n\n")
            && !s.contains(" \n")
            && !s.contains("\t\n")
            && !s.contains("\n \n")
            && !s.contains("\n\t\n")
        {
            return s.to_string();
        }
        let mut out = String::with_capacity(s.len());
        let mut prev_blank = false;
        let mut in_fence = false;
        for raw_line in s.split('\n') {
            let line = raw_line.trim_end_matches('\r');
            if line.trim_start().starts_with("```") {
                if !in_fence && out.ends_with("\n\n") {
                    out.pop();
                }
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
        while out.ends_with('\n') {
            out.pop();
        }
        out
    }

    fn parse_tool_block(s: &str) -> Option<usize> {
        if !s.starts_with("tool ") {
            return None;
        }
        let Some(line_end) = s.find('\n') else {
            return None;
        };
        let mut i = line_end + 1;
        let mut buf = String::new();
        for _ in 0..64 {
            if i >= s.len() {
                break;
            }
            let next_end = s[i..].find('\n').map(|x| i + x).unwrap_or(s.len());
            let line = &s[i..next_end];
            if !buf.is_empty() {
                buf.push('\n');
            }
            buf.push_str(line);
            if serde_json::from_str::<serde_json::Value>(buf.trim()).is_ok() {
                return Some(if next_end < s.len() { next_end + 1 } else { next_end });
            }
            if next_end >= s.len() {
                break;
            }
            i = next_end + 1;
        }
        None
    }

    fn is_think_tag_remainder(remainder: &str) -> bool {
        if remainder.is_empty() || remainder.chars().next().is_some_and(char::is_whitespace) {
            return true;
        }
        remainder.strip_prefix("ing").is_some_and(|after| {
            after.is_empty() || after.chars().next().is_some_and(char::is_whitespace)
        })
    }

    fn find_think_open_tag(s: &str) -> Option<(usize, usize)> {
        let mut search_from = 0usize;
        while let Some(pos_rel) = s[search_from..].find("<think") {
            let pos = search_from + pos_rel;
            if s[pos..].starts_with("</think") {
                search_from = pos + 1;
                continue;
            }
            let Some(tag_end_rel) = s[pos..].find('>') else {
                return None;
            };
            let tag_end = pos + tag_end_rel;
            let tag = &s[pos + "<think".len()..tag_end];
            if is_think_tag_remainder(tag) {
                return Some((pos, tag_end_rel + 1));
            }
            search_from = pos + 1;
        }
        None
    }

    fn find_think_close_tag(s: &str) -> Option<(usize, usize)> {
        let mut search_from = 0usize;
        while let Some(pos_rel) = s[search_from..].find("</think") {
            let pos = search_from + pos_rel;
            let Some(tag_end_rel) = s[pos..].find('>') else {
                return None;
            };
            let tag_end = pos + tag_end_rel;
            let tag = &s[pos + "</think".len()..tag_end];
            if is_think_tag_remainder(tag) {
                return Some((pos, tag_end_rel + 1));
            }
            search_from = pos + 1;
        }
        None
    }

    fn find_reasoning_marker(s: &str) -> Option<(usize, usize)> {
        for (pos, _) in s.match_indices("[reasoning]") {
            if pos > 0 && !s[..pos].ends_with('\n') {
                continue;
            }

            let suffix = &s[pos + "[reasoning]".len()..];
            let marker_len = if suffix.starts_with("\r\n") {
                "[reasoning]".len() + 2
            } else if suffix.starts_with('\n') {
                "[reasoning]".len() + 1
            } else if suffix.is_empty() {
                "[reasoning]".len()
            } else {
                continue;
            };
            return Some((pos, marker_len));
        }
        None
    }

    fn append_visible_without_orphan_close_tags(out: &mut String, mut s: &str) {
        while let Some((start, tag_len)) = find_think_close_tag(s) {
            out.push_str(&s[..start]);
            s = &s[start + tag_len..];
        }
        out.push_str(s);
    }

    fn find_tool_start(s: &str) -> Option<usize> {
        if s.starts_with("tool ") {
            return Some(0);
        }
        s.find("\ntool ").map(|i| i + 1)
    }

    fn strip_tool_blocks(s: &str) -> String {
        let mut out = String::new();
        let mut rest = s;
        loop {
            let Some(pos) = find_tool_start(rest) else {
                out.push_str(rest);
                break;
            };
            out.push_str(&rest[..pos]);
            let tool = &rest[pos..];
            let Some(consumed) = parse_tool_block(tool) else {
                out.push_str(tool);
                break;
            };
            rest = &tool[consumed..];
        }
        out
    }

    loop {
        let think_open = find_think_open_tag(rest);
        let reasoning_marker = find_reasoning_marker(rest);
        let next_start = match (think_open, reasoning_marker) {
            (None, None) => None,
            (Some(tag), None) => Some((tag.0, tag.1, false)),
            (None, Some(marker)) => Some((marker.0, marker.1, true)),
            (Some(tag), Some(marker)) => Some(if tag.0 <= marker.0 {
                (tag.0, tag.1, false)
            } else {
                (marker.0, marker.1, true)
            }),
        };
        let Some((start, open_len, is_reasoning_marker)) = next_start else {
            append_visible_without_orphan_close_tags(&mut visible, rest);
            break;
        };
        append_visible_without_orphan_close_tags(&mut visible, &rest[..start]);
        rest = &rest[start + open_len..];
        if is_reasoning_marker {
            if let Some(tool_start) = find_tool_start(rest) {
                thinks.push(rest[..tool_start].to_string());
                rest = &rest[tool_start..];
                append_visible_without_orphan_close_tags(&mut visible, rest);
            } else if !rest.is_empty() {
                thinks.push(rest.to_string());
            }
            break;
        }
        let Some((end, close_tag_len)) = find_think_close_tag(rest) else {
            if let Some(tool_start) = find_tool_start(rest) {
                thinks.push(rest[..tool_start].to_string());
                thinking_open = true;
                rest = &rest[tool_start..];
                continue;
            }
            thinking_open = true;
            if !rest.is_empty() {
                thinks.push(rest.to_string());
            }
            break;
        };
        let think_chunk = &rest[..end];
        if let Some(tool_start) = find_tool_start(think_chunk) {
            thinks.push(think_chunk[..tool_start].to_string());
        } else {
            thinks.push(think_chunk.to_string());
        }
        rest = &rest[end + close_tag_len..];
    }

    let visible = strip_tool_blocks(&visible);
    (thinks, normalize_visible_text(&visible), thinking_open)
}
