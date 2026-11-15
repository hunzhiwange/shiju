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
