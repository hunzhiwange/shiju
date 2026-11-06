//! Validation of native declarations, matching the source OpenCode V2 known fields.
use serde_json::{Map, Value};
pub(super) fn native_provider_problem(value: &Value) -> Option<String> {
    let Some(obj) = value.as_object() else {
        return Some(String::new());
    };
    if let Some(key) = first_invalid_field(
        obj,
        &[
            ("name", Value::is_string),
            ("package", Value::is_string),
            ("canonical", Value::is_string),
            ("env", native_string_array),
            ("settings", native_provider_settings),
            ("headers", native_headers),
            ("body", Value::is_object),
        ],
    ) {
        return Some(key.to_string());
    }
    match obj.get("models") {
        None => None,
        Some(Value::Object(models)) => models.iter().find_map(|(id, model)| {
            native_model_problem(model).map(|key| match key {
                "" => format!("models.{id}"),
                key => format!("models.{id}.{key}"),
            })
        }),
        Some(_) => Some("models".to_string()),
    }
}

type NativeFieldCheck<'a> = (&'a str, fn(&Value) -> bool);

/// Optional means absent, not null. Unknown keys remain untouched at every level.
fn first_invalid_field<'a>(
    obj: &Map<String, Value>,
    fields: &[NativeFieldCheck<'a>],
) -> Option<&'a str> {
    fields
        .iter()
        .find(|(key, check)| obj.get(*key).is_some_and(|value| !check(value)))
        .map(|(key, _)| *key)
}

fn native_fields_valid(value: &Value, fields: &[NativeFieldCheck<'_>]) -> bool {
    value.as_object().is_some_and(|obj| first_invalid_field(obj, fields).is_none())
}

fn native_string_array(value: &Value) -> bool {
    value.as_array().is_some_and(|values| values.iter().all(Value::is_string))
}

fn native_finite(value: &Value) -> bool {
    value.as_f64().is_some_and(f64::is_finite)
}

fn native_integer(value: &Value) -> bool {
    // Effect Schema.Int uses JavaScript's safe integer range, including 1.0.
    value.as_f64().is_some_and(|n| n.fract() == 0.0 && n.abs() <= 9_007_199_254_740_991.0)
}

fn native_compaction(value: &Value) -> bool {
    value.as_object().is_some_and(|obj| {
        matches!(obj.get("type").and_then(Value::as_str), Some("summary" | "native"))
    })
}

fn native_provider_settings(value: &Value) -> bool {
    native_fields_valid(
        value,
        &[
            ("timeout", |v| v == &Value::Bool(false) || native_finite(v)),
            ("chunkTimeout", native_finite),
            ("compaction", native_compaction),
            ("transport", |v| matches!(v.as_str(), Some("http" | "websocket"))),
        ],
    )
}

fn native_headers(value: &Value) -> bool {
    value.as_object().is_some_and(|headers| headers.values().all(Value::is_string))
}

// Model/variant settings only constrain compaction; timeout and transport here
// are package-specific extensions, unlike provider settings.
fn native_model_settings(value: &Value) -> bool {
    native_fields_valid(value, &[("compaction", native_compaction)])
}

fn native_model_overlays(value: &Value) -> bool {
    native_fields_valid(
        value,
        &[
            ("settings", native_model_settings),
            ("headers", native_headers),
            ("body", Value::is_object),
        ],
    )
}

/// The first invalid field of a model; an empty name means it is not an object.
fn native_model_problem(value: &Value) -> Option<&'static str> {
    let Some(obj) = value.as_object() else {
        return Some("");
    };
    first_invalid_field(
        obj,
        &[
            ("modelID", Value::is_string),
            ("family", Value::is_string),
            ("name", Value::is_string),
            ("package", Value::is_string),
            ("disabled", Value::is_boolean),
            ("settings", native_model_settings),
            ("headers", native_headers),
            ("body", Value::is_object),
            ("compatibility", native_compatibility),
            ("capabilities", |v| {
                v.as_object().is_some_and(|obj| {
                    obj.get("tools").is_some_and(Value::is_boolean)
                        && obj.get("input").is_some_and(native_string_array)
                        && obj.get("output").is_some_and(native_string_array)
                })
            }),
            ("variants", |v| {
                v.as_array().is_some_and(|variants| {
                    variants.iter().all(|variant| {
                        variant.get("id").is_some_and(Value::is_string)
                            && native_model_overlays(variant)
                    })
                })
            }),
            ("cost", |v| match v.as_array() {
                Some(costs) => costs.iter().all(native_cost),
                None => native_cost(v),
            }),
            ("limit", |v| {
                native_fields_valid(
                    v,
                    &[
                        ("context", native_integer),
                        ("input", native_integer),
                        ("output", native_integer),
                    ],
                )
            }),
        ],
    )
}

fn native_compatibility(value: &Value) -> bool {
    native_fields_valid(
        value,
        &[
            ("reasoningField", Value::is_string),
            ("requireReasoning", Value::is_boolean),
            ("maxTokensField", |v| {
                matches!(v.as_str(), Some("max_completion_tokens" | "max_tokens"))
            }),
            ("requireFinishReason", Value::is_boolean),
            ("requireAssistantAfterTool", Value::is_boolean),
            ("supportsPromptCacheKey", Value::is_boolean),
        ],
    )
}

fn native_cost(value: &Value) -> bool {
    value.get("input").is_some_and(native_finite)
        && value.get("output").is_some_and(native_finite)
        && native_fields_valid(
            value,
            &[
                ("tier", |v| {
                    v.as_object().is_some_and(|obj| {
                        obj.get("type").and_then(Value::as_str) == Some("context")
                            && obj.get("size").is_some_and(native_integer)
                    })
                }),
                ("cache", |v| {
                    native_fields_valid(v, &[("read", native_finite), ("write", native_finite)])
                }),
            ],
        )
}
