use super::Tool;
use serde_json::Value;

/// 等待调度的工具调用。
#[derive(Debug, Clone, PartialEq)]
pub struct PendingToolCall {
    /// 工具名称。
    pub name: String,
    /// 工具参数。
    pub arguments: Value,
    /// provider 原生工具调用 ID。
    pub tool_call_id: Option<String>,
}

/// 批次执行模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduledToolBatchMode {
    /// 同批调用可并发执行。
    Parallel,
    /// 同批调用必须顺序执行。
    Sequential,
}

/// 调度后的执行批次。
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledToolBatch {
    /// 当前批次的执行模式。
    pub mode: ScheduledToolBatchMode,
    /// 当前批次内的调用。
    pub calls: Vec<PendingToolCall>,
}

/// 根据工具元数据切分执行批次。
///
/// 当前阶段允许“只读且并发安全”的调用，以及写入隔离子会话的 AgentTool
/// 进入并发批次；其他写操作、非并发安全工具或未知工具都回退为顺序批次。
pub fn schedule_tool_calls(
    tool_calls: &[PendingToolCall],
    tools_registry: &[Box<dyn Tool>],
) -> Vec<ScheduledToolBatch> {
    let mut batches = Vec::new();
    let mut parallel_calls = Vec::new();

    for call in tool_calls {
        if can_run_in_parallel(call.name.as_str(), tools_registry) {
            parallel_calls.push(call.clone());
            continue;
        }

        flush_parallel_batch(&mut batches, &mut parallel_calls);
        batches.push(ScheduledToolBatch {
            mode: ScheduledToolBatchMode::Sequential,
            calls: vec![call.clone()],
        });
    }

    flush_parallel_batch(&mut batches, &mut parallel_calls);
    batches
}

fn flush_parallel_batch(
    batches: &mut Vec<ScheduledToolBatch>,
    parallel_calls: &mut Vec<PendingToolCall>,
) {
    if parallel_calls.is_empty() {
        return;
    }

    batches.push(ScheduledToolBatch {
        mode: ScheduledToolBatchMode::Parallel,
        calls: std::mem::take(parallel_calls),
    });
}

fn can_run_in_parallel(tool_name: &str, tools_registry: &[Box<dyn Tool>]) -> bool {
    let Some(tool) = find_tool_by_name(tools_registry, tool_name) else {
        return false;
    };

    let spec = tool.spec();
    spec.concurrency_safe && (spec.read_only || is_isolated_agent_tool(tool_name))
}

fn is_isolated_agent_tool(tool_name: &str) -> bool {
    matches!(tool_name.trim().to_ascii_lowercase().as_str(), "agenttool" | "agent_tool" | "agent")
}

fn find_tool_by_name<'a>(
    tools_registry: &'a [Box<dyn Tool>],
    requested_name: &str,
) -> Option<&'a dyn Tool> {
    tools_registry
        .iter()
        .find(|tool| {
            let spec = tool.spec();
            spec.id == requested_name || spec.aliases.iter().any(|alias| alias == requested_name)
        })
        .map(|tool| tool.as_ref())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
