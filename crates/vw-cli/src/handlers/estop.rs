//! # 紧急停止命令处理器模块
//!
//! 本模块提供紧急停止（Emergency Stop，简称 Estop）功能的命令行处理逻辑。
//! 紧急停止是 VibeWindow 安全系统的核心组件，允许用户在检测到异常或危险行为时
//! 快速中断代理的执行。
//!
//! ## 主要功能
//!
//! - **启动紧急停止**：支持多种级别的紧急停止（完全停止、网络中断、域名屏蔽、工具冻结）
//! - **恢复执行**：在问题解决后，可选择性地恢复特定功能的执行
//! - **状态查询**：查看当前紧急停止的状态和受影响的功能范围
//!
//! ## 安全机制
//!
//! - 支持可选的 OTP（一次性密码）验证，确保只有授权用户才能恢复执行
//! - 分级停止策略，允许精细控制受影响的功能范围
//!
//! ## 使用示例
//!
//! ```bash
//! # 启动完全紧急停止
//! vibe-agent estop
//!
//! # 查看当前状态
//! vibe-agent estop status
//!
//! # 仅中断网络连接
//! vibe-agent estop --level network-kill
//!
//! # 屏蔽特定域名
//! vibe-agent estop --level domain-block --domain example.com
//!
//! # 恢复执行（如果配置了 OTP，需要输入验证码）
//! vibe-agent estop resume
//! ```

use anyhow::{Context, Result, bail};
use dialoguer::Password;

use crate::cli::{EstopLevelArg, EstopSubcommands};
use crate::config::Config;
use vw_agent::security;

/// 处理紧急停止命令的主入口函数
///
/// 根据用户提供的子命令和参数，执行相应的紧急停止操作。
/// 如果没有指定子命令，默认启动紧急停止。
///
/// # 参数
///
/// * `config` - 应用配置，包含安全策略和 OTP 设置
/// * `estop_command` - 可选的子命令（Status 或 Resume）
/// * `level` - 紧急停止级别（仅用于启动停止时）
/// * `domains` - 要屏蔽的域名列表（与 `DomainBlock` 级别配合使用）
/// * `tools` - 要冻结的工具列表（与 `ToolFreeze` 级别配合使用）
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误信息
///
/// # 错误
///
/// - 紧急停止功能被禁用时返回错误
/// - 参数组合不合法时返回错误
/// - OTP 验证失败时返回错误
///
/// # 示例
///
/// ```ignore
/// // 启动完全紧急停止
/// handle_estop_command(&config, None, None, vec![], vec![])?;
///
/// // 查看状态
/// handle_estop_command(&config, Some(EstopSubcommands::Status), None, vec![], vec![])?;
///
/// // 恢复执行
/// handle_estop_command(&config, Some(EstopSubcommands::Resume{...}), None, vec![], vec![])?;
/// ```
pub(crate) fn handle_estop_command(
    config: &Config,
    estop_command: Option<EstopSubcommands>,
    level: Option<EstopLevelArg>,
    domains: Vec<String>,
    tools: Vec<String>,
) -> Result<()> {
    // 检查紧急停止功能是否启用
    if !config.security.estop.enabled {
        bail!("Emergency stop is disabled. Enable security.estop.enabled in system settings");
    }

    // 加载紧急停止管理器，从配置目录读取状态
    let config_dir =
        config.config_path.parent().context("Config path must have a parent directory")?;
    let mut manager = security::EstopManager::load(&config.security.estop, config_dir)?;

    // 根据子命令执行相应操作
    match estop_command {
        // 查看当前紧急停止状态
        Some(EstopSubcommands::Status) => {
            print_estop_status(&manager.status());
            Ok(())
        }
        // 恢复执行
        Some(EstopSubcommands::Resume { network, domains, tools, otp }) => {
            // 构建恢复选择器，决定恢复哪些功能
            let selector = build_resume_selector(network, domains, tools)?;
            let mut otp_code = otp;

            // 如果配置要求恢复时需要 OTP 验证，则进行验证
            let otp_validator = if config.security.estop.require_otp_to_resume {
                // 确保 OTP 功能已启用
                if !config.security.otp.enabled {
                    bail!(
                        "security.estop.require_otp_to_resume=true but security.otp.enabled=false"
                    );
                }

                // 如果命令行未提供 OTP，则交互式提示用户输入
                if otp_code.is_none() {
                    let entered = Password::new()
                        .with_prompt("Enter OTP code")
                        .allow_empty_password(false)
                        .interact()?;
                    otp_code = Some(entered);
                }

                // 初始化 OTP 验证器
                let store = security::SecretStore::new(config_dir, config.secrets.encrypt);
                let (validator, enrollment_uri) =
                    security::OtpValidator::from_config(&config.security.otp, config_dir, &store)?;

                // 如果是首次使用，输出注册 URI
                if let Some(uri) = enrollment_uri {
                    println!("Initialized OTP secret for ShiJu.");
                    println!("Enrollment URI: {uri}");
                }
                Some(validator)
            } else {
                None
            };

            // 执行恢复操作
            manager.resume(selector, otp_code.as_deref(), otp_validator.as_ref())?;
            println!("Estop resume completed.");
            print_estop_status(&manager.status());
            Ok(())
        }
        // 默认：启动紧急停止
        None => {
            // 构建紧急停止级别
            let engage_level = build_engage_level(level, domains, tools)?;
            manager.engage(engage_level)?;
            println!("Estop engaged.");
            print_estop_status(&manager.status());
            Ok(())
        }
    }
}

