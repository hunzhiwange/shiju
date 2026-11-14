use super::moonshot_schema::*;

/// Codex Desktop `automation_update` shape reduced to the two positions
/// Moonshot rejected: a property `$ref` with a description and a `$defs`
/// entry `$ref` with `type`/`minLength` siblings.
fn desktop_like_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "prompt": { "$ref": "#/$defs/__schema20", "description": "Prompt to run" },
            "mode": { "type": "string", "enum": ["fast", "slow"] }
        },
        "required": ["prompt"],
        "$defs": {
            "__schema20": { "$ref": "#/$defs/__schema2", "type": "string", "minLength": 1 },
            "__schema2": { "type": "string" }
        }
    })
}

fn has_ref_with_siblings(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            (map.len() > 1 && map.contains_key("$ref"))
                || map.values().any(has_ref_with_siblings)
        }
        Value::Array(items) => items.iter().any(has_ref_with_siblings),
        _ => false,
    }
}

#[test]
fn cc_route_052_gate_matches_moonshot_and_kimi_hosts_only() {
    for url in [
        "https://api.moonshot.cn/v1",
        "https://api.moonshot.ai/v1/",
        "https://api.kimi.com/coding/v1",
        "https://API.KIMI.COM/coding/v1",
        " https://api.moonshot.cn/v1/chat/completions ",
    ] {
        assert!(upstream_requires_ref_sibling_all_of(url), "{url}");
    }
    for url in [
        "https://api.openai.com/v1",
        "https://api.x.ai/v1",
        "https://kimi-relay.example.com/v1",
        "https://api.kimi.com.evil.net/v1",
        "https://notmoonshot.cn/v1",
        "api.moonshot.cn/v1",
        "",
    ] {
        assert!(!upstream_requires_ref_sibling_all_of(url), "{url}");
    }
}

#[test]
fn cc_route_052_moonshot_schema_expected_behavior() {
    let mut schema = desktop_like_schema();
    assert_eq!(wrap_ref_siblings(&mut schema), 2);
    assert_eq!(
        schema,
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "description": "Prompt to run",
                    "allOf": [{ "$ref": "#/$defs/__schema20" }]
                },
                "mode": { "type": "string", "enum": ["fast", "slow"] }
            },
            "required": ["prompt"],
            "$defs": {
                "__schema20": {
                    "type": "string",
                    "minLength": 1,
                    "allOf": [{ "$ref": "#/$defs/__schema2" }]
                },
                "__schema2": { "type": "string" }
            }
        })
    );
    assert!(!has_ref_with_siblings(&schema));
}

#[test]
fn cc_route_052_bare_refs_and_ref_free_schemas_are_untouched() {
    let original = json!({
        "type": "object",
        "properties": {
            "a": { "$ref": "#/$defs/A" },
            "b": { "type": "integer", "minimum": 0 }
        },
        "$defs": { "A": { "type": "string" } }
    });
    let mut schema = original.clone();
    assert_eq!(wrap_ref_siblings(&mut schema), 0);
    assert_eq!(schema, original);

    let original = json!({ "type": "object", "properties": { "a": { "type": "string" } } });
    let mut schema = original.clone();
    assert_eq!(wrap_ref_siblings(&mut schema), 0);
    assert_eq!(schema, original);
}

#[test]
fn cc_route_052_appends_ref_to_existing_all_of() {
    let mut schema = json!({
        "allOf": [{ "type": "string" }],
        "$ref": "#/$defs/A",
        "description": "d"
    });
    assert_eq!(wrap_ref_siblings(&mut schema), 1);
    assert_eq!(
        schema,
        json!({
            "allOf": [{ "type": "string" }, { "$ref": "#/$defs/A" }],
            "description": "d"
        })
    );
}

#[test]
fn cc_route_052_root_ref_keeps_type_and_defs() {
    let mut schema = json!({
        "type": "object",
        "$ref": "#/$defs/Root",
        "$defs": { "Root": { "properties": { "a": { "type": "string" } } } }
    });
    assert_eq!(wrap_ref_siblings(&mut schema), 1);
    assert_eq!(
        schema,
        json!({
            "type": "object",
            "$defs": { "Root": { "properties": { "a": { "type": "string" } } } },
            "allOf": [{ "$ref": "#/$defs/Root" }]
        })
    );
}

