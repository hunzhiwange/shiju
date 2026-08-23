//! ApiQL 业务实体行为注册表，解析实体专属的虚拟字段规则。
//!
//! ApiQL business entity behavior registry.

use serde_json::{Map, Value};
use vw_api_types::apiql::ApiQlFieldConfig;

use crate::app::agent::gateway::ApiError;

use super::schema::ApiQlResolvedEntity;

pub(crate) trait ApiQlBusinessEntity: Sync {
    fn entity_key(&self) -> &'static str;

    fn resolve_virtual_field(
        &self,
        field: &ApiQlFieldConfig,
        row: &Map<String, Value>,
    ) -> Result<Option<Value>, ApiError>;
}

pub(crate) fn resolve_business_virtual_field(
    entity: ApiQlResolvedEntity<'_>,
    field: &ApiQlFieldConfig,
    row: &Map<String, Value>,
) -> Result<Option<Value>, ApiError> {
    let Some(business_entity) = business_entity(entity.entity_key()) else {
        return Ok(None);
    };
    business_entity.resolve_virtual_field(field, row)
}

pub(crate) fn prepare_sku_title(spec_title: &str) -> Option<String> {
    let trimmed = spec_title.trim();
    if trimmed.is_empty() {
        return Some(String::new());
    }
    let parsed: Value = serde_json::from_str(trimmed).ok()?;
    let values: Vec<Value> = match parsed {
        Value::Array(items) => items,
        Value::Object(object) => object.into_values().collect(),
        _ => return Some(String::new()),
    };
    let parts = values
        .iter()
        .filter_map(Value::as_str)
        .filter_map(|value| value.split(':').nth(1).map(str::trim))
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    Some(parts.join(" "))
}

fn business_entity(entity_key: &str) -> Option<&'static dyn ApiQlBusinessEntity> {
    let entities: [&'static dyn ApiQlBusinessEntity; 1] = [&PRODUCT_ENTITY];
    entities.into_iter().find(|business_entity| business_entity.entity_key() == entity_key)
}

static PRODUCT_ENTITY: ProductEntity = ProductEntity;

struct ProductEntity;

impl ProductEntity {
    const ENTITY_KEY: &'static str = "product:product";

    fn resolve_sku_title(&self, row: &Map<String, Value>) -> Value {
        let spec_title = row_string(row, &["spec_title", "specTitle"]).unwrap_or_default();
        if spec_title.trim().is_empty() {
            return Value::String(String::new());
        }
        Value::String(prepare_sku_title(spec_title).unwrap_or_default())
    }

    fn resolve_full_title(&self, row: &Map<String, Value>) -> Value {
        let title = row_string(row, &["title"]).unwrap_or_default();
        let spec_title = row_string(row, &["spec_title", "specTitle"]).unwrap_or_default();
        if spec_title.trim().is_empty() {
            return Value::String(title.to_string());
        }
        let sku_title = prepare_sku_title(spec_title).unwrap_or_default();
        if sku_title.is_empty() {
            Value::String(title.to_string())
        } else {
            Value::String(format!("{title} {sku_title}"))
        }
    }
}

impl ApiQlBusinessEntity for ProductEntity {
    fn entity_key(&self) -> &'static str {
        Self::ENTITY_KEY
    }

    fn resolve_virtual_field(
        &self,
        field: &ApiQlFieldConfig,
        row: &Map<String, Value>,
    ) -> Result<Option<Value>, ApiError> {
        match field.name.as_str() {
            "skuTitle" | "sku_title" => Ok(Some(self.resolve_sku_title(row))),
            "fullTitle" | "full_title" => Ok(Some(self.resolve_full_title(row))),
            _ => Ok(None),
        }
    }
}

fn row_string<'a>(row: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| row.get(*key).and_then(Value::as_str))
}

#[cfg(test)]
#[path = "apiql_entities_tests.rs"]
mod apiql_entities_tests;
