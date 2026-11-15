//! `shiju system` 的只读系统状态与发现命令。

use std::io::Write;

use anyhow::Result;
use clap::Subcommand;

use super::context::ApiContext;
use super::output;

/// Gateway 系统状态与发现操作。
#[derive(Subcommand, Debug)]
pub(super) enum SystemCommands {
    /// 获取基础健康探针。
    Health,
    /// 获取 Prometheus 文本指标。
    Metrics,
    /// 获取 Gateway 运行状态。
    Status,
    /// 获取 Gateway 工具目录。
    Tools,
    /// 检查 Gateway 可见的 MCP 服务。
    McpInspect,
    /// 获取受管 API 健康摘要。
    ApiHealth,
    /// 获取全局实例健康状态。
    GlobalHealth,
    /// 获取 OpenAI 兼容模型目录。
    Models,
    /// 获取当前目录上下文的路径信息。
    Path,
    /// 获取当前目录上下文的版本控制状态。
    Vcs,
    /// 获取当前目录上下文的命令目录。
    Commands,
    /// 获取当前目录上下文的代理目录。
    Agents,
    /// 获取当前目录上下文的技能目录。
    Skills,
    /// 获取 Gateway 可发现的 CLI 工具目录。
    CliTools,
}

pub(super) async fn run(
    command: SystemCommands,
    context: &ApiContext,
    writer: &mut impl Write,
) -> Result<()> {
    let client = context.client();
    let response = match command {
        SystemCommands::Health => client.system_health().await,
        SystemCommands::Metrics => {
            let metrics = client.system_metrics().await.map_err(anyhow::Error::msg)?;
            return output::write_text(writer, &metrics);
        }
        SystemCommands::Status => client.system_status().await,
        SystemCommands::Tools => client.system_tools().await,
        SystemCommands::McpInspect => client.system_mcp_inspect().await,
        SystemCommands::ApiHealth => client.system_api_health().await,
        SystemCommands::GlobalHealth => client.system_global_health().await,
        SystemCommands::Models => client.system_models().await,
        SystemCommands::Path => client.system_path(context.directory()).await,
        SystemCommands::Vcs => client.system_vcs(context.directory()).await,
        SystemCommands::Commands => client.system_commands(context.directory()).await,
        SystemCommands::Agents => client.system_agents(context.directory()).await,
        SystemCommands::Skills => client.system_skills(context.directory()).await,
        SystemCommands::CliTools => client.system_cli_tools().await,
    }
    .map_err(anyhow::Error::msg)?;

    output::write_json(writer, &response, context.output_format())
}