#[test]
fn cc_route_052_does_not_enter_data_values_or_unknown_keywords() {
    let original = json!({
        "type": "object",
        "properties": {
            // A property literally named `$ref` is a property, not a reference.
            "$ref": { "type": "string", "description": "literal name" },
            "cfg": {
                "type": "object",
                "default": { "$ref": "literal", "note": "data" },
                "examples": [{ "$ref": "literal", "note": "data" }],
                "enum": [{ "$ref": "literal", "note": "data" }],
                "const": { "$ref": "literal", "note": "data" }
            }
        },
        "x-metadata": { "$ref": "literal", "note": "vendor extension" },
        "$ref_like": { "$ref": "literal", "note": "unknown keyword" }
    });
    let mut schema = original.clone();
    assert_eq!(wrap_ref_siblings(&mut schema), 0);
    assert_eq!(schema, original);
}

#[test]
fn cc_route_052_covers_every_schema_valued_keyword() {
    let node = || json!({ "$ref": "#/$defs/A", "description": "d" });
    let mut schema = json!({
        "type": "object",
        "properties": { "p": node() },
        "patternProperties": { "^x": node() },
        "additionalProperties": node(),
        "unevaluatedProperties": node(),
        "propertyNames": node(),
        "dependentSchemas": { "p": node() },
        "dependencies": { "p": node(), "q": ["p"] },
        "items": node(),
        "prefixItems": [node()],
        "contains": node(),
        "not": node(),
        "if": node(),
        "then": node(),
        "else": node(),
        "allOf": [node()],
        "anyOf": [node()],
        "oneOf": [node()],
        "$defs": { "A": { "type": "string" }, "B": node() },
        "definitions": { "C": node() }
    });
    assert_eq!(wrap_ref_siblings(&mut schema), 19);
    assert!(!has_ref_with_siblings(&schema));
    assert_eq!(schema["dependencies"]["q"], json!(["p"]));

    // Draft-07 tuple form of `items`.
    let mut tuple = json!({ "type": "array", "items": [node(), { "type": "string" }] });
    assert_eq!(wrap_ref_siblings(&mut tuple), 1);
    assert!(!has_ref_with_siblings(&tuple));
}

#[test]
fn cc_route_052_rewrite_is_idempotent() {
    let mut once = desktop_like_schema();
    wrap_ref_siblings(&mut once);
    let mut twice = once.clone();
    assert_eq!(wrap_ref_siblings(&mut twice), 0);
    assert_eq!(twice, once);
}

#[test]
fn cc_route_052_chat_body_helper_counts_changed_tools_and_skips_non_function_tools() {
    let clean = json!({
        "type": "function",
        "function": {
            "name": "clean",
            "parameters": { "type": "object", "properties": { "a": { "type": "string" } } }
        }
    });
    let mut body = json!({
        "model": "k3",
        "messages": [],
        "tools": [
            { "type": "function", "function": { "name": "desktop", "parameters": desktop_like_schema() } },
            clean.clone(),
            { "type": "web_search" }
        ]
    });
    assert_eq!(wrap_ref_siblings_in_chat_tools(&mut body), 1);
    assert!(!has_ref_with_siblings(
        &body["tools"][0]["function"]["parameters"]
    ));
    assert_eq!(body["tools"][1], clean);
    assert_eq!(body["tools"][2], json!({ "type": "web_search" }));

    let mut no_tools = json!({ "model": "k3", "messages": [] });
    assert_eq!(wrap_ref_siblings_in_chat_tools(&mut no_tools), 0);
}

#[test]
fn cc_route_052_tool_input_values_are_unchanged() {
    let literal = json!({"$ref":"literal input", "type":"data"});
    let mut body = json!({
        "messages": [{"role":"assistant", "tool_calls":[{
            "id":"call_1", "type":"function",
            "function":{"name":"run", "arguments":literal.to_string()}
        }]}, {"role":"tool", "tool_call_id":"call_1", "content":literal.to_string()}],
        "tools":[{"type":"function", "function":{
            "name":"run", "parameters":{"type":"object", "$ref":"#/$defs/Input",
                "$defs":{"Input":{"type":"object"}}, "default":literal}
        }}]
    });
    let messages = body["messages"].clone();
    assert_eq!(wrap_ref_siblings_in_chat_tools(&mut body), 1);
    assert_eq!(body["messages"], messages);
    assert_eq!(body["tools"][0]["function"]["parameters"]["default"], literal);
}
