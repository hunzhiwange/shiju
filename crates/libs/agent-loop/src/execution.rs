//! # 工具执行模块
//!
//! 本模块负责把 loop 层的执行编排收口为一个轻量壳层：
//! - 单工具执行统一委托给 tools/executor.rs
//! - 批次切分统一委托给 tools/scheduler.rs
//! - loop 层只保留观测、取消与结果清洗等编排职责
//!
//! ## 主要功能
//!
//! - **单工具执行**: 调用统一 executor，处理成功/失败状态
//! - **并行执行**: 执行 scheduler 切出的并发批次
//! - **顺序执行**: 执行 scheduler 切出的串行批次
//! - **取消支持**: 支持通过取消令牌中断正在执行的工具
//! - **观测性集成**: 记录工具调用的开始、完成和耗时等事件

use crate::dispatcher::ParsedToolCall;
use crate::core::errors::ToolLoopCancelled;
use agent_util::credentials::scrub_credentials;
use agent_observability::{Observer, ObserverEvent};
use tool_types::{Tool, ToolCallResult, ToolCallTelemetry, ToolResult};
use tool_services::scheduler::{PendingToolCall, ScheduledToolBatch};
use tool_context::{context::ToolUseContext, host::ContextHost};
use anyhow::Result;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use vw_api_tools::{PermissionRequestDto, ToolResultDto};



/// 执行单个工具调用
///
/// 该函数负责单个工具的完整执行流程，包括：
/// 1. 记录工具调用开始事件
/// 2. 在注册表中查找工具
/// 3. 执行工具（支持取消）
/// 4. 处理执行结果（成功/失败）
/// 5. 清理输出中的敏感信息
/// 6. 记录工具调用完成事件
///
/// # 参数
///
/// - `call_name`: 工具名称
/// - `call_arguments`: 工具调用参数（JSON 格式）
/// - `tools_registry`: 工具注册表
/// - `observer`: 观测器，用于记录事件
/// - `cancellation_token`: 可选的取消令牌，用于中断执行
///
/// # 返回值
///
/// - `Ok(ToolExecutionOutcome)`: 执行完成（包括工具执行失败的情况）
/// - `Err(...)`: 执行被取消或其他系统错误
///
/// # 取消行为
///
/// 当提供取消令牌且令牌被触发时，函数会立即返回 `ToolLoopCancelled` 错误
async fn execute_one_tool<H: ContextHost>(
    call: &PendingToolCall,
    tools_registry: &[Box<dyn Tool>],
    observer: &dyn Observer,
    tool_use_context: Arc<ToolUseContext<H>>,
    cancellation_token: Option<&CancellationToken>,
) -> Result<ToolExecutionOutcome> {
    // 记录工具调用开始事件
    observer.record_event(&ObserverEvent::ToolCallStart { tool: call.name.clone() });
    let start = Instant::now();

    let execution = if let Some(token) = cancellation_token {
        tokio::select! {
            () = token.cancelled() => return Err(ToolLoopCancelled.into()),
            result = tool_context::executor::execute_tool_from_registry(
                tools_registry,
                call.name.as_str(),
                call.arguments.clone(),
                tool_use_context,
            ) => result,
        }
    } else {
        tool_context::executor::execute_tool_from_registry(
            tools_registry,
            call.name.as_str(),
            call.arguments.clone(),
            tool_use_context,
        )
        .await
    };

    match execution {
        Ok(executed) => {
            let duration = executed.duration;
            let success = executed.result.is_success();
            observer.record_event(&ObserverEvent::ToolCall {
                tool: executed.tool_name.clone(),
                duration,
                success,
            });

            if success {
                let output = scrub_credentials(&executed.result.model_text());
                let result_dto = tool_result_dto_from_call_result(
                    &executed.tool_name,
                    call.tool_call_id.as_deref(),
                    &executed.result,
                    duration,
                );
                return Ok(ToolExecutionOutcome {
                    tool_name: executed.tool_name,
                    output: output.clone(),
                    success: true,
                    error_reason: None,
                    result_dto: Some(result_dto),
                    duration,
                });
            }

            let reason =
                executed.result.error_text().unwrap_or_else(|| executed.result.model_text());
            let scrubbed_reason = scrub_credentials(&reason);
            let result_dto = tool_result_dto_from_call_result(
                &executed.tool_name,
                call.tool_call_id.as_deref(),
                &executed.result,
                duration,
            );
            Ok(ToolExecutionOutcome {
                tool_name: executed.tool_name,
                output: format!("Error: {scrubbed_reason}"),
                success: false,
                error_reason: Some(scrubbed_reason.clone()),
                result_dto: Some(result_dto),
                duration,
            })
        }
        Err(denied) => {
            let tool_context::ToolCallError::Denied { .. } = &denied else {
                let tool_context::ToolCallError::Failed(reason) = denied else {
                    unreachable!();
                };
                let duration = start.elapsed();
                let scrubbed_reason = scrub_credentials(&reason);
                let result_dto = legacy_result_dto(
                    &call.name,
                    call.tool_call_id.as_deref(),
                    false,
                    Some(scrubbed_reason.clone()),
                    None,
                    duration,
                );
                observer.record_event(&ObserverEvent::ToolCall {
                    tool: call.name.clone(),
                    duration,
                    success: false,
                });
                return Ok(ToolExecutionOutcome {
                    tool_name: call.name.clone(),
                    output: scrubbed_reason.clone(),
                    success: false,
                    error_reason: Some(scrubbed_reason.clone()),
                    result_dto: Some(result_dto),
                    duration,
                });
            };
            let duration = start.elapsed();
            let scrubbed_reason = scrub_credentials(denied.message());
            let result_dto = legacy_result_dto(
                &call.name,
                call.tool_call_id.as_deref(),
                false,
                Some(scrubbed_reason.clone()),
                denied.permission_request().cloned(),
                duration,
            );
            observer.record_event(&ObserverEvent::ToolCall {
                tool: call.name.clone(),
                duration,
                success: false,
            });
            Ok(ToolExecutionOutcome {
                tool_name: call.name.clone(),
                output: scrubbed_reason.clone(),
                success: false,
                error_reason: Some(scrubbed_reason.clone()),
                result_dto: Some(result_dto),
                duration,
            })
        }
    }
}

