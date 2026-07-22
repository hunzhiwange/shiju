//! 本机运行环境诊断。
//!
//! 本模块检查 doctor 运行所依赖的基础命令、shell/home 环境变量以及已发现的
//! CLI 工具。检查结果以诊断项形式返回，供上层命令统一渲染。

use super::{COMMAND_VERSION_PREVIEW_CHARS, DiagItem, utils::truncate_for_display};

/// 检查基础系统环境并追加诊断项。
///
/// 参数：
/// - `items`：收集诊断结果的输出列表。
///
/// 返回值：
/// 本函数通过 `items` 输出结果，不返回独立值。
///
/// 错误处理：
/// 命令缺失或环境变量缺失会被转换为警告/错误诊断项；不会中断后续检查。
pub(super) fn check_environment(items: &mut Vec<DiagItem>) {
    let cat = "environment";

    check_command_available("git", &["--version"], cat, items);

    let shell = std::env::var("SHELL").unwrap_or_default();
    if shell.is_empty() {
        items.push(DiagItem::warn(cat, "$SHELL not set"));
    } else {
        items.push(DiagItem::ok(cat, format!("shell: {shell}")));
    }

    if std::env::var("HOME").is_ok() || std::env::var("USERPROFILE").is_ok() {
        items.push(DiagItem::ok(cat, "home directory env set"));
    } else {
        items.push(DiagItem::error(cat, "neither $HOME nor $USERPROFILE is set"));
    }

    check_command_available("curl", &["--version"], cat, items);
}

/// 发现 PATH 中可用的外部 CLI 工具。
///
/// 参数：
/// - `items`：收集诊断结果的输出列表。
///
/// 错误处理：
/// 当前发现逻辑返回可用列表；未发现工具会记录为警告，而不是作为硬错误处理。
pub(super) fn check_cli_tools(items: &mut Vec<DiagItem>) {
    let cat = "cli-tools";
    let discovered = crate::app::agent::tools::cli_discovery::discover_cli_tools(&[], &[]);

    append_cli_tool_items(&discovered, cat, items);
}

pub(super) fn append_cli_tool_items(
    discovered: &[crate::app::agent::tools::cli_discovery::DiscoveredCli],
    cat: &'static str,
    items: &mut Vec<DiagItem>,
) {
    if discovered.is_empty() {
        items.push(DiagItem::warn(cat, "No CLI tools found in PATH"));
        return;
    }

    for cli in discovered {
        let version_info = cli
            .version
            .as_deref()
            .map(|version| truncate_for_display(version, COMMAND_VERSION_PREVIEW_CHARS))
            .unwrap_or_else(|| "unknown version".to_string());
        items
            .push(DiagItem::ok(cat, format!("{} ({}) — {}", cli.name, cli.category, version_info)));
    }

    items.push(DiagItem::ok(cat, format!("{} CLI tools discovered", discovered.len())));
}

/// 执行带版本参数的命令可用性检查。
///
/// 参数：
/// - `command`：要执行的命令名。
/// - `args`：用于探测版本或可用性的参数。
/// - `category`：写入诊断项的分类。
/// - `items`：收集诊断结果的输出列表。
///
/// 错误处理：
/// 进程启动失败会记录为“PATH 中未找到”；非零退出码记录为警告；成功时截断首行
/// 输出，避免过长版本信息影响 doctor 可读性。
pub(super) fn check_command_available(
    command: &str,
    args: &[&str],
    category: &'static str,
    items: &mut Vec<DiagItem>,
) {
    match std::process::Command::new(command)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
    {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            let first_line = version.lines().next().unwrap_or("").trim();
            let display = truncate_for_display(first_line, COMMAND_VERSION_PREVIEW_CHARS);
            items.push(DiagItem::ok(category, format!("{command}: {display}")));
        }
        Ok(_) => {
            items.push(DiagItem::warn(category, format!("{command} found but returned non-zero")));
        }
        Err(_) => {
            items.push(DiagItem::warn(category, format!("{command} not found in PATH")));
        }
    }
}
