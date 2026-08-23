//! Cron、集成、记忆与诊断的远程 Gateway 运维命令。

use std::io::{Read, Write};

use anyhow::{Context, Result};
use clap::Subcommand;
use serde_json::json;
use vw_gateway_client::{CronAddRequest, CronUpdateRequest};

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

/// 远程运维接口族。
#[derive(Subcommand, Debug)]
pub(super) enum OperationsCommands {
    /// 管理远程 Cron 任务与运行记录。
    Cron {
        /// Cron 操作。
        #[command(subcommand)]
        command: CronOperationsCommands,
    },
    /// 查询集成并受控更新凭据。
    Integrations {
        /// 集成操作。
        #[command(subcommand)]
        command: IntegrationOperationsCommands,
    },
    /// 查询、保存或删除远程记忆。
    Memory {
        /// 记忆操作。
        #[command(subcommand)]
        command: MemoryOperationsCommands,
    },
    /// 执行 Gateway 诊断。
    Doctor {
        /// 诊断协议操作。
        #[command(subcommand)]
        command: DoctorOperationsCommands,
    },
}

/// 远程 Cron 操作。
#[derive(Subcommand, Debug)]
pub(super) enum CronOperationsCommands {
    /// 列出 Cron 任务。
    List,
    /// 列出指定任务的运行记录。
    Runs {
        /// Cron 任务 ID。
        #[arg(value_parser = parse_required_value)]
        job_id: String,
    },
    /// 从 JSON 输入创建 Cron 任务。
    Add {
        /// JSON 来源：文件、`-` stdin 或 `env:NAME`。
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
    /// 从 JSON 输入更新 Cron 任务，固定使用规范 PATCH 路由。
    Update {
        /// Cron 任务 ID。
        #[arg(value_parser = parse_required_value)]
        job_id: String,
        /// JSON 来源：文件、`-` stdin 或 `env:NAME`。
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
    /// 删除 Cron 任务。
    Delete {
        /// Cron 任务 ID。
        #[arg(value_parser = parse_required_value)]
        job_id: String,
    },
}

/// 远程集成操作。
#[derive(Subcommand, Debug)]
pub(super) enum IntegrationOperationsCommands {
    /// 列出集成及其状态。
    List,
    /// 读取集成设置与配置修订版本。
    Settings,
    /// 从安全 JSON 输入更新集成凭据。
    Credentials {
        /// 集成稳定标识。
        #[arg(value_parser = parse_required_value)]
        integration_id: String,
        /// JSON 来源：文件、`-` stdin 或 `env:NAME`；不得直接传凭据。
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
}

/// 远程记忆操作。
#[derive(Subcommand, Debug)]
pub(super) enum MemoryOperationsCommands {
    /// 语义查询、按类别列出或列出全部记忆。
    Query {
        /// 语义查询文本。
        #[arg(long, value_parser = parse_optional_non_empty)]
        query: Option<String>,
        /// 记忆类别。
        #[arg(long, value_parser = parse_optional_non_empty)]
        category: Option<String>,
    },
    /// 从 JSON 输入保存记忆。
    Store {
        /// JSON 来源：文件、`-` stdin 或 `env:NAME`。
        #[arg(long, value_parser = parse_required_value)]
        input: String,
    },
    /// 删除指定键的记忆。
    Delete {
        /// 记忆键。
        #[arg(value_parser = parse_required_value)]
        key: String,
    },
}

/// Gateway 诊断的两个文档化 HTTP 方法。
#[derive(Subcommand, Debug)]
pub(super) enum DoctorOperationsCommands {
    /// 通过 GET 执行诊断。
    Get,
    /// 通过 POST 执行诊断。
    Post,
}

impl OperationsCommands {
    pub(super) fn dangerous_operation(
        &self,
        _directory: Option<&str>,
        platform: &str,
        company: &str,
    ) -> Option<String> {
        let tenant = format!("{platform}/{company}");
        match self {
            Self::Cron { command: CronOperationsCommands::Delete { job_id } } => {
                Some(format!("delete cron job '{job_id}' in tenant {tenant}"))
            }
            Self::Integrations {
                command: IntegrationOperationsCommands::Credentials { integration_id, .. },
            } => Some(format!(
                "update credentials for integration '{integration_id}' in tenant {tenant}"
            )),
            Self::Memory { command: MemoryOperationsCommands::Delete { key } } => {
                Some(format!("delete memory '{key}' in tenant {tenant}"))
            }
            Self::Cron { .. }
            | Self::Integrations { .. }
            | Self::Memory { .. }
            | Self::Doctor { .. } => None,
        }
    }
}

pub(super) async fn run<R: Read, W: Write>(
    command: OperationsCommands,
    context: &ApiContext,
    stdin: &mut R,
    writer: &mut W,
) -> Result<()> {
    let client = context.client();
    let response = match command {
        OperationsCommands::Cron { command } => match command {
            CronOperationsCommands::List => {
                json!(client.cron_jobs_get().await.map_err(anyhow::Error::msg)?)
            }
            CronOperationsCommands::Runs { job_id } => {
                json!(client.cron_job_runs_get(&job_id).await.map_err(anyhow::Error::msg)?)
            }
            CronOperationsCommands::Add { input } => {
                let request = parse_input::<CronAddRequest>(&input, stdin, "Cron add")?;
                json!(client.cron_job_add(&request).await.map_err(anyhow::Error::msg)?)
            }
            CronOperationsCommands::Update { job_id, input } => {
                let request = parse_input::<CronUpdateRequest>(&input, stdin, "Cron update")?;
                json!(
                    client
                        .cron_job_update(&job_id, &request)
                        .await
                        .map_err(anyhow::Error::msg)?
                )
            }
            CronOperationsCommands::Delete { job_id } => {
                client.cron_job_delete(&job_id).await.map_err(anyhow::Error::msg)?;
                json!({"deleted": true})
            }
        },
        OperationsCommands::Integrations { command } => match command {
            IntegrationOperationsCommands::List => {
                client.integrations_get().await.map_err(anyhow::Error::msg)?
            }
            IntegrationOperationsCommands::Settings => {
                client.integration_settings_get().await.map_err(anyhow::Error::msg)?
            }
            IntegrationOperationsCommands::Credentials { integration_id, input } => {
                let input = read_json_input(&input, stdin)?;
                client
                    .integration_credentials_put(&integration_id, &input)
                    .await
                    .map_err(anyhow::Error::msg)?
            }
        },
        OperationsCommands::Memory { command } => match command {
            MemoryOperationsCommands::Query { query, category } => client
                .memory_query_get(query.as_deref(), category.as_deref())
                .await
                .map_err(anyhow::Error::msg)?,
            MemoryOperationsCommands::Store { input } => {
                let input = read_json_input(&input, stdin)?;
                client.memory_store(&input).await.map_err(anyhow::Error::msg)?
            }
            MemoryOperationsCommands::Delete { key } => {
                client.memory_delete(&key).await.map_err(anyhow::Error::msg)?
            }
        },
        OperationsCommands::Doctor { command: DoctorOperationsCommands::Get } => {
            client.doctor_get().await.map_err(anyhow::Error::msg)?
        }
        OperationsCommands::Doctor { command: DoctorOperationsCommands::Post } => {
            client.doctor_post().await.map_err(anyhow::Error::msg)?
        }
    };

    output::write_json(writer, &response, context.output_format())
}

fn parse_input<T: serde::de::DeserializeOwned>(
    source: &str,
    stdin: &mut impl Read,
    operation: &str,
) -> Result<T> {
    let input = read_json_input(source, stdin)?;
    serde_json::from_value(input).with_context(|| format!("invalid {operation} JSON input"))
}

fn parse_required_value(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        return Err("value must not be empty".to_string());
    }
    Ok(value.to_string())
}

fn parse_optional_non_empty(value: &str) -> Result<String, String> {
    parse_required_value(value)
}
