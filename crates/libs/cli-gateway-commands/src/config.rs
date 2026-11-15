//! Gateway 配置读取、目录查询与受控写入命令。

use std::io::{Read, Write};

use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

/// 配置接口命令。
#[derive(Subcommand, Debug)]
pub(super) enum ConfigCommands {
    /// 读取当前目录作用域的生效配置。
    Get,
    /// 用 JSON 补丁更新当前目录作用域的配置。
    Patch {
        /// JSON 文件路径；使用 `-` 从 stdin 读取。
        #[arg(long)]
        input: String,
    },
    /// 读取或更新当前租户的全局配置。
    Global {
        /// 全局配置操作。
        #[command(subcommand)]
        command: GlobalConfigCommands,
    },
    /// 读取当前租户的 ACP 配置。
    Acp,
    /// 读取或替换 dashboard 完整配置。
    Dashboard {
        /// Dashboard 配置操作。
        #[command(subcommand)]
        command: DashboardConfigCommands,
    },
    /// 读取当前目录作用域的 provider 配置目录。
    Providers,
}

/// 全局配置操作。
#[derive(Subcommand, Debug)]
pub(super) enum GlobalConfigCommands {
    /// 读取当前租户的全局配置。
    Get,
    /// 用 JSON 补丁更新当前租户的全局配置。
    Patch {
        /// JSON 文件路径；使用 `-` 从 stdin 读取。
        #[arg(long)]
        input: String,
    },
}

/// Dashboard 配置操作。
#[derive(Subcommand, Debug)]
pub(super) enum DashboardConfigCommands {
    /// 读取 dashboard 完整配置。
    Get,
    /// 用用户提供的 JSON 替换 dashboard 完整配置。
    Put {
        /// JSON 文件路径；使用 `-` 从 stdin 读取。
        #[arg(long)]
        input: String,
    },
}

impl ConfigCommands {
    pub(super) fn dangerous_operation(
        &self,
        directory: Option<&str>,
        platform: &str,
        company: &str,
    ) -> Option<String> {
        let tenant = format!("{platform}/{company}");
        match self {
            Self::Patch { .. } => Some(format!(
                "patch config for directory '{}' in tenant {tenant}",
                directory.unwrap_or("<gateway-default>")
            )),
            Self::Global { command: GlobalConfigCommands::Patch { .. } } => {
                Some(format!("patch global config for tenant {tenant}"))
            }
            Self::Dashboard { command: DashboardConfigCommands::Put { .. } } => {
                Some(format!("replace dashboard config for tenant {tenant}"))
            }
            Self::Get
            | Self::Global { command: GlobalConfigCommands::Get }
            | Self::Acp
            | Self::Dashboard { command: DashboardConfigCommands::Get }
            | Self::Providers => None,
        }
    }
}

pub(super) async fn run<R: Read, W: Write>(
    command: ConfigCommands,
    context: &ApiContext,
    stdin: &mut R,
    writer: &mut W,
) -> Result<()> {
    let client = context.client();
    let response = match command {
        ConfigCommands::Get => client.config_get(context.directory()).await,
        ConfigCommands::Patch { input } => {
            let patch = read_json_input(&input, stdin)?;
            client.config_patch(context.directory(), &patch).await
        }
        ConfigCommands::Global { command: GlobalConfigCommands::Get } => {
            client.global_config_get().await
        }
        ConfigCommands::Global { command: GlobalConfigCommands::Patch { input } } => {
            let patch = read_json_input(&input, stdin)?;
            client.global_config_patch(&patch).await.map(|()| Value::Null)
        }
        ConfigCommands::Acp => client.global_acp_config_get::<Value>().await,
        ConfigCommands::Dashboard { command: DashboardConfigCommands::Get } => {
            client.dashboard_config_get().await
        }
        ConfigCommands::Dashboard { command: DashboardConfigCommands::Put { input } } => {
            let config = read_json_input(&input, stdin)?;
            client.dashboard_config_put(&config).await
        }
        ConfigCommands::Providers => client.config_providers(context.directory()).await,
    }
    .map_err(anyhow::Error::msg)?;

    output::write_json(writer, &response, context.output_format())
}
