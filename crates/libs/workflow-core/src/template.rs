//! Dify 模板渲染。

use super::variables::VariablePool;
use minijinja::Environment;
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

static TEMPLATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{#([^#]+)#\}\}").expect("valid workflow template regex"));

pub fn render_template(template: &str, pool: &VariablePool) -> String {
    TEMPLATE_RE
        .replace_all(template, |captures: &regex::Captures<'_>| {
            let selector = captures[1]
                .split('.')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            pool.get_selector(&selector).map(value_to_text).unwrap_or_default()
        })
        .into_owned()
}

pub fn render_jinja_value_template(
    template: &str,
    variables: &std::collections::BTreeMap<String, Value>,
) -> Result<String, String> {
    let variables = variables
        .iter()
        .map(|(key, value)| Ok((key.clone(), jinja_value(value)?)))
        .collect::<Result<std::collections::BTreeMap<_, _>, String>>()?;
    Environment::new()
        .render_str(template, variables)
        .map_err(|error| format!("template 节点渲染失败: {error}"))
}

// serde_json's arbitrary_precision feature serializes numbers as private structs.
// Convert JSON recursively so feature unification cannot expose those structs to Jinja.
fn jinja_value(value: &Value) -> Result<minijinja::Value, String> {
    match value {
        Value::Number(number) => number
            .as_i64()
            .map(minijinja::Value::from)
            .or_else(|| number.as_u64().map(minijinja::Value::from))
            .or_else(|| number.as_f64().map(minijinja::Value::from))
            .ok_or_else(|| "template 节点渲染失败: 数字超出模板支持范围".to_string()),
        Value::Array(items) => items
            .iter()
            .map(jinja_value)
            .collect::<Result<Vec<_>, _>>()
            .map(|items| items.into_iter().collect()),
        Value::Object(items) => items
            .iter()
            .map(|(key, value)| Ok((key.clone(), jinja_value(value)?)))
            .collect::<Result<std::collections::BTreeMap<_, _>, String>>()
            .map(|items| items.into_iter().collect()),
        _ => Ok(minijinja::Value::from_serialize(value)),
    }
}

pub fn value_to_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(_) | Value::Object(_) => {
            serde_json::to_string(value).unwrap_or_else(|_| String::new())
        }
    }
}

#[cfg(test)]
#[path = "template_tests.rs"]
mod template_tests;
