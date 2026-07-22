//! ApiQL 到 AI-DATA 只读 SQL 执行器的桥接。

use std::collections::BTreeMap;

use serde_json::Value;
use vw_api_types::data::{AiDataConnectionDto, AiDataCountMode, AiDataQueryResponse};

use super::super::data::runtime::{ReadOnlySqlQuery, execute_read_only_sql_query};

#[derive(Debug, Clone)]
pub(crate) struct ApiQlDataQuery {
    pub(crate) sql: String,
    pub(crate) count_sql: Option<String>,
    pub(crate) params: BTreeMap<String, Value>,
    pub(crate) page: u32,
    pub(crate) limit: u32,
    pub(crate) count_mode: AiDataCountMode,
    pub(crate) order_by: Option<String>,
    pub(crate) debug: bool,
}

impl Default for ApiQlDataQuery {
    fn default() -> Self {
        Self {
            sql: String::new(),
            count_sql: None,
            params: BTreeMap::new(),
            page: 1,
            limit: 25,
            count_mode: AiDataCountMode::Disabled,
            order_by: None,
            debug: false,
        }
    }
}

pub(crate) async fn execute_apiql_data_query(
    connection: AiDataConnectionDto,
    query: ApiQlDataQuery,
) -> Result<AiDataQueryResponse, String> {
    execute_read_only_sql_query(
        connection,
        ReadOnlySqlQuery {
            sql: query.sql,
            count_sql: query.count_sql,
            params: query.params,
            page: query.page,
            limit: query.limit,
            count_mode: query.count_mode,
            order_by: query.order_by,
            debug: query.debug,
        },
    )
    .await
}
