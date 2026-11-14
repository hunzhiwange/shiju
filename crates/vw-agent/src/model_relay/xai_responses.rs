//! First-party xAI native Responses request differences, ported from the frozen upstream snapshot.
//! Resident routing calls this after stored overrides and exact model selection.
use serde_json::{Map, Value, json};
use std::collections::HashSet;

/// Match the authorized endpoint exactly; provider names and model names are not authority.
pub(crate) fn prepare(body: &mut Value, url: &reqwest::Url) {
    if url.host_str() == Some("api.x.ai")
        && url.path().trim_end_matches('/').ends_with("/responses")
    {
        sanitize_xai_responses_request(body);
    }
}

const RECURSIVE_UNSUPPORTED_FIELDS: &[&str] = &["external_web_access"];

const TOP_LEVEL_UNSUPPORTED_FIELDS: &[&str] = &["prompt_cache_retention", "safety_identifier"];

const XAI_UNSUPPORTED_INCLUDE_MARKERS: &[&str] = &["output.encrypted_content"];

const GROK_45_UNSUPPORTED_FIELDS: &[&str] =
    &["presence_penalty", "presencePenalty", "frequency_penalty", "frequencyPenalty", "stop"];

const XAI_SUPPORTED_TOOL_TYPES: &[&str] = &[
    "function",
    "web_search",
    "x_search",
    "image_generation",
    "collections_search",
    "file_search",
    "code_execution",
    "code_interpreter",
    "mcp",
    "shell",
];

pub(crate) fn sanitize_xai_responses_request(body: &mut Value) -> bool {
    if !body.is_object() {
        return false;
    }

    let mut changed = false;

    for field in TOP_LEVEL_UNSUPPORTED_FIELDS {
        changed |= remove_top_level_field(body, field);
    }

    changed |= filter_unsupported_include_markers(body);

    if request_targets_grok_45(body) {
        for field in GROK_45_UNSUPPORTED_FIELDS {
            changed |= remove_top_level_field(body, field);
        }
    }

    for field in RECURSIVE_UNSUPPORTED_FIELDS {
        changed |= remove_field_recursive(body, field);
    }

    changed |= promote_additional_tools(body);

    changed |= strip_null_reasoning_content(body);

    changed |= filter_unsupported_tools(body);

    changed |= normalize_xai_function_tool_parameter_schemas(body);

    changed
}

fn filter_unsupported_include_markers(body: &mut Value) -> bool {
    let mut remove_empty_include = false;
    let changed = match body.get_mut("include").and_then(Value::as_array_mut) {
        Some(include) => {
            let original_len = include.len();
            include.retain(|item| {
                !item
                    .as_str()
                    .is_some_and(|marker| XAI_UNSUPPORTED_INCLUDE_MARKERS.contains(&marker))
            });
            let changed = include.len() != original_len;
            remove_empty_include = changed && include.is_empty();
            changed
        }
        None => false,
    };

    if remove_empty_include {
        if let Some(body) = body.as_object_mut() {
            body.remove("include");
        }
    }

    changed
}

fn request_targets_grok_45(body: &Value) -> bool {
    let Some(model) = body.get("model").and_then(Value::as_str) else {
        return false;
    };
    let mut model = model.trim();
    if let Some(idx) = model.rfind('/') {
        model = model[idx + 1..].trim();
    }
    model.eq_ignore_ascii_case("grok-4.5")
}

fn remove_top_level_field(body: &mut Value, field: &str) -> bool {
    body.as_object_mut().and_then(|obj| obj.remove(field)).is_some()
}

fn remove_field_recursive(value: &mut Value, field: &str) -> bool {
    match value {
        Value::Object(map) => {
            let mut changed = map.remove(field).is_some();
            for child in map.values_mut() {
                changed |= remove_field_recursive(child, field);
            }
            changed
        }
        Value::Array(items) => {
            let mut changed = false;
            for child in items.iter_mut() {
                changed |= remove_field_recursive(child, field);
            }
            changed
        }
        _ => false,
    }
}

