//! Named JSON/JSONC object-key edits. Comments and trailing commas are accepted;
//! JSON5 identifiers/single quotes are deliberately rejected. Untouched bytes survive.
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchError {
    Parse,
    Shape,
    InvalidPath,
}

#[derive(Default)]
pub struct JsonPatch {
    /// Explicit targets win over remove; missing object ancestors are created.
    pub set: Vec<(Vec<String>, Value)>,
    pub remove: Vec<Vec<String>>,
    /// Write only absent keys. Existing null is a value, not absence.
    pub seed: Vec<(Vec<String>, Value)>,
}

impl JsonPatch {
    /// Pure preparation: callers own file selection, authorization, locks and commit.
    /// Missing files start as {}; existing empty/malformed files are rejected.
    /// Errors never return partial output or include configuration payloads.
    pub fn apply(&self, pre: Option<&[u8]>) -> Result<Vec<u8>, PatchError> {
        let mut source = pre.unwrap_or(b"{}").to_vec();
        parse(&source)?;
        for path in self
            .remove
            .iter()
            .chain(self.set.iter().map(|(p, _)| p))
            .chain(self.seed.iter().map(|(p, _)| p))
        {
            if path.is_empty() || path.len() > 64 {
                return Err(PatchError::InvalidPath);
            }
        }
        for path in &self.remove {
            if !self.set.iter().any(|(target, _)| target == path) {
                edit(&mut source, path, None, false)?;
            }
        }
        for (path, value) in &self.set {
            edit(&mut source, path, Some(value), false)?;
        }
        for (path, value) in &self.seed {
            edit(&mut source, path, Some(value), true)?;
        }
        parse(&source)?;
        Ok(source)
    }
}

/// Read the same validated JSON/JSONC object used by byte-preserving edits.
pub(crate) fn object_value(source: &[u8]) -> Result<Value, PatchError> {
    parse(source)?;
    serde_json::from_slice(&without_trailing_commas(&masked(source)?))
        .map_err(|_| PatchError::Parse)
}

/// Replace one root member, retaining trivia inside compatible objects and arrays.
#[cfg(target_os = "macos")]
pub(crate) fn replace_member_preserving_trivia(
    source: &[u8],
    key: &str,
    desired: Value,
) -> Result<Vec<u8>, PatchError> {
    let root = parse(source)?;
    let Some(member) =
        root.members.as_ref().and_then(|members| members.iter().find(|m| m.key == key))
    else {
        return JsonPatch { set: vec![(vec![key.to_owned()], desired)], ..Default::default() }
            .apply(Some(source));
    };
    let masked = masked(source)?;
    let span = member.value.start..member.value.end;
    let before = serde_json::from_slice(&without_trailing_commas(&masked[span.clone()]))
        .map_err(|_| PatchError::Parse)?;
    let replacement = reconcile(source, &masked, &member.value, &before, &desired)?;
    let mut result = source.to_vec();
    result.splice(span, replacement);
    parse(&result)?;
    Ok(result)
}

/// Reconcile an entire root object while retaining file and value trivia.
#[cfg(target_os = "macos")]
pub(crate) fn replace_object_preserving_trivia(
    source: &[u8],
    desired: Value,
) -> Result<Vec<u8>, PatchError> {
    let root = parse(source)?;
    let masked = masked(source)?;
    let span = root.start..root.end;
    let before = serde_json::from_slice(&without_trailing_commas(&masked[span.clone()]))
        .map_err(|_| PatchError::Parse)?;
    let replacement = reconcile(source, &masked, &root, &before, &desired)?;
    let mut result = source.to_vec();
    result.splice(span, replacement);
    parse(&result)?;
    Ok(result)
}

#[cfg(target_os = "macos")]
fn reconcile(
    source: &[u8],
    masked: &[u8],
    target: &Node,
    before: &Value,
    desired: &Value,
) -> Result<Vec<u8>, PatchError> {
    let span = target.start..target.end;
    if before == desired {
        return Ok(source[span].to_vec());
    }
    match (before, desired) {
        (Value::Object(before), Value::Object(desired)) => {
            let mut result = source[span].to_vec();
            let mut patch = JsonPatch::default();
            for member in target.members.as_ref().ok_or(PatchError::Shape)?.iter().rev() {
                if let Some(next) = desired.get(&member.key) {
                    let child = &member.value;
                    let old = before.get(&member.key).ok_or(PatchError::Shape)?;
                    if old != next {
                        let replacement = reconcile(source, masked, child, old, next)?;
                        result.splice(
                            child.start - target.start..child.end - target.start,
                            replacement,
                        );
                    }
                } else {
                    patch.remove.push(vec![member.key.clone()]);
                }
            }
            for (key, value) in desired.iter().filter(|(key, _)| !before.contains_key(*key)) {
                patch.set.push((vec![key.clone()], value.clone()));
            }
            if patch.remove.is_empty() && patch.set.is_empty() {
                Ok(result)
            } else {
                patch.apply(Some(&result))
            }
        }
        (Value::Array(before), Value::Array(desired)) => {
            reconcile_array(source, masked, target, before, desired)
        }
        _ => serde_json::to_vec(desired).map_err(|_| PatchError::Parse),
    }
}

