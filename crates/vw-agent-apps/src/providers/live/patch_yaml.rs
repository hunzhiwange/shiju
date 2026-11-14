//! Surgical YAML mapping edits adapted from OMP's block-mapping editor.
//! Callers own schema validation, file authorization, locks and commit.
use serde_json::Value;
use serde_yaml::{Mapping, Value as Yaml};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchError {
    /// The document cannot be safely spliced. Contains no configuration payload.
    ManualEdit,
    InvalidNode,
}

#[derive(Debug, Clone, Copy)]
pub enum ProviderSection {
    Providers,
    CustomProvider,
}

impl ProviderSection {
    fn key(self) -> &'static str {
        match self {
            Self::Providers => "providers",
            Self::CustomProvider => "custom_provider",
        }
    }
}

/// Replace/remove one literal provider key. Missing input starts an empty mapping.
/// Block section headers may be created or normalized from null. Only the target
/// node is emitted; surrounding comments, sibling nodes and root bytes survive.
/// Nonempty flow sections, references, tags and ambiguous layouts require manual edit.
/// This prepares bytes only and does not establish a successful live switch.
pub fn patch_provider(
    pre: Option<&[u8]>,
    section: ProviderSection,
    id: &str,
    replacement: Option<&Value>,
) -> Result<Vec<u8>, PatchError> {
    if replacement.is_some_and(|value| !value.is_object()) {
        return Err(PatchError::InvalidNode);
    }
    let replacement =
        replacement.map(serde_yaml::to_value).transpose().map_err(|_| PatchError::InvalidNode)?;
    patch_entry(pre, section.key(), id, replacement.as_ref())
}

#[cfg(target_os = "macos")]
pub(crate) fn patch_mcp_server(
    pre: Option<&[u8]>,
    id: &str,
    replacement: Option<Mapping>,
) -> Result<Vec<u8>, PatchError> {
    patch_entry(pre, "mcp_servers", id, replacement.map(Yaml::Mapping).as_ref())
}

/// Hermes owns the two supported memory switches; no arbitrary section path.
pub(crate) fn patch_hermes_memory_enabled(
    pre: Option<&[u8]>,
    kind: vw_api_types::hermes_memory::HermesMemoryKind,
    enabled: bool,
) -> Result<Vec<u8>, PatchError> {
    use vw_api_types::hermes_memory::HermesMemoryKind;
    let key = match kind {
        HermesMemoryKind::Memory => "memory_enabled",
        HermesMemoryKind::User => "user_profile_enabled",
    };
    patch_entry(pre, "memory", key, Some(&Yaml::Bool(enabled)))
}

fn patch_entry(
    pre: Option<&[u8]>,
    section: &str,
    id: &str,
    replacement: Option<&Yaml>,
) -> Result<Vec<u8>, PatchError> {
    let bytes = pre.unwrap_or_default();
    if bytes.len() > 1024 * 1024 {
        return Err(PatchError::ManualEdit);
    }
    if id.is_empty() {
        return Err(PatchError::InvalidNode);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| PatchError::ManualEdit)?;
    let output = edit_entry(text, section, id, replacement)?;
    if output.len() > 1024 * 1024 {
        return Err(PatchError::ManualEdit);
    }
    Ok(output.into_bytes())
}

fn manual_edit() -> PatchError {
    PatchError::ManualEdit
}

fn parse(text: &str) -> Result<Yaml, PatchError> {
    let value: Yaml = serde_yaml::from_str(text).map_err(|_| manual_edit())?;
    match value {
        Yaml::Null => Ok(Yaml::Mapping(Mapping::new())),
        Yaml::Mapping(_) => Ok(value),
        _ => Err(manual_edit()),
    }
}