fn is_additional_tools_item(item: &Value) -> bool {
    item.get("type").and_then(Value::as_str).map(str::trim) == Some("additional_tools")
}

fn promote_additional_tools(body: &mut Value) -> bool {
    let input_items: Vec<Value> = match body.get("input").and_then(Value::as_array) {
        Some(arr) if arr.iter().any(is_additional_tools_item) => arr.clone(),
        _ => return false,
    };

    let mut merged: Vec<Value> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    if let Some(tools) = body.get("tools").and_then(Value::as_array) {
        for tool in tools {
            seen.insert(tool_dedup_key(tool));
            merged.push(tool.clone());
        }
    }

    let mut filtered_input: Vec<Value> = Vec::with_capacity(input_items.len());
    let mut promoted = false;
    for item in input_items {
        if is_additional_tools_item(&item) {
            if let Some(carrier_tools) = item.get("tools").and_then(Value::as_array) {
                for tool in carrier_tools {
                    if seen.insert(tool_dedup_key(tool)) {
                        merged.push(tool.clone());
                        promoted = true;
                    }
                }
            }
            continue; // carrier item dropped regardless of dedup outcome
        }
        filtered_input.push(item);
    }

    if let Some(obj) = body.as_object_mut() {
        obj.insert("input".to_string(), Value::Array(filtered_input));
        if promoted {
            obj.insert("tools".to_string(), Value::Array(merged));
        }
    }
    true
}

fn tool_dedup_key(tool: &Value) -> String {
    let tool_type = tool.get("type").and_then(Value::as_str).unwrap_or("").trim();
    if !tool_type.is_empty() {
        if let Some(name) = tool.get("name").and_then(Value::as_str) {
            let name = name.trim();
            if !name.is_empty() {
                return format!("type:{tool_type}\u{0}name:{name}");
            }
        }
        if tool_type == "mcp" {
            if let Some(label) = tool.get("server_label").and_then(Value::as_str) {
                let label = label.trim();
                if !label.is_empty() {
                    return format!("type:mcp\u{0}server_label:{label}");
                }
            }
        }
    }
    format!("json:{tool}")
}

fn strip_null_reasoning_content(body: &mut Value) -> bool {
    let Some(input) = body.get_mut("input").and_then(Value::as_array_mut) else {
        return false;
    };
    let mut changed = false;
    for item in input.iter_mut() {
        if item.get("type").and_then(Value::as_str).map(str::trim) != Some("reasoning") {
            continue;
        }
        if let Some(obj) = item.as_object_mut() {
            if matches!(obj.get("content"), Some(Value::Null)) {
                obj.remove("content");
                changed = true;
            }
        }
    }
    changed
}

fn filter_unsupported_tools(body: &mut Value) -> bool {
    let Some(tools) = body.get("tools").and_then(Value::as_array) else {
        return false;
    };
    let original_len = tools.len();
    let filtered: Vec<Value> = tools
        .iter()
        .filter(|tool| {
            let t = tool.get("type").and_then(Value::as_str).unwrap_or("").trim();
            XAI_SUPPORTED_TOOL_TYPES.contains(&t)
        })
        .cloned()
        .collect();

    let mut changed = false;
    if filtered.len() != original_len {
        if let Some(obj) = body.as_object_mut() {
            if filtered.is_empty() {
                obj.remove("tools");
            } else {
                obj.insert("tools".to_string(), Value::Array(filtered.clone()));
            }
        }
        changed = true;
    }

    if body.get("tool_choice").is_some() && should_drop_tool_choice(body, &filtered) {
        if let Some(obj) = body.as_object_mut() {
            obj.remove("tool_choice");
        }
        changed = true;
    }

    changed
}