#[cfg(target_os = "macos")]
fn reconcile_array(
    source: &[u8],
    masked: &[u8],
    target: &Node,
    before: &[Value],
    desired: &[Value],
) -> Result<Vec<u8>, PatchError> {
    let mut result = vec![b'['];
    let mut cursor = target.start + 1;
    for (index, old) in before.iter().enumerate() {
        let mut pos = cursor;
        let child = node(masked, &mut pos)?;
        result.extend_from_slice(&source[cursor..child.start]);
        skip(masked, &mut pos);
        let comma = masked[pos] == b',';
        if let Some(next) = desired.get(index) {
            result.extend(reconcile(source, masked, &child, old, next)?);
            // Insert a new separator before trailing trivia (which may be a line comment).
            if !comma && index + 1 < desired.len() {
                result.push(b',');
            }
        }
        result.extend_from_slice(&source[child.end..pos]);
        if comma
            && index < desired.len()
            && (index + 1 < desired.len() || index + 1 == before.len())
        {
            result.push(b',');
        }
        cursor = pos + usize::from(comma);
    }
    result.extend_from_slice(&source[cursor..target.end - 1]);
    for (index, value) in desired.iter().skip(before.len()).enumerate() {
        if index > 0 {
            result.push(b',');
        }
        serde_json::to_writer(&mut result, value).map_err(|_| PatchError::Parse)?;
    }
    result.push(b']');
    Ok(result)
}

struct Node {
    start: usize,
    end: usize,
    // None distinguishes scalar/array values from an empty object.
    members: Option<Vec<Member>>,
}

struct Member {
    key: String,
    start: usize,
    value: Node,
    comma: Option<usize>,
}

fn skip(bytes: &[u8], pos: &mut usize) {
    while bytes.get(*pos).is_some_and(u8::is_ascii_whitespace) {
        *pos += 1;
    }
}

fn string_end(bytes: &[u8], pos: &mut usize) {
    *pos += 1;
    while *pos < bytes.len() {
        match bytes[*pos] {
            b'\\' => *pos += 2,
            b'"' => {
                *pos += 1;
                break;
            }
            _ => *pos += 1,
        }
    }
}

/// Mask trivia in a copy, retaining every original byte offset. String escapes
/// are skipped before looking for comment markers (including URLs and Unicode).
fn masked(source: &[u8]) -> Result<Vec<u8>, PatchError> {
    std::str::from_utf8(source).map_err(|_| PatchError::Parse)?;
    let mut bytes = source.to_vec();
    let mut pos = 0;
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        bytes[..3].fill(b' ');
        pos = 3;
    }
    while pos < bytes.len() {
        if bytes[pos] == b'"' {
            string_end(&bytes, &mut pos);
        } else if bytes[pos..].starts_with(b"//") {
            while pos < bytes.len() && !matches!(bytes[pos], b'\r' | b'\n') {
                bytes[pos] = b' ';
                pos += 1;
            }
        } else if bytes[pos..].starts_with(b"/*") {
            let start = pos;
            pos += 2;
            while pos + 1 < bytes.len() && &bytes[pos..pos + 2] != b"*/" {
                pos += 1;
            }
            if pos + 1 >= bytes.len() {
                return Err(PatchError::Parse);
            }
            pos += 2;
            for byte in &mut bytes[start..pos] {
                if !matches!(*byte, b'\r' | b'\n') {
                    *byte = b' ';
                }
            }
        } else {
            pos += 1;
        }
    }
    Ok(bytes)
}

fn without_trailing_commas(bytes: &[u8]) -> Vec<u8> {
    let mut strict = bytes.to_vec();
    let mut pos = 0;
    while pos < bytes.len() {
        if bytes[pos] == b'"' {
            string_end(bytes, &mut pos);
            continue;
        }
        if bytes[pos] == b',' {
            let mut next = pos + 1;
            skip(bytes, &mut next);
            let previous = bytes[..pos].iter().rfind(|byte| !byte.is_ascii_whitespace());
            if matches!(bytes.get(next), Some(b'}' | b']'))
                && !matches!(previous, None | Some(b'{' | b'[' | b',' | b':'))
            {
                strict[pos] = b' ';
            }
        }
        pos += 1;
    }
    strict
}

