//! ApiQL 只读列表兼容响应与实体配置 DTO。
//!
//! 本模块只表达跨 crate 边界上的协议形状，不承载查询构建或运行时行为。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// ApiQL V1 只读列表入口模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiQlListMode {
    Index,
    IndexOnly,
    IndexCount,
    IndexPage,
}

/// PHP `Leevel\Page\Page::toArray()` 兼容分页字段。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiQlPageDto {
    pub per_page: u32,
    pub current_page: u32,
    pub total_page: Option<u64>,
    pub total_record: Option<u64>,
    pub total_macro: u64,
    pub from: u64,
    pub to: Option<u64>,
}

/// ApiQL 列表响应，顶层结果字段保持为 PHP 兼容的 `data`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiQlResponse<T = Vec<Value>> {
    pub data: T,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<ApiQlPageDto>,
    #[serde(default, rename = "#page", skip_serializing_if = "Option::is_none")]
    pub page_comment: Option<String>,
    #[serde(default, rename = "#data", skip_serializing_if = "Option::is_none")]
    pub data_comment: Option<String>,
    #[serde(default, rename = "#next_cursor", skip_serializing_if = "Option::is_none")]
    pub next_cursor_comment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// ApiQL 实体字段配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ApiQlFieldConfig {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub column_struct: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validator: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validator_messages: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_class: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub readonly: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub show_prop_black: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub operation_log: bool,
    #[serde(default)]
    pub searchable: bool,
    #[serde(default)]
    pub sortable: bool,
    #[serde(default)]
    pub virtual_field: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub virtual_column: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_one: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_key: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub meta: BTreeMap<String, Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// ApiQL 实体关系配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ApiQlRelationConfig {
    pub name: String,
    pub entity_class: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreign_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_key: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub meta: BTreeMap<String, Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// ApiQL PHP condition scope 元数据。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ApiQlConditionScopeConfig {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition_field: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// ApiQL 实体元数据配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ApiQlEntityConfig {
    pub entity_class: String,
    pub connection_id: String,
    pub table: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connect: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_increment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soft_delete_column: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_log_entity: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub unique_indexes: BTreeMap<String, Value>,
    #[serde(default)]
    pub fields: Vec<ApiQlFieldConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<ApiQlFieldConfig>,
    #[serde(default)]
    pub search_fields: Vec<String>,
    #[serde(default)]
    pub virtual_fields: Vec<ApiQlFieldConfig>,
    #[serde(default)]
    pub relations: Vec<ApiQlRelationConfig>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub condition_scopes: BTreeMap<String, ApiQlConditionScopeConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lifecycle_events: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

fn is_false(value: &bool) -> bool {
    !*value
}
