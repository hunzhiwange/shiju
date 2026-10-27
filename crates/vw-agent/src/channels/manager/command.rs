//! 通道命令处理模块
//!
//! 本模块提供通道管理命令的处理逻辑，包括：
//! - 列出可用通道及其配置状态
//! - 绑定通道身份标识
//! - 其他通道管理操作的入口
//!
//! # 命令类型
//!
//! 支持的命令由 `ChannelCommands` 枚举定义，包括：
//! - `Start`：启动通道服务（需要在异步运行时中处理）
//! - `Doctor`：诊断通道健康状态（需要在异步运行时中处理）
//! - `List`：列出所有可用通道
//! - `Add`：添加新通道配置
//! - `Remove`：移除通道配置
//! - `BindTelegram`：绑定 Telegram 身份标识

use super::*;
use crate::app::agent::config::schema::ChannelsConfigExt;

/// 处理通道管理命令
///
/// 根据传入的命令类型执行相应的通道管理操作。
///
/// # 参数
///
/// - `command`：要处理的通道命令，类型为 `ChannelCommands` 枚举
/// - `config`：应用配置引用，用于读取通道配置信息
///
/// # 返回值
///
/// - `Ok(())`：命令执行成功
/// - `Err(...)`：命令执行失败，包含错误信息
///
/// # 错误
///
/// 本函数可能在以下情况返回错误：
/// - `Start` 命令：必须在 main.rs 中处理（需要异步运行时）
/// - `Doctor` 命令：必须在 main.rs 中处理（需要异步运行时）
/// - `Add` 命令：当前构建版本不支持指定的通道类型
/// - `Remove` 命令：需要直接编辑配置文件
///
/// # 示例
///
/// ```no_run
/// use vibe_agent::channels::ChannelCommands;
///
/// // 列出所有通道
/// let result = handle_command(ChannelCommands::List, &config).await;
/// ```
pub async fn handle_command(command: ChannelCommands, config: &Config) -> Result<()> {
    match command {
        // Start 命令必须在 main.rs 中处理，因为需要初始化异步运行时
        ChannelCommands::Start => {
            anyhow::bail!("Start must be handled in main.rs (requires async runtime)")
        }
        // Doctor 命令必须在 main.rs 中处理，因为需要完整的运行时环境进行诊断
        ChannelCommands::Doctor => {
            anyhow::bail!("Doctor must be handled in main.rs (requires async runtime)")
        }
        // List 命令：列出所有可用通道及其配置状态
        ChannelCommands::List => {
            println!("Channels:");
            // CLI 通道始终可用
            println!("  ✅ CLI (always available)");

            // 遍历配置中的所有通道，显示其配置状态
            for (channel, configured) in config.channels_config.channels() {
                println!("  {} {}", if configured { "✅" } else { "❌" }, channel.name());
            }

            // 检查 Lark/飞书通道是否在当前构建中启用
            if !cfg!(feature = "channel-lark") {
                println!(
                    "  ℹ️ Lark/Feishu channel support is disabled in this build (enable `channel-lark`)."
                );
            }

            // 显示使用提示
            println!("\nTo start channels: shiju channel start");
            println!("To check health:    shiju channel doctor");
            Ok(())
        }
        // Add 命令：添加新通道（当前构建版本不支持）
        ChannelCommands::Add { channel_type, config: _ } => {
            anyhow::bail!("Channel type '{channel_type}' is not supported in this build.")
        }
        // Remove 命令：移除通道（需要直接编辑配置文件）
        ChannelCommands::Remove { name } => {
            anyhow::bail!(
                "Remove channel '{name}' — edit {} directly",
                config.config_path.display()
            )
        }
        // BindTelegram 命令：绑定 Telegram 身份标识
        ChannelCommands::BindTelegram { identity } => {
            bind_telegram_identity(config, &identity).await
        }
    }
}

#[cfg(test)]
#[path = "command_tests.rs"]
mod command_tests;
