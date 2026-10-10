//! APIQL 四种只读列表模式。

use serde_json::Value;
use vw_api_types::apiql::{ApiQlListMode, ApiQlResponse};

use super::GatewayClient;

/// 已验证、保持顺序的 APIQL 查询参数。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ApiQlQuery {
    pairs: Vec<(String, String)>,
}

impl ApiQlQuery {
    /// 从 `key=value` 参数构造查询，分页参数必须是正整数。
    pub fn from_parameters(
        parameters: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, String> {
        let mut pairs = Vec::new();
        for parameter in parameters {
            let parameter = parameter.into();
            let (key, value) =
                parameter.split_once('=').ok_or_else(|| invalid_parameter(&parameter))?;
            if !valid_query_key(key) {
                return Err(invalid_parameter(&parameter));
            }
            if matches!(key, "limit" | "page" | "page_size")
                && value.parse::<u32>().ok().filter(|value| *value > 0).is_none()
            {
                return Err(invalid_parameter(&parameter));
            }
            pairs.push((key.to_string(), value.to_string()));
        }
        Ok(Self { pairs })
    }
}

impl GatewayClient {
    pub async fn apiql_list(
        &self,
        entity_class: &str,
        query: &ApiQlQuery,
    ) -> Result<ApiQlResponse<Value>, String> {
        self.apiql_query(entity_class, query, ApiQlListMode::Index).await
    }

    pub async fn apiql_list_only(
        &self,
        entity_class: &str,
        query: &ApiQlQuery,
    ) -> Result<ApiQlResponse<Value>, String> {
        self.apiql_query(entity_class, query, ApiQlListMode::IndexOnly).await
    }

    pub async fn apiql_list_count(
        &self,
        entity_class: &str,
        query: &ApiQlQuery,
    ) -> Result<ApiQlResponse<Value>, String> {
        self.apiql_query(entity_class, query, ApiQlListMode::IndexCount).await
    }

    pub async fn apiql_list_page(
        &self,
        entity_class: &str,
        query: &ApiQlQuery,
    ) -> Result<ApiQlResponse<Value>, String> {
        self.apiql_query(entity_class, query, ApiQlListMode::IndexPage).await
    }

    async fn apiql_query(
        &self,
        entity_class: &str,
        query: &ApiQlQuery,
        mode: ApiQlListMode,
    ) -> Result<ApiQlResponse<Value>, String> {
        if entity_class.trim().is_empty() {
            return Err("APIQL entity class must not be empty".to_string());
        }
        let suffix = match mode {
            ApiQlListMode::Index => "",
            ApiQlListMode::IndexOnly => "/list-only",
            ApiQlListMode::IndexCount => "/list-count",
            ApiQlListMode::IndexPage => "/list-page",
        };
        let path = format!("/apiQL/{}{suffix}", encode_path_segment(entity_class));
        self.get_json(&path, &query.pairs).await
    }
}

fn valid_query_key(key: &str) -> bool {
    !key.is_empty()
        && key.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':' | b'[' | b']')
        })
}

fn invalid_parameter(parameter: &str) -> String {
    format!("invalid APIQL query parameter '{parameter}'")
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
#[path = "apiql_api_tests.rs"]
mod apiql_api_tests;
