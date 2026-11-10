use super::private_params::filter_private_params;
use serde_json::json;

#[test]
fn cc_route_014_private_params_expected_behavior() {
    let mut body = json!({
        "_session_token": "fixture",
        "model": "selected",
        "messages": [{"role": "user", "content": "lookup", "_trace": true}],
        "metadata": {"properties": {"_debug": true}, "$defs": {"_account": true}},
        "tools": [{
            "type": "function",
            "_auth": "fixture",
            "function": {
                "name": "lookup",
                "parameters": {
                    "_private_note": true,
                    "type": "object",
                    "properties": {"_id": {"type": "string", "_private_note": true}},
                    "required": ["_id"],
                    "allOf": [{"$defs": {"_value": {"type": "string"}}}]
                }
            }
        }]
    });
    filter_private_params(&mut body);
    assert_eq!(
        body,
        json!({
            "model": "selected",
            "messages": [{"role": "user", "content": "lookup"}],
            "metadata": {"properties": {}, "$defs": {}},
            "tools": [{
                "type": "function",
                "function": {
                    "name": "lookup",
                    "parameters": {
                        "type": "object",
                        "properties": {"_id": {"type": "string"}},
                        "required": ["_id"],
                        "allOf": [{"$defs": {"_value": {"type": "string"}}}]
                    }
                }
            }]
        })
    );
}

#[test]
fn cc_route_014_private_params_preserves_schema_names_at_tool_roots() {
    let schema = json!({
        "patternProperties": {"^_": {"type": "string"}},
        "definitions": {"_legacy": {"properties": {"_nested": {"type": "number"}}}},
        "items": {"properties": {"_item": {"type": "string"}}}
    });
    let mut body = json!({
        "tools": [
            {"name": "messages", "input_schema": schema},
            {"type": "function", "name": "responses", "parameters": schema},
            {"functionDeclarations": [{"name": "gemini", "parametersJsonSchema": schema}]}
        ],
        "other": {"tools": [{"input_schema": {"properties": {"_private": true}}}]}
    });
    let mut expected = body.clone();
    expected["other"]["tools"][0]["input_schema"]["properties"] = json!({});
    filter_private_params(&mut body);
    assert_eq!(body, expected);
}
