//! Pi template metadata conversion; argument expansion belongs to the native app.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PiPromptTemplateSummary {
    pub description: Option<String>,
    pub argument_hint: Option<String>,
}

struct Document<'a> {
    lines: Vec<&'a str>,
    newline: &'static str,
    closing: usize,
}

fn newline(content: &str) -> &'static str {
    if content.contains("\r\n") { "\r\n" } else { "\n" }
}

fn parse(content: &str) -> Option<Document<'_>> {
    // CRLF is one separator; bare CR and LF are also separators.
    let mut lines = Vec::new();
    let mut start = 0;
    let bytes = content.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if matches!(bytes[index], b'\r' | b'\n') {
            lines.push(&content[start..index]);
            if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
                index += 1;
            }
            start = index + 1;
        }
        index += 1;
    }
    lines.push(&content[start..]);
    if lines.first()?.trim() != "---" { return None; }
    let closing = lines.iter().skip(1).position(|line| line.trim() == "---")? + 1;
    Some(Document { lines, newline: newline(content), closing })
}

fn clean(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        return serde_json::from_str::<String>(value)
            .unwrap_or_else(|_| value[1..value.len() - 1].to_string())
            .trim().to_string();
    }
    if value.len() >= 2 && value.starts_with('\'') && value.ends_with('\'') {
        return value[1..value.len() - 1].replace("''", "'").trim().to_string();
    }
    value.to_string()
}

fn field(document: &Document<'_>, name: &str) -> Option<(usize, usize, String)> {
    for start in 1..document.closing {
        let Some((key, raw)) = document.lines[start].split_once(':') else { continue; };
        let key = key.trim_end();
        if key.is_empty() || !key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-') || key != name {
            continue;
        }
        let inline = raw.trim();
        let mut end = start + 1;
        let value = if matches!(inline, "" | "|" | ">") {
            while end < document.closing && (document.lines[end].trim().is_empty()
                || document.lines[end].starts_with(char::is_whitespace)) {
                end += 1;
            }
            document.lines[start + 1..end].iter().map(|line| line.trim())
                .filter(|line| !line.is_empty()).collect::<Vec<_>>()
                .join(if inline == "|" { "\n" } else { " " }).trim().to_string()
        } else { clean(inline) };
        return Some((start, end, value));
    }
    None
}

pub fn get_pi_prompt_template_description(content: &str) -> Option<String> {
    let document = parse(content)?;
    let (_, _, value) = field(&document, "description")?;
    if value.is_empty() { None } else { Some(value) }
}

pub fn strip_pi_prompt_template_description(content: &str) -> String {
    let Some(mut document) = parse(content) else { return content.to_string(); };
    let Some((start, end, _)) = field(&document, "description") else { return content.to_string(); };
    document.lines.drain(start..end);
    let closing = document.closing - (end - start);
    if document.lines[1..closing].iter().any(|line| !line.trim().is_empty()) {
        document.lines.join(document.newline)
    } else {
        document.lines[closing + 1..].join(document.newline)
    }
}

pub fn set_pi_prompt_template_description(content: &str, description: &str) -> String {
    let stripped = strip_pi_prompt_template_description(content);
    let description = description.trim();
    if description.is_empty() { return stripped; }
    let line = format!("description: {}", serde_json::Value::String(description.to_string()));
    if let Some(mut document) = parse(&stripped) {
        document.lines.insert(1, &line);
        return document.lines.join(document.newline);
    }
    let newline = newline(content);
    format!("---{newline}{line}{newline}---{newline}{stripped}")
}

/// Count UTF-16 code units as in JavaScript. A split surrogate is represented
/// by U+FFFD because Rust strings cannot contain an unpaired surrogate.
fn truncate_summary(value: &str) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let units: Vec<_> = compact.encode_utf16().collect();
    if units.len() <= 140 { return compact; }
    format!("{}…", String::from_utf16_lossy(&units[..139]).trim_end())
}

pub fn get_pi_prompt_template_summary(content: &str) -> PiPromptTemplateSummary {
    let Some(document) = parse(content) else { return PiPromptTemplateSummary::default(); };
    PiPromptTemplateSummary {
        description: field(&document, "description").map(|(_, _, value)| value)
            .filter(|value| !value.is_empty()).map(|value| truncate_summary(&value)),
        argument_hint: field(&document, "argument-hint").map(|(_, _, value)| value)
            .filter(|value| !value.is_empty()),
    }
}