fn section_mapping(document: &Yaml, section: &str) -> Result<Mapping, PatchError> {
    match document.get(section) {
        None | Some(Yaml::Null) => Ok(Mapping::new()),
        Some(Yaml::Mapping(nodes)) if nodes.keys().all(|key| key.as_str().is_some()) => {
            Ok(nodes.clone())
        }
        _ => Err(manual_edit()),
    }
}
fn edit_entry(
    text: &str,
    section: &str,
    id: &str,
    replacement: Option<&Yaml>,
) -> Result<String, PatchError> {
    if text.contains('\t') || text.lines().any(has_yaml_reference_tokens) {
        return Err(manual_edit());
    }
    let document = parse(text).map_err(|_| manual_edit())?;
    let mut intended = section_mapping(&document, section)?;
    if let Some(value) = replacement {
        intended.insert(id.into(), value.clone());
    } else {
        intended.remove(Yaml::from(id));
    }
    if replacement.is_none() && !section_mapping(&document, section)?.contains_key(Yaml::from(id)) {
        return Ok(text.to_string());
    }
    let mut expected = document.clone();
    expected
        .as_mapping_mut()
        .ok_or_else(manual_edit)?
        .insert(section.into(), Yaml::Mapping(intended.clone()));
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut offsets = Vec::new();
    let mut offset = 0;
    for line in &lines {
        offsets.push(offset);
        offset += line.len();
    }
    offsets.push(offset);
    let mut header = None;
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with(' ') || line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if line.trim() == "---" {
            continue;
        }
        if line.trim() == "..." {
            return Err(manual_edit());
        }
        if mapping_key(line)? == section {
            header = Some(i);
        }
    }
    let mut output = text.to_string();
    let mut null_span = None;
    let (start, end, indent) = if let Some(header) = header {
        let line = lines[header];
        let (_, tail) = line.split_once(':').ok_or_else(manual_edit)?;
        let value = tail.split('#').next().unwrap_or_default().trim();
        if !matches!(value, "" | "null" | "~" | "{}") {
            return Err(manual_edit());
        }
        if !value.is_empty() {
            // Remove only the null or empty map token, preserving header whitespace and comments.
            let start = offsets[header] + line.len() - tail.trim_start().len();
            null_span = Some(start..start + value.len());
        }
        let end = ((header + 1)..lines.len())
            .find(|&i| {
                !lines[i].starts_with(' ')
                    && !lines[i].trim().is_empty()
                    && !lines[i].trim_start().starts_with('#')
            })
            .unwrap_or(lines.len());
        let mut indent = None;
        let mut children = Vec::new();
        for (i, line) in lines.iter().enumerate().take(end).skip(header + 1) {
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            reject_yaml_tokens(line)?;
            let spaces = line.len() - line.trim_start_matches(' ').len();
            let child_indent = *indent.get_or_insert(spaces);
            if spaces < child_indent {
                return Err(manual_edit());
            }
            if spaces == child_indent {
                if line.trim() == "{}" {
                    children.push((i, String::new()));
                    continue;
                }
                let key = mapping_key(line.trim_start())?;
                if key == "<<" {
                    return Err(manual_edit());
                }
                children.push((i, key));
            }
        }
        let indent = indent.unwrap_or(2);
        let mut start = end;
        while start > header + 1
            && (lines[start - 1].trim().is_empty()
                || lines[start - 1].trim_start().starts_with('#'))
        {
            start -= 1;
        }
        let mut stop = start;
        if let Some(index) = children.iter().position(|(_, key)| key == id || key.is_empty()) {
            start = children[index].0;
            stop = children.get(index + 1).map(|c| c.0).unwrap_or(end);
            while stop > start + 1
                && (lines[stop - 1].trim().is_empty()
                    || lines[stop - 1].trim_start().starts_with('#'))
            {
                stop -= 1;
            }
            // A peer or root comment interleaved with the node has ambiguous
            // ownership. Replacing the entire span would silently discard it.
            if lines[start + 1..stop].iter().any(|line| {
                let trimmed = line.trim_start_matches(' ');
                trimmed.starts_with('#') && line.len() - trimmed.len() <= indent
            }) {
                return Err(manual_edit());
            }
        }
        (offsets[start], offsets[stop], indent)
    } else {
        if replacement.is_none() {
            return Ok(output);
        }
        if !output.is_empty() && !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str(&format!("{section}:\n"));
        (output.len(), output.len(), 2)
    };
    let mut emitted = if intended.is_empty() {
        format!("{}{{}}\n", " ".repeat(indent))
    } else {
        emit_entry(id, replacement, indent)?
    };
    if start == end && !output[..start].is_empty() && !output[..start].ends_with('\n') {
        emitted.insert(0, '\n');
    }
    output.replace_range(start..end, &emitted);
    if let Some(span) = null_span {
        output.replace_range(span, "");
    }
    verify_edit(&output, &expected)?;
    Ok(output)
}

