//! Moonshot / Kimi Chat Completions schema compatibility for the Codex bridge.
//!
//! Moonshot's Chat Completions endpoint (`api.moonshot.cn`, `api.moonshot.ai`
//! and the Kimi For Coding endpoint `api.kimi.com`) validates
//! `tools[].function.parameters` with a pre-2019-09 reading of JSON Schema: an
//! object that carries `$ref` together with any sibling keyword is rejected with
//! HTTP 400 (`tools.function.parameters is not a valid moonshot flavored json
//! schema … when using $ref, type should be defined in the referenced schema
//! instead of the parent schema`). Codex Desktop's built-in tool set
//! (`automation_update`, schemars-generated `$defs.__schemaN` entries) carries
//! exactly that shape, so every Codex Desktop turn against Kimi failed
//! (farion1231/cc-switch#6867). Codex CLI's own tools never emit it.
//!
//! The rewrite is the draft-07 idiom for the same constraint: move the `$ref`
//! into `allOf` and leave every sibling where it is,
//! `{"$ref": P, "type": "string"}` → `{"allOf": [{"$ref": P}], "type": "string"}`.
//! Under 2020-12 a `$ref` with siblings is already a conjunction, so nothing is
//! dereferenced, merged or dropped and the schema stays exact. Moonshot accepts
//! the `allOf` form at property, `$defs` entry, root and existing-`allOf`
//! positions (probed 2026-09-04 against Kimi For Coding, model `k3`; the root
//! object must keep `type: "object"`, which the Chat transform guarantees).
//!
//! Scope is deliberately narrow: only the Responses → Chat path, and only when
//! the resolved upstream host is Moonshot/Kimi. Their Anthropic-compatible
//! endpoint accepts the original shape (probed the same day), and every other
//! provider keeps byte-identical tool schemas so prompt-cache prefixes are not
//! disturbed. Relays that forward to Moonshot under their own host are not
//! matched; extend [`MOONSHOT_HOST_SUFFIXES`] if such a report shows up.

use serde_json::{json, Map, Value};
use reqwest::Url;

/// Upstream hosts whose Chat Completions validator rejects `$ref` siblings.
/// Matched as the host itself or any subdomain of it.
const MOONSHOT_HOST_SUFFIXES: &[&str] = &["moonshot.cn", "moonshot.ai", "kimi.com"];

/// Keywords whose value is a single schema (or, for draft-07 `items`, a tuple
/// of schemas). Boolean schemas (`additionalProperties: false`) are skipped by
/// the object check in [`wrap_ref_siblings`].
const SINGLE_SCHEMA_KEYWORDS: &[&str] = &[
    "items",
    "additionalItems",
    "unevaluatedItems",
    "contains",
    "additionalProperties",
    "unevaluatedProperties",
    "propertyNames",
    "not",
    "if",
    "then",
    "else",
    "contentSchema",
];

/// Keywords whose value is an array of schemas.
const SCHEMA_ARRAY_KEYWORDS: &[&str] = &["allOf", "anyOf", "oneOf", "prefixItems"];

/// Keywords whose value is a map from name to schema. Draft-07 `dependencies`
/// mixes schema values with string arrays; the array form is left alone by
/// the object check.
const SCHEMA_MAP_KEYWORDS: &[&str] = &[
    "properties",
    "patternProperties",
    "$defs",
    "definitions",
    "dependentSchemas",
    "dependencies",
];

/// Whether the resolved upstream `base_url` points at a Moonshot / Kimi host.
///
/// Fails closed on unparsable URLs: no host, no rewrite, and the request goes
/// out unchanged exactly as it did before this module existed.
pub fn upstream_requires_ref_sibling_all_of(base_url: &str) -> bool {
    let Ok(url) = Url::parse(base_url.trim()) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    MOONSHOT_HOST_SUFFIXES.iter().any(|suffix| {
        host == *suffix
            || host
                .strip_suffix(suffix)
                .is_some_and(|prefix| prefix.ends_with('.'))
    })
}

/// Rewrite every `$ref`-with-siblings node inside `tools[].function.parameters`
/// of a Chat Completions body. Returns how many tools had their schema changed.
pub fn wrap_ref_siblings_in_chat_tools(chat_body: &mut Value) -> usize {
    let Some(tools) = chat_body.get_mut("tools").and_then(Value::as_array_mut) else {
        return 0;
    };
    let mut changed = 0;
    for tool in tools.iter_mut() {
        let Some(parameters) = tool
            .get_mut("function")
            .and_then(|function| function.get_mut("parameters"))
        else {
            continue;
        };
        if wrap_ref_siblings(parameters) > 0 {
            changed += 1;
        }
    }
    changed
}

/// Walk a JSON Schema through schema-valued keywords only and move every
/// `$ref` that has sibling keywords into `allOf`. Returns the number of nodes
/// rewritten. Data-valued keywords (`default`, `examples`, `enum`, `const`) and
/// unknown keywords such as `x-…` extensions are never entered, so a literal
/// `$ref` key inside them stays untouched.
pub fn wrap_ref_siblings(schema: &mut Value) -> usize {
    let Value::Object(map) = schema else {
        return 0;
    };
    let mut rewritten = 0;
    if map.len() > 1 && map.get("$ref").is_some_and(Value::is_string) {
        move_ref_into_all_of(map);
        rewritten += 1;
    }
    for (key, child) in map.iter_mut() {
        let key = key.as_str();
        if SCHEMA_MAP_KEYWORDS.contains(&key) {
            if let Value::Object(entries) = child {
                rewritten += entries.values_mut().map(wrap_ref_siblings).sum::<usize>();
            }
        } else if SCHEMA_ARRAY_KEYWORDS.contains(&key) {
            if let Value::Array(entries) = child {
                rewritten += entries.iter_mut().map(wrap_ref_siblings).sum::<usize>();
            }
        } else if SINGLE_SCHEMA_KEYWORDS.contains(&key) {
            match child {
                // Draft-07 tuple validation: `items` may be an array of schemas.
                Value::Array(entries) => {
                    rewritten += entries.iter_mut().map(wrap_ref_siblings).sum::<usize>();
                }
                other => rewritten += wrap_ref_siblings(other),
            }
        }
    }
    rewritten
}

fn move_ref_into_all_of(map: &mut Map<String, Value>) {
    let Some(reference) = map.remove("$ref") else {
        return;
    };
    let branch = json!({ "$ref": reference });
    match map.get_mut("allOf") {
        Some(Value::Array(branches)) => branches.push(branch),
        _ => {
            map.insert("allOf".to_string(), Value::Array(vec![branch]));
        }
    }
}
