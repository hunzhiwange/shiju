//! DSML tool calls, including repeated bars, whitespace and the `calls` alias.
//! Only tag syntax is repaired. Parameter bytes and JSON types are never rewritten.
use super::ParsedToolCall;
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

static TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<(/?)[｜|]+DSML[｜|]+\s*([a-z_]+)\b([^>]*)>").unwrap());
static PARAMETER_CLOSE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)</[｜|]+DSML[｜|]+\s*parameter\s*>").unwrap());
static ATTRIBUTE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\s+([a-z_]+)\s*=\s*(?:"([^"]*)"|'([^']*)')"#).unwrap());

pub(crate) fn contains_deepseek_dsml_markup(response: &str) -> bool {
    static MARKER_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)</?[｜|]+DSML[｜|]+").unwrap());
    MARKER_RE.is_match(response)
}

struct Tag<'a> {
    closing: bool,
    name: &'a str,
    attributes: &'a str,
}

fn take_tag<'a>(remaining: &mut &'a str) -> Option<Tag<'a>> {
    *remaining = remaining.trim_start();
    let capture = TAG_RE.captures(remaining)?;
    let full = capture.get(0)?;
    if full.start() != 0 {
        return None;
    }
    let tag = Tag {
        closing: !capture.get(1)?.as_str().is_empty(),
        name: capture.get(2)?.as_str(),
        attributes: capture.get(3)?.as_str(),
    };
    *remaining = &remaining[full.end()..];
    Some(tag)
}

fn attributes(raw: &str) -> Option<Map<String, Value>> {
    let mut result = Map::new();
    let mut end = 0;
    for capture in ATTRIBUTE_RE.captures_iter(raw) {
        let full = capture.get(0)?;
        if !raw[end..full.start()].trim().is_empty() {
            return None;
        }
        let key = capture.get(1)?.as_str().to_string();
        let value = capture.get(2).or_else(|| capture.get(3))?.as_str();
        if result.insert(key, Value::String(value.to_string())).is_some() {
            return None;
        }
        end = full.end();
    }
    raw[end..].trim().is_empty().then_some(result)
}

fn parse_invoke(remaining: &mut &str, tag: Tag<'_>) -> Option<ParsedToolCall> {
    let attrs = attributes(tag.attributes)?;
    let name = attrs.get("name")?.as_str()?.trim();
    if name.is_empty() || attrs.len() != 1 {
        return None;
    }
    let mut arguments = Map::new();
    loop {
        let parameter = take_tag(remaining)?;
        if parameter.closing && parameter.name.eq_ignore_ascii_case("invoke") {
            if !parameter.attributes.trim().is_empty() {
                return None;
            }
            break;
        }
        if parameter.closing || !parameter.name.eq_ignore_ascii_case("parameter") {
            return None;
        }
        let attrs = attributes(parameter.attributes)?;
        let key = attrs.get("name")?.as_str()?.trim();
        if key.is_empty() || attrs.len() != 2 {
            return None;
        }
        let close = PARAMETER_CLOSE_RE.find(remaining)?;
        let raw_value = &remaining[..close.start()];
        let value = match attrs.get("string")?.as_str()? {
            "true" => Value::String(raw_value.to_string()),
            "false" => serde_json::from_str(raw_value).ok()?,
            _ => return None,
        };
        if arguments.insert(key.to_string(), value).is_some() {
            return None;
        }
        *remaining = &remaining[close.end()..];
    }
    Some(ParsedToolCall {
        name: name.to_string(),
        arguments: Value::Object(arguments),
        tool_call_id: None,
    })
}

pub(super) fn parse_deepseek_dsml_calls(response: &str) -> Option<(String, Vec<ParsedToolCall>)> {
    let mut remaining = response;
    let mut text = Vec::new();
    let mut calls = Vec::new();
    while let Some(first) = TAG_RE.find(remaining) {
        let before = &remaining[..first.start()];
        if contains_deepseek_dsml_markup(before) {
            return None;
        }
        if !before.trim().is_empty() {
            text.push(before.trim().to_string());
        }
        remaining = &remaining[first.start()..];
        let wrapper = take_tag(&mut remaining)?;
        if wrapper.closing
            || !(wrapper.name.eq_ignore_ascii_case("tool_calls")
                || wrapper.name.eq_ignore_ascii_case("calls"))
            || !wrapper.attributes.trim().is_empty()
        {
            return None;
        }
        let count = calls.len();
        loop {
            let tag = take_tag(&mut remaining)?;
            if tag.closing && tag.name.eq_ignore_ascii_case(wrapper.name) {
                if calls.len() == count || !tag.attributes.trim().is_empty() {
                    return None;
                }
                break;
            }
            if tag.closing || !tag.name.eq_ignore_ascii_case("invoke") {
                return None;
            }
            calls.push(parse_invoke(&mut remaining, tag)?);
        }
    }
    if calls.is_empty() || contains_deepseek_dsml_markup(remaining) {
        return None;
    }
    if !remaining.trim().is_empty() {
        text.push(remaining.trim().to_string());
    }
    Some((text.join("\n"), calls))
}
