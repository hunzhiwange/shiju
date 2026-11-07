//! Pure native projection owner, shared with future application adapters. No filesystem or credentials lookup.
use serde_json::Value;
use vw_api_types::app_providers::preview::{PreviewParseError, PreviewUnavailable};
mod codex;
pub(super) mod codex_route;
pub(crate) mod fields;
mod grok;
mod json;
pub(crate) use codex::{native as codex_native, project as codex};
pub(crate) use grok::{document as grok_document, project as grok, selected as grok_selected};
pub(crate) use json::{claude, claude_with_residue, gemini};
pub(crate) struct Projected {
    pub before: Value,
    pub settings: Value,
    pub inactive: Vec<Vec<String>>,
    pub difference: Option<(Value, Value)>,
}
pub(crate) enum Error {
    Keyless,
    Draft(Value),
    Parse(PreviewParseError),
    Shape,
    Unavailable(PreviewUnavailable),
}
pub(crate) fn toml(text: &str, source: &str) -> Result<toml_edit::DocumentMut, Error> {
    text.parse::<toml_edit::DocumentMut>().map_err(|e| {
        let offset = e.span().map(|s| s.start).unwrap_or(0).min(text.len());
        let prefix = &text.as_bytes()[..offset];
        Error::Parse(PreviewParseError {
            source: source.into(),
            line: 1 + prefix.iter().filter(|b| **b == b'\n').count(),
            column: 1 + prefix.iter().rev().take_while(|b| **b != b'\n').count(),
        })
    })
}
pub(crate) fn value(doc: &toml_edit::DocumentMut) -> Result<Value, Error> {
    ::toml::from_str::<Value>(&doc.to_string()).map_err(|_| Error::Shape)
}
pub(crate) fn patch(
    doc: &toml_edit::DocumentMut,
    patch: crate::providers::live::patch_toml::TomlPatch,
) -> Result<toml_edit::DocumentMut, Error> {
    let bytes = patch.apply(Some(doc.to_string().as_bytes())).map_err(|_| Error::Shape)?;
    toml(std::str::from_utf8(&bytes).map_err(|_| Error::Shape)?, "projection")
}
/// TOML editor global entries use top-level values, each immediate table member,
/// and whole provider tables. Owned paths are excluded before inactive comparison.
pub(crate) fn inactive(
    row: &Value,
    shown: &Value,
    skip: impl Fn(&[String]) -> bool,
) -> Vec<Vec<String>> {
    let mut result = vec![];
    if let Some(root) = row.as_object() {
        for (key, value) in root {
            if let Some(table) = value.as_object() {
                for (child, v) in table {
                    let path = vec!["config".into(), key.clone(), child.clone()];
                    if !skip(&path) && shown.get(key).and_then(|s| s.get(child)) != Some(v) {
                        result.push(path);
                    }
                }
            } else {
                let path = vec!["config".into(), key.clone()];
                if !skip(&path) && shown.get(key) != Some(value) {
                    result.push(path);
                }
            }
        }
    }
    result
}