fn legacy_result_dto(
    tool_name: &str,
    tool_call_id: Option<&str>,
    success: bool,
    error: Option<String>,
    permission_request: Option<PermissionRequestDto>,
    duration: Duration,
) -> ToolResultDto {
    let mut result =
        ToolCallResult::from_legacy_result(ToolResult { success, output: String::new(), error });
    result.permission_request = permission_request;
    tool_result_dto_from_call_result(tool_name, tool_call_id, &result, duration)
}

fn tool_result_dto_from_call_result(
    tool_name: &str,
    tool_call_id: Option<&str>,
    result: &ToolCallResult,
    duration: Duration,
) -> ToolResultDto {
    let mut enriched = result.clone();
    let telemetry = enriched.telemetry.get_or_insert_with(ToolCallTelemetry::default);
    telemetry.success = result.is_success();
    telemetry.attributes.insert(
        "duration_ms".to_string(),
        Value::from(u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)),
    );
    telemetry.attributes.insert("duration_secs".to_string(), json!(duration.as_secs()));
    enriched.to_dto_with_meta(Some(tool_name), tool_call_id)
}

/// 工具执行结果
///
/// 封装单个工具调用的执行结果，包含输出内容、执行状态和耗时信息。
///
/// # 字段说明
///
/// - `output`: 工具执行后的输出内容（已清理敏感信息）
/// - `success`: 执行是否成功
/// - `error_reason`: 失败时的错误原因（可选，已清理敏感信息）
/// - `duration`: 工具执行耗时
pub struct ToolExecutionOutcome {
    /// 实际执行的工具名称。
    pub tool_name: String,
    /// 工具执行输出内容
    pub output: String,
    /// 执行是否成功
    pub success: bool,
    /// 失败时的错误原因（None 表示成功执行）
    pub error_reason: Option<String>,
    /// 面向 WebSocket / ACP 等边界层的结构化工具结果。
    pub result_dto: Option<ToolResultDto>,
    /// 执行耗时
    pub duration: Duration,
}

pub fn schedule_tool_batches(
    tool_calls: &[ParsedToolCall],
    tools_registry: &[Box<dyn Tool>],
) -> Vec<ScheduledToolBatch> {
    let pending: Vec<PendingToolCall> = tool_calls
        .iter()
        .map(|call| PendingToolCall {
            name: call.name.clone(),
            arguments: call.arguments.clone(),
            tool_call_id: call.tool_call_id.clone(),
        })
        .collect();
    tool_services::scheduler::schedule_tool_calls(&pending, tools_registry)
}