fn emit_entry(id: &str, value: Option<&Yaml>, indent: usize) -> Result<String, PatchError> {
    let Some(value) = value else {
        return Ok(String::new());
    };
    let yaml = serde_yaml::to_string(&std::collections::BTreeMap::from([(id, value)]))
        .map_err(|_| PatchError::InvalidNode)?;
    Ok(yaml.lines().map(|line| format!("{}{line}\n", " ".repeat(indent))).collect())
}

fn verify_edit(text: &str, intended: &Yaml) -> Result<(), PatchError> {
    if parse(text)? != *intended {
        return Err(manual_edit());
    }
    Ok(())
}

fn mapping_key(line: &str) -> Result<String, PatchError> {
    // Find the mapping delimiter outside a quoted key (provider IDs are opaque).
    let mut quote = None;
    let mut escaped = false;
    let mut colon = None;
    for (index, c) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quote == Some('"') && c == '\\' {
            escaped = true;
            continue;
        }
        if quote == Some(c) {
            quote = None;
            continue;
        }
        if quote.is_some() {
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            ':' if line[index + 1..].chars().next().is_none_or(char::is_whitespace) => {
                colon = Some(index);
                break;
            }
            _ => {}
        }
    }
    let colon = colon.ok_or_else(manual_edit)?;
    let header = format!("{}: null", &line[..colon]);
    let mapping: serde_yaml::Mapping = serde_yaml::from_str(&header).map_err(|_| manual_edit())?;
    mapping
        .keys()
        .next()
        .and_then(serde_yaml::Value::as_str)
        .map(str::to_string)
        .ok_or_else(manual_edit)
}

/// Anchors, aliases, tags and merge keys can't be edited as text without breaking a
/// reference elsewhere; refuse them before writing.
fn reject_yaml_tokens(line: &str) -> Result<(), PatchError> {
    if has_yaml_reference_tokens(line) {
        return Err(manual_edit());
    }
    Ok(())
}

/// Whether a YAML line has an anchor, alias, tag or merge key outside quotes and comments.
/// `&`, `*` and `!` are only YAML indicators where a node starts: inside a plain scalar
/// (`name: Search & Fetch`, which serde_yaml and Bun both write unquoted) they are
/// ordinary text.
pub(crate) fn has_yaml_reference_tokens(line: &str) -> bool {
    let mut quote = None;
    let mut escaped = false;
    // A node may start here: line start, after `:`, a flow opener or `,`, and after a
    // `- `/`? ` entry marker that itself sits where a node starts. Whitespace keeps the
    // state; any other character is node content.
    let mut node_start = true;
    // `#` starts a comment only at line start or after whitespace.
    let mut after_space = true;
    let mut previous = [' '; 2];
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if escaped {
            escaped = false;
            continue;
        }
        if quote == Some('"') && c == '\\' {
            escaped = true;
            continue;
        }
        if quote == Some('\'') && c == '\'' && chars.peek() == Some(&'\'') {
            // `''` is an escaped quote inside a single-quoted scalar.
            chars.next();
            continue;
        }
        if quote == Some(c) {
            quote = None;
            node_start = false;
            after_space = false;
            continue;
        }
        if quote.is_some() {
            continue;
        }
        if c == ':' && previous == ['<', '<'] {
            return true;
        }
        match c {
            '#' if after_space => break,
            '"' | '\'' if node_start => quote = Some(c),
            '&' | '*' | '!' if node_start => return true,
            _ => {}
        }
        previous = [previous[1], c];
        after_space = c.is_whitespace();
        if !c.is_whitespace() {
            let entry_marker = node_start
                && matches!(c, '-' | '?')
                && chars.peek().is_none_or(|next| next.is_whitespace());
            node_start = entry_marker || matches!(c, ':' | '[' | '{' | ',');
        }
    }
    false
}