fn should_drop_tool_choice(body: &Value, tools: &[Value]) -> bool {
    let Some(tool_choice) = body.get("tool_choice") else {
        return false;
    };
    if tools.is_empty() {
        return true;
    }
    let Some(choice) = tool_choice.as_object() else {
        return false; // "auto"/"none"/"required" string choices stay
    };
    let choice_type = choice.get("type").and_then(Value::as_str).unwrap_or("").trim();
    if choice_type.is_empty() {
        return false;
    }
    if !XAI_SUPPORTED_TOOL_TYPES.contains(&choice_type) {
        return true;
    }
    if choice_type == "function" {
        let choice_name = choice
            .get("name")
            .and_then(Value::as_str)
            .or_else(|| choice.get("function").and_then(|f| f.get("name")).and_then(Value::as_str))
            .unwrap_or("")
            .trim();
        if choice_name.is_empty() {
            return false;
        }
        let exists = tools.iter().any(|tool| {
            let t = tool.get("type").and_then(Value::as_str).unwrap_or("").trim();
            let name = tool
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| {
                    tool.get("function").and_then(|f| f.get("name")).and_then(Value::as_str)
                })
                .unwrap_or("")
                .trim();
            t == "function" && name == choice_name
        });
        return !exists;
    }
    false
}

fn xai_function_parameters_need_simplification(params: &Value) -> bool {
    match params {
        Value::Null => true,
        Value::Object(obj) if obj.is_empty() => true,
        Value::Object(obj) => {
            match obj.get("type") {
                None | Some(Value::Null) => return true,
                Some(Value::String(type_name)) if type_name != "object" => return true,
                _ => {}
            }

            for union_key in ["oneOf", "anyOf"] {
                let Some(branches) = obj.get(union_key).and_then(Value::as_array) else {
                    continue;
                };
                if branches.is_empty() {
                    continue;
                }
                if branches
                    .iter()
                    .any(|branch| branch.get("type").and_then(Value::as_str) != Some("object"))
                {
                    return true;
                }
            }

            false
        }
        _ => true,
    }
}

fn flatten_union_branches_to_object(branches: &[Value]) -> Value {
    let object_branches: Vec<&Value> = branches
        .iter()
        .filter(|branch| branch.get("type").and_then(Value::as_str) == Some("object"))
        .collect();

    if object_branches.len() == 1 {
        let mut result = object_branches[0].clone();
        if let Some(obj) = result.as_object_mut() {
            obj.insert("type".to_string(), json!("object"));
            obj.entry("properties".to_string()).or_insert_with(|| json!({}));
        }
        return result;
    }

    if !object_branches.is_empty() {
        let mut merged_properties = Map::new();
        let mut merged_required: Option<Vec<Value>> = None;
        for branch in object_branches {
            if let Some(properties) = branch.get("properties").and_then(Value::as_object) {
                for (key, value) in properties {
                    merged_properties.entry(key.clone()).or_insert_with(|| value.clone());
                }
            }
            let branch_required =
                branch.get("required").and_then(Value::as_array).cloned().unwrap_or_default();
            merged_required = Some(match merged_required {
                None => branch_required,
                Some(existing) => {
                    existing.into_iter().filter(|item| branch_required.contains(item)).collect()
                }
            });
        }

        let mut result = json!({
            "type": "object",
            "properties": Value::Object(merged_properties),
        });
        let merged_required = merged_required.unwrap_or_default();
        if !merged_required.is_empty() {
            result["required"] = Value::Array(merged_required);
        }
        return result;
    }

    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": true
    })
}

fn simplify_xai_function_parameters(params: Option<&Value>) -> Value {
    match params {
        None | Some(Value::Null) => {
            json!({"type": "object", "properties": {}, "additionalProperties": true})
        }
        Some(Value::Object(obj)) if obj.is_empty() => {
            json!({"type": "object", "properties": {}, "additionalProperties": true})
        }
        Some(Value::Object(obj)) => {
            for union_key in ["oneOf", "anyOf"] {
                if let Some(branches) = obj.get(union_key).and_then(Value::as_array) {
                    if branches
                        .iter()
                        .any(|branch| branch.get("type").and_then(Value::as_str) != Some("object"))
                    {
                        return flatten_union_branches_to_object(branches);
                    }
                }
            }

            let mut result = Value::Object(obj.clone());
            if let Some(obj) = result.as_object_mut() {
                match obj.get("type").and_then(Value::as_str) {
                    Some("object") => {}
                    _ => {
                        obj.insert("type".to_string(), json!("object"));
                        obj.entry("properties".to_string()).or_insert_with(|| json!({}));
                    }
                }
            }
            result
        }
        _ => json!({"type": "object", "properties": {}, "additionalProperties": true}),
    }
}

