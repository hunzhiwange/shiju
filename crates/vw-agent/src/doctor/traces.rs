//! 运行时 trace 查询命令。
//!
//! 本模块为 `doctor traces` 提供只读查询能力：可以按 trace id 输出完整事件，
//! 也可以按事件类型、文本包含关系和数量限制列出最近事件摘要。

use super::utils::truncate_for_display;
use crate::app::agent::config::Config;
use anyhow::Result;

/// 查询并打印运行时 trace。
///
/// 参数：
/// - `config`：用于解析 trace 文件路径的运行配置。
/// - `id`：可选 trace id；提供时优先查询单个完整事件。
/// - `event_filter`：可选事件类型过滤。
/// - `contains`：可选消息内容过滤。
/// - `limit`：最多返回的事件数量，内部会提升到至少 1。
///
/// 返回值：
/// 查询与打印成功时返回 `Ok(())`。
///
/// 错误处理：
/// trace 文件解析、按 id 查询或 JSON 格式化失败会向调用者返回错误；trace 文件
/// 不存在或无匹配事件则打印说明并正常返回，便于 doctor 作为排障辅助命令使用。
pub fn run_traces(
    config: &Config,
    id: Option<&str>,
    event_filter: Option<&str>,
    contains: Option<&str>,
    limit: usize,
) -> Result<()> {
    let path = crate::app::agent::observability::runtime_trace::resolve_trace_path(
        &config.observability,
        &config.workspace_dir,
    );

    if let Some(target_id) = id.map(str::trim).filter(|value| !value.is_empty()) {
        match crate::app::agent::observability::runtime_trace::find_event_by_id(&path, target_id)? {
            Some(event) => {
                println!("{}", serde_json::to_string_pretty(&event)?);
            }
            None => {
                println!(
                    "No runtime trace event found for id '{}' (path: {}).",
                    target_id,
                    path.display()
                );
            }
        }
        return Ok(());
    }

    if !path.exists() {
        println!(
            "Runtime trace file not found: {}.\n\
             Enable [observability] runtime_trace_mode = \"rolling\" or \"full\", then reproduce the issue.",
            path.display()
        );
        return Ok(());
    }

    // limit 为 0 时通常来自命令行默认值或用户误填；提升为 1 可以保持输出语义明确，
    // 同时避免“成功但永远无结果”的困惑。
    let safe_limit = limit.max(1);
    let events = crate::app::agent::observability::runtime_trace::load_events(
        &path,
        safe_limit,
        event_filter,
        contains,
    )?;

    if events.is_empty() {
        println!("No runtime trace events matched query (path: {}).", path.display());
        return Ok(());
    }

    println!("Runtime traces (newest first)");
    println!("Path: {}", path.display());
    println!(
        "Filters: event={} contains={} limit={}",
        event_filter.unwrap_or("*"),
        contains.unwrap_or("*"),
        safe_limit
    );
    println!();

    for event in events {
        let success = match event.success {
            Some(true) => "ok",
            Some(false) => "fail",
            None => "-",
        };
        let message = event.message.unwrap_or_default();
        let preview = truncate_for_display(&message, 80);
        println!(
            "- {} | {} | {} | {} | {}",
            event.timestamp, event.id, event.event_type, success, preview
        );
    }

    println!();
    println!("Use `shiju doctor traces --id <trace-id>` to inspect a full event payload.");
    Ok(())
}
