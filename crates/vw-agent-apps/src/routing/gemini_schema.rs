//! Gemini tool schema helpers.
//!
//! Gemini `FunctionDeclaration` supports two schema channels:
//! - `parameters`: a restricted `Schema` subset
//! - `parametersJsonSchema`: richer JSON Schema via arbitrary JSON `Value`
//!
//! Anthropic tool schemas are closer to JSON Schema, so we choose the richer
//! channel when unsupported `Schema` fields are present.

use serde_json::{Map, Value, json};

#[derive(Debug, Clone, PartialEq)]
pub enum GeminiFunctionParameters {
    Schema(Value),
    JsonSchema(Value),
}

pub fn build_gemini_function_parameters(input_schema: Value) -> GeminiFunctionParameters {
    let schema = ensure_object_schema(normalize_json_schema(input_schema));

    if requires_parameters_json_schema(&schema) {
        GeminiFunctionParameters::JsonSchema(schema)
    } else {
        GeminiFunctionParameters::Schema(to_gemini_schema(schema))
    }
}

/// Vertex AI rejects FunctionDeclarations whose `parameters` schema lacks an
/// explicit `type: "object"`, returning:
///
/// > functionDeclaration parameters schema should be of type OBJECT.
///
/// Anthropic tools sometimes arrive with empty or type-less `input_schema`
/// (e.g. no-argument tools like Claude Code's `TodoRead`). Normalize those to
/// `{type: "object", properties: {}}` so the Gemini upstream accepts them.
///
/// References: google-gemini/generative-ai-python#423, BerriAI/litellm#5055.
fn ensure_object_schema(schema: Value) -> Value {
    match schema {
        Value::Object(mut obj) => {
            obj.entry("type".to_string()).or_insert_with(|| json!("object"));
            if obj.get("type").and_then(|v| v.as_str()) == Some("object") {
                obj.entry("properties".to_string()).or_insert_with(|| json!({}));
            }
            Value::Object(obj)
        }
        other => other,
    }
}

fn normalize_json_schema(schema: Value) -> Value {
    match schema {
        Value::Object(mut obj) => {
            obj.remove("$schema");
            obj.remove("$id");

            if let Some(properties) =
                obj.get_mut("properties").and_then(|value| value.as_object_mut())
            {
                for value in properties.values_mut() {
                    *value = normalize_json_schema(value.clone());
                }
            }

            if let Some(items) = obj.get_mut("items") {
                *items = normalize_json_schema(items.clone());
            }

            for key in ["anyOf", "oneOf", "allOf", "prefixItems"] {
                if let Some(values) = obj.get_mut(key).and_then(|value| value.as_array_mut()) {
                    for value in values.iter_mut() {
                        *value = normalize_json_schema(value.clone());
                    }
                }
            }

            for key in ["not", "if", "then", "else", "additionalProperties"] {
                if let Some(value) = obj.get_mut(key) {
                    *value = normalize_json_schema(value.clone());
                }
            }

            Value::Object(obj)
        }
        Value::Array(values) => {
            Value::Array(values.into_iter().map(normalize_json_schema).collect())
        }
        other => other,
    }
}

fn requires_parameters_json_schema(schema: &Value) -> bool {
    match schema {
        Value::Object(obj) => object_requires_parameters_json_schema(obj),
        Value::Array(values) => values.iter().any(requires_parameters_json_schema),
        _ => false,
    }
}

fn object_requires_parameters_json_schema(obj: &Map<String, Value>) -> bool {
    for (key, value) in obj {
        match key.as_str() {
            "type" => {
                if value.is_array() {
                    return true;
                }
            }
            "format" | "title" | "description" | "nullable" | "enum" | "maxItems" | "minItems"
            | "required" | "minProperties" | "maxProperties" | "minLength" | "maxLength"
            | "pattern" | "example" | "propertyOrdering" | "default" | "minimum" | "maximum" => {}
            "properties" => {
                let Some(properties) = value.as_object() else {
                    return true;
                };
                if properties.values().any(requires_parameters_json_schema) {
                    return true;
                }
            }
            "items" => {
                if !value.is_object() || requires_parameters_json_schema(value) {
                    return true;
                }
            }
            "anyOf" => {
                let Some(values) = value.as_array() else {
                    return true;
                };
                if values.iter().any(requires_parameters_json_schema) {
                    return true;
                }
            }
            // JSON Schema keywords that Gemini `parameters` does not accept.
            "$ref"
            | "$defs"
            | "definitions"
            | "additionalProperties"
            | "unevaluatedProperties"
            | "patternProperties"
            | "oneOf"
            | "allOf"
            | "const"
            | "not"
            | "if"
            | "then"
            | "else"
            | "dependentRequired"
            | "dependentSchemas"
            | "contains"
            | "minContains"
            | "maxContains"
            | "prefixItems"
            | "exclusiveMinimum"
            | "exclusiveMaximum"
            | "multipleOf"
            | "examples" => return true,
            // Be conservative for unknown keywords.
            _ => return true,
        }
    }

    false
}

