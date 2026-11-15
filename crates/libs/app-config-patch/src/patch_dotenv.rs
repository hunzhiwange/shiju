//! Named dotenv edits; authorization and transactional file writes belong to callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchError {
    InvalidInput,
}

#[derive(Default)]
pub struct DotenvPatch {
    pub clear: Option<fn(&str) -> bool>,
    /// Raw dotenv values, including any desired quotes. Explicit sets win over removals.
    pub set: Vec<(String, String)>,
    pub remove_if: Vec<(String, Vec<String>)>,
    pub remove: Vec<String>,
}
impl DotenvPatch {
    pub fn apply(&self, pre: Option<&[u8]>) -> Result<Vec<u8>, PatchError> {
        let bytes = pre.unwrap_or_default();
        if bytes.len() > 1024 * 1024 || bytes.contains(&0) {
            return Err(PatchError::InvalidInput);
        }
        let text = std::str::from_utf8(bytes).map_err(|_| PatchError::InvalidInput)?;
        for key in self
            .set
            .iter()
            .map(|(k, _)| k)
            .chain(self.remove.iter())
            .chain(self.remove_if.iter().map(|(k, _)| k))
        {
            if !valid_key(key) {
                return Err(PatchError::InvalidInput);
            }
        }
        for value in self
            .set
            .iter()
            .map(|(_, v)| v)
            .chain(self.remove_if.iter().flat_map(|(_, values)| values))
        {
            if value.contains(['\n', '\r', '\0']) {
                return Err(PatchError::InvalidInput);
            }
        }
        // Keep each untouched line's own terminator, including mixed LF/CRLF files.
        let mut lines: Vec<(String, String)> = text
            .split_inclusive('\n')
            .map(|line| {
                if let Some(raw) = line.strip_suffix("\r\n") {
                    (raw.into(), "\r\n".into())
                } else if let Some(raw) = line.strip_suffix('\n') {
                    (raw.into(), "\n".into())
                } else {
                    (line.into(), String::new())
                }
            })
            .collect();
        let targeted = |key: &str| self.set.iter().any(|(target, _)| target == key);
        lines.retain(|(raw, _)| {
            let Some(key) = parse_key(raw) else {
                return true;
            };
            if targeted(key) {
                return true;
            }
            if self.clear.is_some_and(|clear| clear(key)) || self.remove.iter().any(|k| k == key) {
                return false;
            }
            !self
                .remove_if
                .iter()
                .any(|(k, values)| k == key && values.iter().any(|v| unquote(value_of(raw)) == v))
        });
        let separator = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let trailing = pre.is_none() || text.is_empty() || text.ends_with('\n');
        for (key, value) in &self.set {
            let mut seen = false;
            lines.retain_mut(|(raw, _)| {
                if parse_key(raw) != Some(key.as_str()) {
                    return true;
                }
                if seen {
                    return false;
                }
                seen = true;
                let export = if raw.trim_start().starts_with("export ") { "export " } else { "" };
                *raw = format!("{export}{key}={value}");
                true
            });
            if !seen {
                if let Some((_, ending)) = lines.last_mut()
                    && ending.is_empty()
                {
                    *ending = separator.into();
                }
                lines.push((
                    format!("{key}={value}"),
                    if trailing { separator.into() } else { String::new() },
                ));
            }
        }
        let output: String = lines.into_iter().flat_map(|(raw, ending)| [raw, ending]).collect();
        if output.len() > 1024 * 1024 {
            return Err(PatchError::InvalidInput);
        }
        Ok(output.into_bytes())
    }
}

/// Raw values retain their quotes; duplicate definitions take the last value,
/// while keys retain their first appearance order (as in the source editor).
pub fn entries(text: &str) -> Vec<(String, String)> {
    let mut entries: Vec<(String, String)> = Vec::new();
    for raw in text.lines() {
        let Some(key) = parse_key(raw) else {
            continue;
        };
        let value = value_of(raw).to_string();
        match entries.iter_mut().find(|(existing, _)| existing == key) {
            Some(entry) => entry.1 = value,
            None => entries.push((key.into(), value)),
        }
    }
    entries
}

fn valid_key(key: &str) -> bool {
    !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
fn parse_key(raw: &str) -> Option<&str> {
    let line = raw.trim_start();
    if line.starts_with('#') {
        return None;
    }
    let line = line.strip_prefix("export ").unwrap_or(line);
    let (key, _) = line.split_once('=')?;
    let key = key.trim();
    valid_key(key).then_some(key)
}
fn value_of(raw: &str) -> &str {
    raw.split_once('=').map_or("", |(_, value)| value.trim())
}
fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = value.strip_prefix(quote).and_then(|rest| rest.strip_suffix(quote)) {
            return inner;
        }
    }
    value
}
