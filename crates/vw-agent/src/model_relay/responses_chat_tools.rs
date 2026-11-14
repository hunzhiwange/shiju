//! Request-local mapping between Responses tools and Chat function names.
use super::{UNSUPPORTED, arguments, string};
use axum::http::StatusCode;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, hash_map::Entry};

#[derive(PartialEq, Eq, Debug)]
struct Tool {
    name: String,
    namespace: Option<String>,
    custom: bool,
    search: bool,
}

#[derive(Default, PartialEq, Eq, Debug)]
pub(in crate::model_relay) struct ChatTools(HashMap<String, Tool>);

fn namespace(value: &Value) -> Result<Option<&str>, StatusCode> {
    value
        .get("namespace")
        .filter(|value| !value.is_null())
        .map(|value| value.as_str().filter(|name| !name.is_empty()).ok_or(StatusCode::BAD_REQUEST))
        .transpose()
}

fn chat_name(name: &str, namespace: Option<&str>) -> String {
    let Some(namespace) = namespace else {
        return name.into();
    };
    let digest = Sha256::digest(serde_json::to_vec(&(namespace, name)).expect("string tuple"));
    // Hash the complete names since MCP namespaces can exceed Chat's 64-byte limit.
    let prefix: String = name
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
        .take(27)
        .collect();
    format!("ns_{}_{prefix}", hex::encode(&digest[..16]))
}

impl ChatTools {
    pub(super) fn definitions(&mut self, tools: &Value) -> Result<Value, StatusCode> {
        let mut chat = Vec::new();
        for tool in tools.as_array().ok_or(StatusCode::BAD_REQUEST)? {
            if tool["type"] == "namespace" {
                super::object_keys(tool, &["type", "name", "description", "tools"])?;
                let namespace = string(tool, "name")?;
                let description = tool["description"].as_str().unwrap_or_default();
                for member in tool["tools"].as_array().ok_or(StatusCode::BAD_REQUEST)? {
                    let mut definition = self.definition(member, Some(namespace))?;
                    if !description.is_empty() {
                        let member_description =
                            definition["function"]["description"].as_str().unwrap_or_default();
                        definition["function"]["description"] =
                            format!("{description}\n{member_description}").into();
                    }
                    chat.push(definition);
                }
            } else {
                chat.push(self.definition(tool, None)?);
            }
        }
        Ok(chat.into())
    }

    fn definition(&mut self, tool: &Value, namespace: Option<&str>) -> Result<Value, StatusCode> {
        let mut function = match tool["type"].as_str() {
            Some("function") => {
                super::object_keys(tool, &["type", "name", "description", "parameters", "strict"])?;
                if !tool["parameters"].is_object() {
                    return Err(StatusCode::BAD_REQUEST);
                }
                let mut function = tool.as_object().cloned().ok_or(StatusCode::BAD_REQUEST)?;
                function.remove("type");
                Value::Object(function)
            }
            Some("custom") => {
                super::object_keys(tool, &["type", "name", "description", "format"])?;
                let description = tool
                    .get("description")
                    .map(|value| value.as_str().ok_or(StatusCode::BAD_REQUEST))
                    .transpose()?
                    .unwrap_or_default();
                let mut description = format!(
                    "{description}\nPass the original tool input verbatim in the input field."
                );
                if let Some(format) = tool.get("format") {
                    description.push_str(&format!(" Original input format: {format}"));
                }
                json!({"description": description, "parameters": {
                    "type": "object", "properties": {"input": {"type": "string"}},
                    "required": ["input"], "additionalProperties": false
                }})
            }
            _ => return Err(UNSUPPORTED),
        };
        let name = string(tool, "name")?;
        let name_on_wire = chat_name(name, namespace);
        let custom = tool["type"] == "custom";
        let Entry::Vacant(entry) = self.0.entry(name_on_wire.clone()) else {
            return Err(UNSUPPORTED);
        };
        entry.insert(Tool {
            name: name.into(),
            namespace: namespace.map(str::to_owned),
            custom,
            search: false,
        });
        function["name"] = name_on_wire.into();
        Ok(json!({"type":"function","function":function}))
    }

    pub(in crate::model_relay) fn mark_search(&mut self) {
        if let Some(tool) = self.0.get_mut("tool_search") {
            tool.search = true;
        }
    }

    pub(super) fn choice(&self, choice: &Value) -> Result<Value, StatusCode> {
        super::object_keys(choice, &["type", "name", "namespace"])?;
        let name = string(choice, "name")?;
        let namespace = namespace(choice)?;
        let wire = chat_name(name, namespace);
        match choice["type"].as_str() {
            Some("function") if self.0.get(&wire).is_some_and(|tool| !tool.custom) => {}
            Some("custom") if self.0.get(&wire).is_some_and(|tool| tool.custom) => {}
            _ => return Err(UNSUPPORTED),
        }
        Ok(json!({"type":"function","function":{"name":wire}}))
    }

    pub(in crate::model_relay) fn replay(&self, item: &Value) -> Result<Value, StatusCode> {
        let custom = item["type"] == "custom_tool_call";
        let arguments = if custom {
            super::object_keys(
                item,
                &["type", "id", "call_id", "name", "input", "status", "namespace"],
            )?;
            let input = item["input"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
            json!({"input": input}).to_string()
        } else {
            super::object_keys(
                item,
                &["type", "id", "call_id", "name", "arguments", "status", "namespace"],
            )?;
            arguments(item)?;
            string(item, "arguments")?.into()
        };
        let name = string(item, "name")?;
        let namespace = namespace(item)?;
        let wire = chat_name(name, namespace);
        if self.0.get(&wire).is_some_and(|tool| {
            tool.custom != custom || tool.name != name || tool.namespace.as_deref() != namespace
        }) {
            return Err(UNSUPPORTED);
        }
        Ok(json!({"id":string(item,"call_id")?,"type":"function",
            "function":{"name":wire,"arguments":arguments}}))
    }

    pub(super) fn output(
        &self,
        block: &Value,
        call: &Value,
        id: String,
        status: &str,
    ) -> Result<Value, StatusCode> {
        let name = string(block, "name")?;
        let tool = self.0.get(name);
        let mut item = if tool.is_some_and(|tool| tool.search) {
            let arguments: Value = serde_json::from_str(
                call["function"]["arguments"].as_str().ok_or(StatusCode::BAD_GATEWAY)?,
            )
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
            if !arguments.is_object() {
                return Err(StatusCode::BAD_GATEWAY);
            }
            json!({"type":"tool_search_call","id":id,"status":status,
                "call_id":block["id"],"execution":"client","arguments":arguments})
        } else if let Some(tool) = tool.filter(|tool| tool.custom) {
            let input = &block["input"];
            super::object_keys(input, &["input"]).map_err(|_| StatusCode::BAD_GATEWAY)?;
            let input = input["input"].as_str().ok_or(StatusCode::BAD_GATEWAY)?;
            json!({"type":"custom_tool_call","id":id,"status":status,
                "call_id":block["id"],"name":tool.name,"input":input})
        } else {
            json!({"type":"function_call","id":id,"status":status,
                "call_id":block["id"],"name":tool.map_or(name, |tool| &tool.name),
                "arguments":call["function"]["arguments"]})
        };
        if let Some(namespace) = tool.and_then(|tool| tool.namespace.as_ref()) {
            item["namespace"] = namespace.as_str().into();
        }
        Ok(item)
    }
}