fn to_gemini_schema(schema: Value) -> Value {
    match schema {
        Value::Object(obj) => {
            let mut result = Map::new();

            for (key, value) in obj {
                match key.as_str() {
                    "type" | "format" | "title" | "description" | "nullable" | "enum"
                    | "maxItems" | "minItems" | "required" | "minProperties" | "maxProperties"
                    | "minLength" | "maxLength" | "pattern" | "example" | "propertyOrdering"
                    | "default" | "minimum" | "maximum" => {
                        result.insert(key, value);
                    }
                    "properties" => {
                        if let Some(properties) = value.as_object() {
                            let converted = properties
                                .iter()
                                .map(|(name, property_schema)| {
                                    (name.clone(), to_gemini_schema(property_schema.clone()))
                                })
                                .collect();
                            result.insert("properties".to_string(), Value::Object(converted));
                        }
                    }
                    "items" if value.is_object() => {
                        result.insert("items".to_string(), to_gemini_schema(value));
                    }
                    "anyOf" => {
                        if let Some(values) = value.as_array() {
                            result.insert(
                                "anyOf".to_string(),
                                Value::Array(
                                    values
                                        .iter()
                                        .map(|value| to_gemini_schema(value.clone()))
                                        .collect(),
                                ),
                            );
                        }
                    }
                    _ => {}
                }
            }

            Value::Object(result)
        }
        other => other,
    }
}

pub fn build_gemini_function_declaration(
    name: &str,
    description: Option<&str>,
    input_schema: Value,
) -> Value {
    let mut declaration = Map::new();
    declaration.insert("name".to_string(), json!(name));
    declaration.insert("description".to_string(), json!(description.unwrap_or("")));

    match build_gemini_function_parameters(input_schema) {
        GeminiFunctionParameters::Schema(schema) => {
            declaration.insert("parameters".to_string(), schema);
        }
        GeminiFunctionParameters::JsonSchema(schema) => {
            declaration.insert("parametersJsonSchema".to_string(), schema);
        }
    }

    Value::Object(declaration)
}

use std::collections::{HashMap, HashSet};

/// Request-local hints; never retained across clients or used to execute tools.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolSchemaHint {
    pub expected_keys: Vec<String>,
    pub required_keys: Vec<String>,
}

pub type ToolSchemaHints = HashMap<String, ToolSchemaHint>;

/// Extract the same hints used by the source's Gemini response rectifier.
pub fn extract_tool_schema_hints(body: &Value) -> ToolSchemaHints {
    body.get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|tool| {
            let name = tool.get("name")?.as_str()?;
            let schema = tool.get("input_schema")?;
            schema_hint(schema).map(|hint| (name.to_owned(), hint))
        })
        .collect()
}

fn schema_hint(schema: &Value) -> Option<ToolSchemaHint> {
    let properties = schema.get("properties")?.as_object()?;
    if properties.is_empty() {
        return None;
    }
    Some(ToolSchemaHint {
        expected_keys: properties.keys().cloned().collect(),
        required_keys: schema
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
    })
}

