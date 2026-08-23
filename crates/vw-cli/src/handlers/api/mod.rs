//! 远程 Gateway API CLI 的共享边界。

use std::io::Write;
use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Args, Subcommand, ValueEnum};

use self::context::ApiContext;

mod context;
mod input;
mod output;
mod system;

use self::system::SystemCommands;

/// `shiju api` 的连接、认证、输出和命令参数。
#[derive(Args, Debug)]
pub(crate) struct ApiArgs {
    /// Gateway 主机名或 IP。
    #[arg(long, global = true, default_value = "127.0.0.1")]
    host: String,

    /// Gateway 端口。
    #[arg(
        long,
        global = true,
        default_value_t = vw_config_types::gateway::DEFAULT_GATEWAY_PORT
    )]
    port: u16,

    /// 请求的平台租户 ID。
    #[arg(
        long,
        global = true,
        default_value = "local",
        value_parser = parse_required_scope_id
    )]
    platform: String,

    /// 请求的公司租户 ID。
    #[arg(
        long,
        global = true,
        default_value = "local",
        value_parser = parse_required_scope_id
    )]
    company: String,

    /// 从指定环境变量读取 Gateway skey。
    #[arg(long, global = true, conflicts_with = "skey_file")]
    skey_env: Option<String>,

    /// 从指定文件读取 Gateway skey。
    #[arg(long, global = true, conflicts_with = "skey_env")]
    skey_file: Option<PathBuf>,

    /// Gateway 请求使用的项目目录上下文。
    #[arg(long, global = true)]
    directory: Option<String>,

    /// 结构化输出格式。
    #[arg(long, global = true, value_enum, default_value_t = ApiOutputFormat::Json)]
    output: ApiOutputFormat,

    /// 确认危险或有宿主副作用的操作。
    #[arg(long, global = true)]
    yes: bool,

    /// API 领域子命令。
    #[command(subcommand)]
    command: ApiCommands,
}

/// API 领域命令注册表。
#[derive(Subcommand, Debug)]
enum ApiCommands {
    /// 查询 Gateway 可见的 provider。
    Provider {
        /// Provider 操作。
        #[command(subcommand)]
        command: ProviderCommands,
    },
    /// 查询 Gateway 系统状态与发现信息。
    System {
        /// 系统状态与发现操作。
        #[command(subcommand)]
        command: SystemCommands,
    },
}

impl ApiCommands {
    fn dangerous_operation(&self) -> Option<&'static str> {
        match self {
            Self::Provider { command: ProviderCommands::List } | Self::System { .. } => None,
        }
    }
}

/// Provider 示例命令。
#[derive(Subcommand, Debug)]
enum ProviderCommands {
    /// 列出 Gateway 可见的 provider。
    List,
}

/// API 结构化输出格式。
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ApiOutputFormat {
    /// 单个格式化 JSON 文档。
    Json,
    /// 每行一个紧凑 JSON 值。
    Jsonl,
}

/// 执行远程 Gateway API 命令。
pub(crate) async fn run(args: ApiArgs) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    run_with_writer(args, &mut stdout).await
}

async fn run_with_writer<W: Write>(args: ApiArgs, writer: &mut W) -> Result<()> {
    if let Some(operation) = args.command.dangerous_operation() {
        ensure_confirmed(args.yes, operation)?;
    }

    let context = ApiContext::from_args(&args)?;
    match args.command {
        ApiCommands::Provider { command: ProviderCommands::List } => {
            let response = context
                .client()
                .provider_list(context.directory())
                .await
                .map_err(anyhow::Error::msg)?;
            output::write_json(writer, &response, context.output_format())
        }
        ApiCommands::System { command } => system::run(command, &context, writer).await,
    }
}

fn parse_required_scope_id(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        return Err("tenant scope id must not be empty".to_string());
    }
    Ok(value.to_string())
}

fn ensure_confirmed(yes: bool, operation: &str) -> Result<()> {
    if !yes {
        bail!("refusing dangerous operation '{operation}' without --yes");
    }
    Ok(())
}

#[cfg(test)]
mod system_tests;
#[cfg(test)]
mod tests;
