//! 命令风险评估模块
//!
//! 本模块提供 shell 命令的风险等级分类功能，用于安全策略执行时的风险评估。
//! 通过分析命令的基础名称、参数和组合模式，将命令划分为三个风险等级：
//!
//! - **高风险**：可能导致系统破坏、数据丢失或安全漏洞的命令（如 `rm`、`sudo`、`dd`）
//! - **中等风险**：可能影响文件系统或项目状态的命令（如 `git commit`、`npm install`）
//! - **低风险**：相对安全的只读或查询类命令
//!
//! # 设计原则
//!
//! - 采用保守评估策略：不确定时倾向于更高的风险等级
//! - 支持多段命令分析（如 `cmd1 && cmd2`）
//! - 忽略环境变量赋值，仅评估实际执行的命令
//!
//! # 示例
//!
//! ```ignore
//! use agent_security::policy::risk::classify_command_risk;
//! use agent_security::policy::types::CommandRiskLevel;
//!
//! let level = classify_command_risk("ls -la");
//! assert_eq!(level, CommandRiskLevel::Low);
//!
//! let level = classify_command_risk("rm -rf /");
//! assert_eq!(level, CommandRiskLevel::High);
//! ```

use super::shell_lexer::{skip_env_assignments, split_unquoted_segments};
use super::types::CommandRiskLevel;

/// 对 shell 命令进行风险等级分类
///
/// 该函数分析给定的命令字符串，评估其潜在风险并返回对应的风险等级。
/// 支持复合命令（使用 `&&`、`||`、`;` 等分隔符）的分析，只要任一段命令
/// 为高风险，整体即判定为高风险。
///
/// # 参数
///
/// - `command`: 待评估的 shell 命令字符串
///
/// # 返回值
///
/// 返回 `CommandRiskLevel` 枚举值：
/// - `CommandRiskLevel::High`: 高风险命令
/// - `CommandRiskLevel::Medium`: 中等风险命令
/// - `CommandRiskLevel::Low`: 低风险命令
///
/// # 评估逻辑
///
/// 1. 将命令拆分为多个段（处理引号和转义）
/// 2. 对每段命令跳过环境变量赋值部分
/// 3. 提取命令基础名称和参数
/// 4. 匹配高风险和中等风险命令模式
/// 5. 返回最高风险等级（高 > 中 > 低）
///
/// # 示例
///
/// ```ignore
/// // 低风险命令
/// assert_eq!(classify_command_risk("echo hello"), CommandRiskLevel::Low);
///
/// // 中等风险命令
/// assert_eq!(classify_command_risk("git commit -m 'msg'"), CommandRiskLevel::Medium);
///
/// // 高风险命令
/// assert_eq!(classify_command_risk("sudo rm -rf /"), CommandRiskLevel::High);
/// ```
pub fn classify_command_risk(command: &str) -> CommandRiskLevel {
    // 跟踪是否遇到中等风险的命令段
    let mut saw_medium = false;

    // 将命令拆分为多个段（处理复合命令）
    for segment in split_unquoted_segments(command) {
        // 跳过环境变量赋值，获取实际命令部分
        let cmd_part = skip_env_assignments(&segment);
        let mut words = cmd_part.split_whitespace();

        // 提取命令的第一个词（基础命令名）
        let Some(base_raw) = words.next() else {
            continue;
        };

        // 提取基础命令名（去除路径前缀），统一转为小写
        let base = base_raw.rsplit('/').next().unwrap_or("").to_ascii_lowercase();

        // 收集所有参数并转为小写
        let args: Vec<String> = words.map(|w| w.to_ascii_lowercase()).collect();
        // 保留完整的命令段用于模式匹配
        let joined_segment = cmd_part.to_ascii_lowercase();

        // 检查是否为高风险命令
        if is_high_risk_command(&base, &joined_segment) {
            return CommandRiskLevel::High;
        }

        // 检查是否为中等风险命令，并记录结果
        saw_medium |= is_medium_risk_command(&base, &args);
    }

    // 根据检测结果返回风险等级：中或低
    if saw_medium { CommandRiskLevel::Medium } else { CommandRiskLevel::Low }
}

