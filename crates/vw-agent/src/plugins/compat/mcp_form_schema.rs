//! Validate the frozen MCP primitive form dialect; never fetch remote schemas.
use anyhow::{Result, bail, ensure};
use serde::Deserialize;
use serde_json::{Number, Value};
use std::collections::{BTreeMap, HashSet};

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct FormSchema {
    #[serde(rename = "type")]
    kind: String,
    properties: BTreeMap<String, Field>,
    #[serde(default)]
    required: Vec<String>,
    title: Option<String>,
    description: Option<String>,
    additional_properties: Option<bool>,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Kind {
    String,
    Number,
    Integer,
    Boolean,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Field {
    #[serde(rename = "type")]
    kind: Kind,
    title: Option<String>,
    description: Option<String>,
    min_length: Option<u64>,
    max_length: Option<u64>,
    format: Option<String>,
    #[serde(rename = "enum")]
    choices: Option<Vec<String>>,
    enum_names: Option<Vec<String>>,
    minimum: Option<Number>,
    maximum: Option<Number>,
    default: Option<Value>,
}

fn number_cmp(a: &Number, b: &Number) -> std::cmp::Ordering {
    let integer = |n: &Number| n.as_i64().map(i128::from).or_else(|| n.as_u64().map(i128::from));
    match (integer(a), integer(b)) {
        (Some(a), Some(b)) => a.cmp(&b),
        _ => a
            .as_f64()
            .expect("JSON number")
            .partial_cmp(&b.as_f64().expect("JSON number"))
            .expect("finite JSON numbers"),
    }
}

impl FormSchema {
    pub(super) fn parse(value: &Value) -> Result<Self> {
        fn has_null(value: &Value) -> bool {
            match value {
                Value::Null => true,
                Value::Object(object) => object.values().any(has_null),
                Value::Array(array) => array.iter().any(has_null),
                _ => false,
            }
        }
        // Option<T> would otherwise turn invalid null constraints into absent constraints.
        ensure!(!has_null(value), "MCP primitive form schema cannot contain null constraints");
        let schema: Self = serde_json::from_value(value.clone())
            .map_err(|_| anyhow::anyhow!("unsupported or invalid MCP form schema"))?;
        ensure!(schema.kind == "object", "MCP form schema must be an object");
        ensure!(schema.properties.len() <= 64, "MCP form has too many fields");
        ensure!(schema.additional_properties != Some(true), "MCP forms require declared fields");
        let _ = (&schema.title, &schema.description);
        let mut required = HashSet::new();
        for name in &schema.required {
            ensure!(
                schema.properties.contains_key(name) && required.insert(name),
                "invalid MCP form required fields"
            );
        }
        for (name, field) in &schema.properties {
            ensure!(!name.is_empty() && name.len() <= 256, "invalid MCP form field name");
            field.validate_definition()?;
        }
        Ok(schema)
    }

    pub(super) fn validate(&self, value: &Value) -> Result<()> {
        ensure!(serde_json::to_vec(value)?.len() <= 65_536, "MCP form response is too large");
        let object = value
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("MCP form response must be an object"))?;
        ensure!(
            self.required.iter().all(|name| object.contains_key(name)),
            "MCP form response is missing required fields"
        );
        for (key, value) in object {
            let field = self
                .properties
                .get(key)
                .ok_or_else(|| anyhow::anyhow!("MCP form response contains undeclared fields"))?;
            field.validate(value)?;
        }
        Ok(())
    }
}

impl Field {
    fn validate_definition(&self) -> Result<()> {
        let _ = (&self.title, &self.description);
        if self.kind != Kind::String {
            ensure!(
                self.min_length.is_none()
                    && self.max_length.is_none()
                    && self.format.is_none()
                    && self.choices.is_none()
                    && self.enum_names.is_none(),
                "invalid constraints for MCP form field type"
            );
        }
        if !matches!(self.kind, Kind::Number | Kind::Integer) {
            ensure!(
                self.minimum.is_none() && self.maximum.is_none(),
                "invalid numeric MCP form constraints"
            );
        }
        if let (Some(min), Some(max)) = (self.min_length, self.max_length) {
            ensure!(min <= max, "invalid MCP form length range");
        }
        if let (Some(min), Some(max)) = (&self.minimum, &self.maximum) {
            ensure!(!number_cmp(min, max).is_gt(), "invalid MCP form numeric range");
        }
        if let Some(format) = &self.format {
            ensure!(
                matches!(format.as_str(), "email" | "uri" | "date" | "date-time"),
                "unsupported MCP form string format"
            );
        }
        if let Some(choices) = &self.choices {
            let unique = choices.iter().collect::<HashSet<_>>();
            ensure!(
                !choices.is_empty() && choices.len() <= 64 && unique.len() == choices.len(),
                "invalid MCP form enum"
            );
            ensure!(
                self.enum_names.as_ref().is_none_or(|names| names.len() == choices.len()),
                "invalid MCP form enum labels"
            );
        } else {
            ensure!(self.enum_names.is_none(), "MCP form enum labels require enum values");
        }
        // Defaults are presentation hints only. No code here supplies a user answer.
        if let Some(default) = &self.default {
            self.validate(default)?;
        }
        Ok(())
    }

    fn validate(&self, value: &Value) -> Result<()> {
        let invalid = || anyhow::anyhow!("MCP form value does not match requested schema");
        match self.kind {
            Kind::String => {
                let value = value.as_str().ok_or_else(invalid)?;
                let len = value.chars().count() as u64;
                ensure!(
                    self.min_length.is_none_or(|min| len >= min)
                        && self.max_length.is_none_or(|max| len <= max),
                    "MCP form string length is outside its range"
                );
                ensure!(
                    self.choices
                        .as_ref()
                        .is_none_or(|choices| choices.iter().any(|choice| choice == value)),
                    "MCP form value is outside its enum"
                );
                let valid = match self.format.as_deref() {
                    None => true,
                    Some("email") => value.parse::<lettre::Address>().is_ok(),
                    Some("uri") => url::Url::parse(value).is_ok(),
                    Some("date") => {
                        value.len() == 10
                            && chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                                .is_ok_and(|date| date.format("%Y-%m-%d").to_string() == value)
                    }
                    Some("date-time") => chrono::DateTime::parse_from_rfc3339(value).is_ok(),
                    _ => false,
                };
                ensure!(valid, "MCP form string has an invalid format");
            }
            Kind::Number | Kind::Integer => {
                let number = value.as_number().ok_or_else(invalid)?;
                if self.kind == Kind::Integer
                    && !number.is_i64()
                    && !number.is_u64()
                    && number.as_f64().is_none_or(|n| n.fract() != 0.0)
                {
                    bail!("MCP form value must be an integer");
                }
                ensure!(
                    self.minimum.as_ref().is_none_or(|min| !number_cmp(number, min).is_lt())
                        && self.maximum.as_ref().is_none_or(|max| !number_cmp(number, max).is_gt()),
                    "MCP form number is outside its range"
                );
            }
            Kind::Boolean => ensure!(value.is_boolean(), "MCP form value must be a boolean"),
        }
        Ok(())
    }
}