fn function_tool_name(tool: &Value) -> &str {
    tool.get("name")
        .and_then(Value::as_str)
        .or_else(|| {
            tool.get("function").and_then(|function| function.get("name")).and_then(Value::as_str)
        })
        .unwrap_or("")
        .trim()
}

fn is_automation_update_tool(name: &str) -> bool {
    name == "codex_app__automation_update"
        || name == "mcp__codex_app__automation_update"
        || name.ends_with("__automation_update")
}

fn rewrite_function_tool_parameters(tool: &mut Value, params: Option<&Value>) -> bool {
    let simplified = simplify_xai_function_parameters(params);
    if params == Some(&simplified) {
        return false;
    }

    if let Some(obj) = tool.as_object_mut() {
        if obj.contains_key("parameters") || params.is_some() {
            obj.insert("parameters".to_string(), simplified);
            return true;
        }
        if let Some(function) = obj.get_mut("function").and_then(Value::as_object_mut) {
            function.insert("parameters".to_string(), simplified);
            return true;
        }
    }

    false
}

fn xai_safe_empty_object_schema() -> Value {
    json!({"type": "object", "properties": {}, "additionalProperties": true})
}

fn normalize_xai_function_tool_parameters(tool: &mut Value) -> bool {
    if tool.get("type").and_then(Value::as_str) != Some("function") {
        return false;
    }

    if is_automation_update_tool(function_tool_name(tool)) {
        let safe = xai_safe_empty_object_schema();
        let needs_rewrite = {
            let current = tool
                .get("parameters")
                .or_else(|| tool.get("function").and_then(|function| function.get("parameters")));
            current != Some(&safe)
        };
        let mut changed = needs_rewrite;
        if let Some(obj) = tool.as_object_mut() {
            if needs_rewrite {
                if obj.contains_key("parameters") || obj.get("function").is_none() {
                    obj.insert("parameters".to_string(), safe);
                } else if let Some(function) =
                    obj.get_mut("function").and_then(Value::as_object_mut)
                {
                    function.insert("parameters".to_string(), safe);
                }
            }
            if obj.get("strict") == Some(&json!(true)) {
                obj.insert("strict".to_string(), json!(false));
                changed = true;
            }
            if let Some(function) = obj.get_mut("function").and_then(Value::as_object_mut) {
                if function.get("strict") == Some(&json!(true)) {
                    function.insert("strict".to_string(), json!(false));
                    changed = true;
                }
            }
        }
        return changed;
    }

    let params = tool
        .get("parameters")
        .or_else(|| tool.get("function").and_then(|function| function.get("parameters")))
        .cloned();

    let changed = match params.as_ref() {
        Some(params) if xai_function_parameters_need_simplification(params) => {
            rewrite_function_tool_parameters(tool, Some(params))
        }
        None => rewrite_function_tool_parameters(tool, None),
        _ => false,
    };

    if changed && is_automation_update_tool(function_tool_name(tool)) {
        if let Some(obj) = tool.as_object_mut() {
            if obj.get("strict") == Some(&json!(true)) {
                obj.insert("strict".to_string(), json!(false));
            }
            if let Some(function) = obj.get_mut("function").and_then(Value::as_object_mut) {
                if function.get("strict") == Some(&json!(true)) {
                    function.insert("strict".to_string(), json!(false));
                }
            }
        }
    }

    changed
}

fn normalize_xai_function_tool_parameter_schemas(body: &mut Value) -> bool {
    let Some(tools) = body.get_mut("tools").and_then(Value::as_array_mut) else {
        return false;
    };

    let mut changed = false;
    for tool in tools.iter_mut() {
        changed |= normalize_xai_function_tool_parameters(tool);
    }
    changed
}