/// 并行执行多个工具调用
///
/// 使用 `futures::join_all` 同时执行所有工具调用，适用于独立的、无依赖关系的工具调用。
///
/// # 参数
///
/// - `tool_calls`: 待执行的工具调用列表
/// - `tools_registry`: 工具注册表
/// - `observer`: 观测器，用于记录事件
/// - `cancellation_token`: 可选的取消令牌
///
/// # 返回值
///
/// - `Ok(Vec<ToolExecutionOutcome>)`: 所有工具执行结果
/// - `Err(...)`: 任一工具执行过程中发生取消或系统错误
///
/// # 注意事项
///
/// 并行执行时，如果任一工具失败，其他工具仍会继续执行完成。
/// 调用方应在执行前使用 `should_execute_tools_in_parallel` 判断是否适合并行。
pub async fn execute_tools_parallel<H: ContextHost>(
    tool_calls: &[PendingToolCall],
    tools_registry: &[Box<dyn Tool>],
    observer: &dyn Observer,
    tool_use_context: Arc<ToolUseContext<H>>,
    cancellation_token: Option<&CancellationToken>,
) -> Result<Vec<ToolExecutionOutcome>> {
    // 为每个工具调用创建异步任务
    let futures: Vec<_> = tool_calls
        .iter()
        .map(|call| {
            execute_one_tool(
                call,
                tools_registry,
                observer,
                tool_use_context.clone(),
                cancellation_token,
            )
        })
        .collect();

    // 并行执行所有任务并收集结果
    let results = futures_util::future::join_all(futures).await;
    // 将结果收集到 Result<Vec<...>> 中
    results.into_iter().collect()
}

/// 顺序执行多个工具调用
///
/// 按顺序依次执行每个工具调用，适用于有依赖关系或需要审批的场景。
///
/// # 参数
///
/// - `tool_calls`: 待执行的工具调用列表
/// - `tools_registry`: 工具注册表
/// - `observer`: 观测器，用于记录事件
/// - `cancellation_token`: 可选的取消令牌
///
/// # 返回值
///
/// - `Ok(Vec<ToolExecutionOutcome>)`: 所有工具执行结果
/// - `Err(...)`: 任一工具执行失败或被取消时立即返回错误
///
/// # 短路行为
///
/// 与并行执行不同，顺序执行会在第一个失败的工具处停止，
/// 不会继续执行后续工具。这确保了审批策略和错误处理的一致性。
pub async fn execute_tools_sequential<H: ContextHost>(
    tool_calls: &[PendingToolCall],
    tools_registry: &[Box<dyn Tool>],
    observer: &dyn Observer,
    tool_use_context: Arc<ToolUseContext<H>>,
    cancellation_token: Option<&CancellationToken>,
) -> Result<Vec<ToolExecutionOutcome>> {
    // 预分配结果向量容量以提高性能
    let mut outcomes = Vec::with_capacity(tool_calls.len());

    // 顺序执行每个工具调用
    for call in tool_calls {
        outcomes.push(
            execute_one_tool(
                call,
                tools_registry,
                observer,
                tool_use_context.clone(),
                cancellation_token,
            )
            .await?,
        );
    }

    Ok(outcomes)
}

pub fn should_execute_tools_in_parallel(
    tool_calls: &[ParsedToolCall],
    tools_registry: &[Box<dyn Tool>],
) -> bool {
    let batches = schedule_tool_batches(tool_calls, tools_registry);
    matches!(
        batches.as_slice(),
        [ScheduledToolBatch { mode: tool_services::scheduler::ScheduledToolBatchMode::Parallel, .. }]
    )
}

#[cfg(test)]
#[path = "execution_dto_tests.rs"]
mod dto_tests;

#[cfg(test)]
#[path = "execution_scheduling_tests.rs"]
mod scheduling_tests;

/// Schedule calls produced by model parsing through the shared dispatcher representation.
pub fn schedule_parsed_tool_batches(
    calls: &[crate::parsing::ParsedToolCall],
    registry: &[Box<dyn Tool>],
) -> Vec<tool_services::scheduler::ScheduledToolBatch> {
    let calls: Vec<_> = calls.iter().map(|call| crate::dispatcher::ParsedToolCall {
        name: call.name.clone(), arguments: call.arguments.clone(), tool_call_id: call.tool_call_id.clone(),
    }).collect();
    schedule_tool_batches(&calls, registry)
}
