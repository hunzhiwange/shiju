//! APIQL 四种规范只读查询命令。

use std::io::Write;

use anyhow::{Result, Context};
use clap::Subcommand;
use serde_json::json;
use vw_gateway_client::ApiQlQuery;

use super::context::ApiContext;
use super::output;

#[derive(Subcommand, Debug)]
pub(super) enum ApiQlCommands {
    /// 查询实体列表。
    List {
        #[arg(value_parser = required)]
        entity_class: String,
        #[arg(long = "param")]
        parameters: Vec<String>,
    },
    /// 查询实体行列表，不附加分页投影。
    ListOnly {
        #[arg(value_parser = required)]
        entity_class: String,
        #[arg(long = "param")]
        parameters: Vec<String>,
    },
    /// 查询实体计数。
    ListCount {
        #[arg(value_parser = required)]
        entity_class: String,
        #[arg(long = "param")]
        parameters: Vec<String>,
    },
    /// 查询实体分页。
    ListPage {
        #[arg(value_parser = required)]
        entity_class: String,
        #[arg(long = "param")]
        parameters: Vec<String>,
    },
}

pub(super) async fn run(
    command: ApiQlCommands,
    context: &ApiContext,
    writer: &mut impl Write,
) -> Result<()> {
    let client = context.client();
    let response = match command {
        ApiQlCommands::List { entity_class, parameters } => {
            let query = query(parameters)?;
            json!(client.apiql_list(&entity_class, &query).await.map_err(anyhow::Error::msg)?)
        }
        ApiQlCommands::ListOnly { entity_class, parameters } => {
            let query = query(parameters)?;
            json!(client.apiql_list_only(&entity_class, &query).await.map_err(anyhow::Error::msg)?)
        }
        ApiQlCommands::ListCount { entity_class, parameters } => {
            let query = query(parameters)?;
            json!(client.apiql_list_count(&entity_class, &query).await.map_err(anyhow::Error::msg)?)
        }
        ApiQlCommands::ListPage { entity_class, parameters } => {
            let query = query(parameters)?;
            json!(client.apiql_list_page(&entity_class, &query).await.map_err(anyhow::Error::msg)?)
        }
    };
    output::write_json(writer, &response, context.output_format())
}

fn query(parameters: Vec<String>) -> Result<ApiQlQuery> {
    ApiQlQuery::from_parameters(parameters).map_err(anyhow::Error::msg).context("invalid APIQL query parameter")
}

fn required(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("APIQL entity class must not be empty".to_string());
    }
    Ok(value.to_string())
}
