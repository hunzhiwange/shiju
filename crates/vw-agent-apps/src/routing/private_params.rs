//! Remove private request fields while preserving declared tool schema names.

use serde_json::Value;

/// Apply the upstream's empty private-field allowlist. Only names declared
/// in a tool schema's name maps are exempt from the underscore rule.
pub fn filter_private_params(body: &mut Value) {
    filter(body, Object::Request);
}

#[derive(Clone, Copy)]
enum Object {
    Request,
    Tool,
    ChatFunction,
    GeminiFunction,
    Schema,
    SchemaNames,
    Plain,
}

fn filter(value: &mut Value, context: Object) {
    match value {
        Value::Object(fields) => fields.retain(|key, value| {
            if key.starts_with('_') && !matches!(context, Object::SchemaNames) {
                return false;
            }
            match (context, key.as_str(), value) {
                (Object::Request, "tools", Value::Array(tools)) => {
                    filter_array(tools, Object::Tool);
                }
                (Object::Tool, "function", value) => filter(value, Object::ChatFunction),
                (Object::Tool, "functionDeclarations", Value::Array(functions)) => {
                    filter_array(functions, Object::GeminiFunction);
                }
                (Object::Tool, "input_schema" | "parameters", value)
                | (Object::ChatFunction, "parameters", value)
                | (Object::GeminiFunction, "parameters" | "parametersJsonSchema", value)
                | (Object::SchemaNames, _, value) => filter(value, Object::Schema),
                (
                    Object::Schema,
                    "properties" | "patternProperties" | "definitions" | "$defs",
                    value,
                ) => filter(value, Object::SchemaNames),
                (
                    Object::Schema,
                    "allOf" | "anyOf" | "oneOf" | "prefixItems" | "items",
                    Value::Array(schemas),
                ) => filter_array(schemas, Object::Schema),
                (
                    Object::Schema,
                    "items"
                    | "additionalItems"
                    | "contains"
                    | "additionalProperties"
                    | "propertyNames"
                    | "unevaluatedProperties"
                    | "unevaluatedItems"
                    | "not"
                    | "if"
                    | "then"
                    | "else",
                    value,
                ) => filter(value, Object::Schema),
                (_, _, value) => filter(value, Object::Plain),
            }
            true
        }),
        Value::Array(values) => filter_array(values, Object::Plain),
        _ => {}
    }
}

fn filter_array(values: &mut [Value], context: Object) {
    for value in values {
        filter(value, context);
    }
}