/// 根据命令行参数构建紧急停止级别
///
/// 将用户提供的级别参数和域名/工具列表转换为内部的 `EstopLevel` 枚举。
/// 同时验证参数组合的合法性，防止不兼容的参数同时使用。
///
/// # 参数
///
/// * `level` - 可选的紧急停止级别参数，默认为 `KillAll`
/// * `domains` - 要屏蔽的域名列表
/// * `tools` - 要冻结的工具列表
///
/// # 返回值
///
/// 返回构建好的 `EstopLevel` 实例
///
/// # 错误
///
/// - `DomainBlock` 级别未提供域名时返回错误
/// - `ToolFreeze` 级别未提供工具时返回错误
/// - 参数组合不合法时返回错误（例如 `DomainBlock` 级别使用了 `--tool` 参数）
///
/// # 参数组合规则
///
/// | 级别 | 允许的额外参数 |
/// |------|----------------|
/// | `KillAll` | 无 |
/// | `NetworkKill` | 无 |
/// | `DomainBlock` | `--domain`（至少一个） |
/// | `ToolFreeze` | `--tool`（至少一个） |
fn build_engage_level(
    level: Option<EstopLevelArg>,
    domains: Vec<String>,
    tools: Vec<String>,
) -> Result<security::EstopLevel> {
    // 默认使用最高级别的停止
    let requested = level.unwrap_or(EstopLevelArg::KillAll);

    match requested {
        // 完全停止：终止所有操作
        EstopLevelArg::KillAll => {
            if !domains.is_empty() || !tools.is_empty() {
                bail!("--domain/--tool are only valid with --level domain-block/tool-freeze");
            }
            Ok(security::EstopLevel::KillAll)
        }
        // 网络中断：仅中断网络连接
        EstopLevelArg::NetworkKill => {
            if !domains.is_empty() || !tools.is_empty() {
                bail!("--domain/--tool are not valid with --level network-kill");
            }
            Ok(security::EstopLevel::NetworkKill)
        }
        // 域名屏蔽：阻止访问指定域名
        EstopLevelArg::DomainBlock => {
            if domains.is_empty() {
                bail!("--level domain-block requires at least one --domain");
            }
            if !tools.is_empty() {
                bail!("--tool is not valid with --level domain-block");
            }
            Ok(security::EstopLevel::DomainBlock(domains))
        }
        // 工具冻结：禁止使用指定工具
        EstopLevelArg::ToolFreeze => {
            if tools.is_empty() {
                bail!("--level tool-freeze requires at least one --tool");
            }
            if !domains.is_empty() {
                bail!("--domain is not valid with --level tool-freeze");
            }
            Ok(security::EstopLevel::ToolFreeze(tools))
        }
    }
}