/// 判断命令是否为高风险
///
/// 高风险命令包括：
/// - 系统管理命令：`sudo`、`su`、`shutdown`、`reboot`、`mount`、`umount`
/// - 用户管理命令：`useradd`、`userdel`、`usermod`、`passwd`
/// - 权限管理命令：`chown`、`chmod`
/// - 文件系统破坏命令：`rm`、`mkfs`、`dd`
/// - 网络工具命令：`curl`、`wget`、`nc`、`ssh`、`scp`、`ftp`、`telnet`
/// - 防火墙命令：`iptables`、`ufw`、`firewall-cmd`
/// - 危险模式：`rm -rf /`、fork 炸弹等
///
/// # 参数
///
/// - `base`: 命令的基础名称（已转为小写）
/// - `joined_segment`: 完整的命令段字符串（已转为小写）
///
/// # 返回值
///
/// 如果命令为高风险则返回 `true`，否则返回 `false`
fn is_high_risk_command(base: &str, joined_segment: &str) -> bool {
    // 检查基础命令是否在已知高风险命令列表中
    if matches!(
        base,
        "rm" | "mkfs"
            | "dd"
            | "shutdown"
            | "reboot"
            | "halt"
            | "poweroff"
            | "sudo"
            | "su"
            | "chown"
            | "chmod"
            | "useradd"
            | "userdel"
            | "usermod"
            | "passwd"
            | "mount"
            | "umount"
            | "iptables"
            | "ufw"
            | "firewall-cmd"
            | "curl"
            | "wget"
            | "nc"
            | "ncat"
            | "netcat"
            | "scp"
            | "ssh"
            | "ftp"
            | "telnet"
    ) {
        return true;
    }

    // 检查特定的危险命令模式
    // - `rm -rf /` 或 `rm -fr /`: 删除根目录
    // - `:(){:|:&};:`: fork 炸弹
    if joined_segment.contains("rm -rf /")
        || joined_segment.contains("rm -fr /")
        || joined_segment.contains(":(){:|:&};:")
    {
        return true;
    }

    false
}

/// 判断命令是否为中等风险
///
/// 中等风险命令包括：
/// - **Git 操作**: `commit`、`push`、`reset`、`clean`、`rebase`、`merge`、`cherry-pick`、
///   `revert`、`branch`、`checkout`、`switch`、`tag`
/// - **包管理器操作**: `npm`/`pnpm`/`yarn` 的 `install`、`add`、`remove`、`uninstall`、
///   `update`、`publish`
/// - **Cargo 操作**: `add`、`remove`、`install`、`clean`、`publish`
/// - **文件操作**: `touch`、`mkdir`、`mv`、`cp`、`ln`
///
/// # 参数
///
/// - `base`: 命令的基础名称（已转为小写）
/// - `args`: 命令的参数列表（已转为小写）
///
/// # 返回值
///
/// 如果命令为中等风险则返回 `true`，否则返回 `false`
fn is_medium_risk_command(base: &str, args: &[String]) -> bool {
    match base {
        // Git 命令：检查子命令是否为可能改变仓库状态的操作
        "git" => args.first().is_some_and(|verb| {
            matches!(
                verb.as_str(),
                "commit"
                    | "push"
                    | "reset"
                    | "clean"
                    | "rebase"
                    | "merge"
                    | "cherry-pick"
                    | "revert"
                    | "branch"
                    | "checkout"
                    | "switch"
                    | "tag"
            )
        }),
        // JavaScript 包管理器：检查子命令是否为可能改变依赖或发布的操作
        "npm" | "pnpm" | "yarn" => args.first().is_some_and(|verb| {
            matches!(
                verb.as_str(),
                "install" | "add" | "remove" | "uninstall" | "update" | "publish"
            )
        }),
        // Rust 包管理器：检查子命令是否为可能改变依赖或发布的操作
        "cargo" => args.first().is_some_and(|verb| {
            matches!(verb.as_str(), "add" | "remove" | "install" | "clean" | "publish")
        }),
        // 文件系统操作命令：可能改变文件系统状态
        "touch" | "mkdir" | "mv" | "cp" | "ln" => true,
        // 其他命令默认为低风险
        _ => false,
    }
}

#[cfg(test)]
#[path = "risk_tests.rs"]
mod risk_tests;