/// Project only Schema channel declarations. Explicit JSON Schema and native
/// tool kinds (search, code execution, etc.) keep their original payloads.
pub fn prepare_native_request(body: &Value) -> Result<Value, &'static str> {
    let mut body = body.clone();
    let mut hints = ToolSchemaHints::new();
    if let Some(tools) = body.get_mut("tools") {
        for tool in tools.as_array_mut().ok_or("tools must be an array")? {
            let Some(declarations) = tool.get_mut("functionDeclarations") else {
                continue;
            };
            for declaration in
                declarations.as_array_mut().ok_or("functionDeclarations must be an array")?
            {
                let declaration =
                    declaration.as_object_mut().ok_or("invalid function declaration")?;
                if declaration.contains_key("parameters")
                    && declaration.contains_key("parametersJsonSchema")
                {
                    return Err("conflicting function parameter channels");
                }
                let name = declaration
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|name| !name.is_empty())
                    .ok_or("missing function name")?
                    .to_owned();
                if let Some(schema) = declaration
                    .get("parameters")
                    .or_else(|| declaration.get("parametersJsonSchema"))
                {
                    if !schema.is_object() {
                        return Err("function parameters must be a schema object");
                    }
                    if let Some(hint) = schema_hint(schema) {
                        hints.insert(name, hint);
                    }
                }
                if let Some(schema) = declaration.remove("parameters") {
                    match build_gemini_function_parameters(schema) {
                        GeminiFunctionParameters::Schema(schema) => {
                            declaration.insert("parameters".into(), schema);
                        }
                        GeminiFunctionParameters::JsonSchema(schema) => {
                            declaration.insert("parametersJsonSchema".into(), schema);
                        }
                    }
                }
            }
        }
    }
    if let Some(contents) = body.get_mut("contents").and_then(Value::as_array_mut) {
        for content in contents {
            if let Some(parts) = content.get_mut("parts").and_then(Value::as_array_mut) {
                for part in parts {
                    if let Some(call) = part.get_mut("functionCall").and_then(Value::as_object_mut)
                    {
                        let name = call.get("name").and_then(Value::as_str).map(str::to_owned);
                        if let (Some(name), Some(args)) = (name, call.get_mut("args")) {
                            rectify_tool_call_args(&name, args, Some(&hints));
                        }
                    }
                }
            }
        }
    }
    Ok(body)
}

pub fn rectify_tool_call_args(
    tool_name: &str,
    args: &mut Value,
    tool_schema_hints: Option<&ToolSchemaHints>,
) -> bool {
    let Some(tool_schema_hints) = tool_schema_hints else {
        return false;
    };
    let Some(hint) = tool_schema_hints.get(tool_name) else {
        return false;
    };
    let Some(args_object) = args.as_object_mut() else {
        return false;
    };
    if args_object.is_empty() || hint.expected_keys.is_empty() {
        return false;
    }
    let mut changed = false;

    if hint.expected_keys.iter().any(|key| key == "skill")
        && !hint.expected_keys.iter().any(|key| key == "name")
        && !args_object.contains_key("skill")
    {
        if let Some(value) = args_object.remove("name") {
            args_object.insert("skill".to_string(), value);
            changed = true;
        }
    }

    let expects_parameters_key = hint.expected_keys.iter().any(|key| key == "parameters");
    if !expects_parameters_key {
        let extracted_parameters = args_object
            .get("parameters")
            .and_then(|value| value.as_object())
            .map(|parameters_object| {
                hint.expected_keys
                    .iter()
                    .filter_map(|expected_key| {
                        if args_object.contains_key(expected_key) {
                            return None;
                        }
                        let value = parameters_object.get(expected_key)?;
                        let normalized_value = match value {
                            Value::Array(values) if values.len() == 1 => values[0].clone(),
                            _ => value.clone(),
                        };
                        Some((expected_key.clone(), normalized_value))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !extracted_parameters.is_empty() {
            for (expected_key, normalized_value) in extracted_parameters {
                args_object.insert(expected_key, normalized_value);
            }
            args_object.remove("parameters");
            changed = true;
        }
    }

    if hint.required_keys.iter().all(|key| args_object.contains_key(key.as_str())) {
        return changed;
    }

    let expected_key_set = hint.expected_keys.iter().map(String::as_str).collect::<HashSet<_>>();
    let unexpected_keys = args_object
        .keys()
        .filter(|key| !expected_key_set.contains(key.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if unexpected_keys.len() != 1 {
        return changed;
    }
    if hint.required_keys.iter().filter(|key| !args_object.contains_key(key.as_str())).count() > 1 {
        return changed;
    }

    let target_key = hint
        .required_keys
        .iter()
        .find(|key| !args_object.contains_key(key.as_str()))
        .cloned()
        .or_else(|| {
            if hint.expected_keys.len() == 1 && args_object.len() == 1 {
                hint.expected_keys.first().cloned()
            } else {
                None
            }
        });
    let Some(target_key) = target_key else {
        return changed;
    };
    if args_object.contains_key(&target_key) {
        return false;
    }

    let source_key = &unexpected_keys[0];
    let Some(value) = args_object.remove(source_key) else {
        return false;
    };
    args_object.insert(target_key, value);
    true
}
