//! Workflow 变量池。

use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct VariablePool {
    values: BTreeMap<String, Value>,
}

impl VariablePool {
    pub fn from_values(values: BTreeMap<String, Value>) -> Self {
        Self { values }
    }

    pub fn values(&self) -> BTreeMap<String, Value> {
        self.values.clone()
    }

    pub fn insert_selector(&mut self, selector: &[String], value: Value) {
        if selector.is_empty() {
            return;
        }
        self.values.insert(selector_key(selector), value);
    }

    pub fn insert_node_output(&mut self, node_id: &str, key: &str, value: Value) {
        self.values.insert(format!("{node_id}.{key}"), value);
    }

    pub fn get_selector(&self, selector: &[String]) -> Option<&Value> {
        if selector.is_empty() {
            return None;
        }
        let full_key = selector_key(selector);
        if let Some(value) = self.values.get(&full_key) {
            return Some(value);
        }
        for prefix_len in (1..selector.len()).rev() {
            let prefix_key = selector_key(&selector[..prefix_len]);
            if let Some(value) = self.values.get(&prefix_key) {
                return value_at_path(value, &selector[prefix_len..]);
            }
        }
        None
    }

    pub fn node_outputs(&self, node_id: &str) -> BTreeMap<String, Value> {
        let prefix = format!("{node_id}.");
        self.values
            .iter()
            .filter_map(|(key, value)| {
                key.strip_prefix(&prefix).map(|name| (name.to_string(), value.clone()))
            })
            .collect()
    }
}

pub fn selector_from_value(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| items.iter().filter_map(|item| item.as_str().map(ToOwned::to_owned)).collect())
        .unwrap_or_default()
}

pub(crate) fn selector_key(selector: &[String]) -> String {
    selector.join(".")
}

fn value_at_path<'a>(value: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut current = value;
    for part in path {
        current = value_at_part(current, part)?;
    }
    Some(current)
}

fn value_at_part<'a>(value: &'a Value, part: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in part.split('.').filter(|segment| !segment.is_empty()) {
        current = value_at_segment(current, segment)?;
    }
    Some(current)
}

fn value_at_segment<'a>(value: &'a Value, segment: &str) -> Option<&'a Value> {
    let mut current = value;
    let mut rest = segment;

    if !rest.starts_with('[') {
        let bracket_index = rest.find('[').unwrap_or(rest.len());
        let field = &rest[..bracket_index];
        current = current.get(field)?;
        rest = &rest[bracket_index..];
    }

    while !rest.is_empty() {
        let after_open = rest.strip_prefix('[')?;
        let close_index = after_open.find(']')?;
        let index = after_open[..close_index].parse::<usize>().ok()?;
        current = current.get(index)?;
        rest = &after_open[close_index + 1..];
    }

    Some(current)
}

#[cfg(test)]
#[path = "variables_tests.rs"]
mod variables_tests;