/// 构建恢复选择器，决定要恢复哪些功能
///
/// 在紧急停止后恢复执行时，用户可以选择性地恢复特定功能，
/// 而不是一次性恢复所有功能。此函数根据用户提供的参数构建相应的选择器。
///
/// # 参数
///
/// * `network` - 是否恢复网络连接
/// * `domains` - 要恢复访问的域名列表
/// * `tools` - 要恢复使用的工具列表
///
/// # 返回值
///
/// 返回 `ResumeSelector` 枚举实例，表示要恢复的功能范围
///
/// # 错误
///
/// 如果同时指定了多个恢复选项（`--network`、`--domain`、`--tool`），返回错误
///
/// # 恢复优先级
///
/// 1. 如果指定了 `--network`，则恢复网络连接
/// 2. 如果指定了 `--domain`，则恢复指定域名的访问
/// 3. 如果指定了 `--tool`，则恢复指定工具的使用
/// 4. 如果都未指定，则恢复所有功能（相当于完全解除紧急停止）
///
/// # 示例
///
/// ```ignore
/// // 恢复网络连接
/// let selector = build_resume_selector(true, vec![], vec![])?;
///
/// // 恢复特定域名访问
/// let selector = build_resume_selector(false, vec!["example.com".to_string()], vec![])?;
///
/// // 恢复所有功能
/// let selector = build_resume_selector(false, vec![], vec![])?;
/// ```
fn build_resume_selector(
    network: bool,
    domains: Vec<String>,
    tools: Vec<String>,
) -> Result<security::ResumeSelector> {
    // 计算用户选择的恢复选项数量，确保只选择了一个
    let selected =
        usize::from(network) + usize::from(!domains.is_empty()) + usize::from(!tools.is_empty());

    // 不允许同时指定多个恢复选项
    if selected > 1 {
        bail!("Use only one of --network, --domain, or --tool for estop resume");
    }

    // 根据用户选择返回相应的选择器
    if network {
        return Ok(security::ResumeSelector::Network);
    }
    if !domains.is_empty() {
        return Ok(security::ResumeSelector::Domains(domains));
    }
    if !tools.is_empty() {
        return Ok(security::ResumeSelector::Tools(tools));
    }

    // 默认恢复所有功能
    Ok(security::ResumeSelector::KillAll)
}

/// 打印紧急停止状态到标准输出
///
/// 以用户友好的格式显示当前紧急停止的状态信息，包括：
/// - 是否处于紧急停止状态
/// - 各级别停止的激活情况
/// - 被屏蔽的域名列表
/// - 被冻结的工具列表
/// - 最后更新时间
///
/// # 参数
///
/// * `state` - 紧急停止状态引用，包含所有状态信息
///
/// # 输出格式示例
///
/// ```text
/// Estop status:
///   engaged:        yes
///   kill_all:       inactive
///   network_kill:   active
///   domain_blocks:  example.com, api.example.com
///   tool_freeze:    shell, file_ops
///   updated_at:     2026-03-20T10:30:00Z
/// ```
fn print_estop_status(state: &security::EstopState) {
    println!("Estop status:");
    println!("  engaged:        {}", if state.is_engaged() { "yes" } else { "no" });
    println!("  kill_all:       {}", if state.kill_all { "active" } else { "inactive" });
    println!("  network_kill:   {}", if state.network_kill { "active" } else { "inactive" });

    // 显示被屏蔽的域名列表
    if state.blocked_domains.is_empty() {
        println!("  domain_blocks:  (none)");
    } else {
        println!("  domain_blocks:  {}", state.blocked_domains.join(", "));
    }

    // 显示被冻结的工具列表
    if state.frozen_tools.is_empty() {
        println!("  tool_freeze:    (none)");
    } else {
        println!("  tool_freeze:    {}", state.frozen_tools.join(", "));
    }

    // 显示最后更新时间（如果有）
    if let Some(updated_at) = &state.updated_at {
        println!("  updated_at:     {updated_at}");
    }
}
#[cfg(test)]
#[path = "estop_tests.rs"]
mod estop_tests;