fn parse(source: &[u8]) -> Result<Node, PatchError> {
    if source.len() > 1024 * 1024 {
        return Err(PatchError::Parse);
    }
    let bytes = masked(source)?;
    let strict = without_trailing_commas(&bytes);
    let value: Value = serde_json::from_slice(&strict).map_err(|_| PatchError::Parse)?;
    if !value.is_object() {
        return Err(PatchError::Shape);
    }
    node(&bytes, &mut 0)
}

/// Called only after serde_json validation (including its nesting limit).
fn node(bytes: &[u8], pos: &mut usize) -> Result<Node, PatchError> {
    skip(bytes, pos);
    let start = *pos;
    let members = match bytes[*pos] {
        b'{' => {
            *pos += 1;
            let mut members: Vec<Member> = Vec::new();
            let mut keys = HashSet::new();
            loop {
                skip(bytes, pos);
                if bytes[*pos] == b'}' {
                    *pos += 1;
                    break;
                }
                let key_start = *pos;
                string_end(bytes, pos);
                let key: String = serde_json::from_slice(&bytes[key_start..*pos])
                    .map_err(|_| PatchError::Parse)?;
                if !keys.insert(key.clone()) {
                    return Err(PatchError::Shape);
                }
                skip(bytes, pos);
                *pos += 1; // colon
                let value = node(bytes, pos)?;
                skip(bytes, pos);
                let comma = (bytes[*pos] == b',').then_some(*pos);
                if comma.is_some() {
                    *pos += 1;
                }
                members.push(Member { key, start: key_start, value, comma });
            }
            Some(members)
        }
        b'[' => {
            *pos += 1;
            loop {
                skip(bytes, pos);
                if bytes[*pos] == b']' {
                    *pos += 1;
                    break;
                }
                node(bytes, pos)?;
                skip(bytes, pos);
                if bytes[*pos] == b',' {
                    *pos += 1;
                }
            }
            None
        }
        b'"' => {
            string_end(bytes, pos);
            None
        }
        _ => {
            while *pos < bytes.len()
                && !bytes[*pos].is_ascii_whitespace()
                && !matches!(bytes[*pos], b',' | b'}' | b']')
            {
                *pos += 1;
            }
            None
        }
    };
    Ok(Node { start, end: *pos, members })
}

fn edit(
    source: &mut Vec<u8>,
    path: &[String],
    value: Option<&Value>,
    seed: bool,
) -> Result<(), PatchError> {
    let root = parse(source)?;
    let mut current = &root;
    for (depth, key) in path.iter().enumerate() {
        let members = current.members.as_ref().ok_or(PatchError::Shape)?;
        if let Some((index, member)) =
            members.iter().enumerate().find(|(_, member)| &member.key == key)
        {
            if depth + 1 != path.len() {
                current = &member.value;
                continue;
            }
            if let Some(value) = value {
                if !seed {
                    // Semantic no-ops keep escaped strings/numbers and inner comments.
                    let old = masked(&source[member.value.start..member.value.end])?;
                    let strict = without_trailing_commas(&old);
                    let old: Value =
                        serde_json::from_slice(&strict).map_err(|_| PatchError::Parse)?;
                    if old != *value {
                        let replacement =
                            serde_json::to_vec(value).map_err(|_| PatchError::Parse)?;
                        source.splice(member.value.start..member.value.end, replacement);
                    }
                }
            } else {
                if let Some(comma) = member.comma {
                    source.remove(comma);
                } else if index > 0 {
                    let comma = members[index - 1].comma.ok_or(PatchError::Parse)?;
                    // Remove the value first so the earlier comma's offset stays valid.
                    source.drain(member.start..member.value.end);
                    source.remove(comma);
                    return Ok(());
                }
                source.drain(member.start..member.value.end);
            }
            return Ok(());
        }
        let Some(value) = value else {
            return Ok(());
        };
        let mut nested = value.clone();
        for child in path[depth + 1..].iter().rev() {
            let mut map = serde_json::Map::new();
            map.insert(child.clone(), nested);
            nested = Value::Object(map);
        }
        let mut insertion = serde_json::to_vec(key).map_err(|_| PatchError::Parse)?;
        insertion.push(b':');
        insertion.extend(serde_json::to_vec(&nested).map_err(|_| PatchError::Parse)?);
        let trailing = members.last().is_some_and(|member| member.comma.is_some());
        if trailing {
            insertion.push(b',');
        }
        // Insert after existing trivia so comments are never duplicated or consumed.
        source.splice(current.end - 1..current.end - 1, insertion);
        if let Some(last) = members.last().filter(|member| member.comma.is_none()) {
            source.insert(last.value.end, b',');
        }
        return Ok(());
    }
    Err(PatchError::InvalidPath)
}
