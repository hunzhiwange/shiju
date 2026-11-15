//! 非 CLI 授权运行时命令的具体处理器。
//!
//! 本模块生成用户可见回复，并把授权请求、确认、拒绝、撤销等操作委托给
//! `approval_manager`。所有持久化都通过相邻的配置模块完成，避免命令处理器直接
//! 操作配置结构之外的状态。

use super::*;
use super::{
    approval_target_label, describe_non_cli_approvals, persist_non_cli_approval_to_config,
    remove_non_cli_approval_from_config,
};

fn available_tools_preview(ctx: &ChannelRuntimeContext) -> String {
    let mut available_tools =
        ctx.tools_registry.iter().map(|tool| tool.spec().id).collect::<Vec<_>>();
    available_tools.sort();
    available_tools.into_iter().take(12).collect::<Vec<_>>().join(", ")
}

/// 创建一次性所有工具放行请求。
///
/// 参数包含运行时上下文、发送者、来源通道和回复目标，用于限定后续确认作用域。
///
/// 返回值：面向用户的请求 ID、作用域和过期时间说明。
///
/// 错误处理：该函数只操作内存状态并返回字符串，不向上传递错误。
pub fn handle_request_all_tools_once(
    ctx: &ChannelRuntimeContext,
    sender: &str,
    source_channel: &str,
    reply_target: &str,
) -> String {
    // 该请求只创建 pending 记录，不直接放权；确认步骤用于防止误触发。
    let req = ctx.approval_manager.create_non_cli_pending_request(
        APPROVAL_ALL_TOOLS_ONCE_TOKEN,
        sender,
        source_channel,
        reply_target,
        Some("human-confirmed one-time bypass request for all tools/commands".to_string()),
        serde_json::Value::Null,
        None,
        None,
    );
    runtime_trace::record_event(
        "approval_request_created",
        Some(source_channel),
        None,
        None,
        None,
        Some(true),
        Some("pending one-time all-tools request created"),
        serde_json::json!({
            "request_id": req.request_id,
            "tool_name": req.tool_name,
            "sender": sender,
            "channel": source_channel,
            "expires_at": req.expires_at,
        }),
    );
    format!(
        "One-time all-tools approval request created.\nRequest ID: `{}`\nScope: next non-CLI agent tool-execution turn may run without per-tool approval prompts.\nExpires: `{}`\nConfirm with `/approve-confirm {}` (must be the same sender in this chat/channel).",
        req.request_id, req.expires_at, req.request_id
    )
}

/// 为单个工具创建待确认授权请求。
///
/// 参数：`raw_tool_name` 来自用户输入，函数内部会裁剪空白并校验工具是否存在。
///
/// 返回值：请求创建结果或使用说明。
///
/// 错误处理：未知工具和空输入都会转换为用户可读回复。
pub fn handle_request_tool_approval(
    ctx: &ChannelRuntimeContext,
    sender: &str,
    source_channel: &str,
    reply_target: &str,
    raw_tool_name: String,
) -> String {
    let tool_name = raw_tool_name.trim().to_string();
    if tool_name.is_empty() {
        "Usage: `/approve-request <tool-name>`".to_string()
    } else if !ctx.tools_registry.iter().any(|tool| tool.spec().id == tool_name.as_str()) {
        // 工具名必须精确匹配，避免把用户自由文本误当成可执行能力。
        let preview = available_tools_preview(ctx);
        format!(
            "Unknown tool `{tool_name}`.\nKnown tools (top 12): {preview}\nUse `/approve-request <tool-name>` with an exact tool name."
        )
    } else if !ctx.approval_manager.needs_approval(&tool_name) {
        format!(
            "`{tool_name}` is already approved in the current runtime policy. You can use it directly."
        )
    } else {
        let req = ctx.approval_manager.create_non_cli_pending_request(
            &tool_name,
            sender,
            source_channel,
            reply_target,
            None,
            serde_json::Value::Null,
            None,
            None,
        );
        runtime_trace::record_event(
            "approval_request_created",
            Some(source_channel),
            None,
            None,
            None,
            Some(true),
            Some("pending request created"),
            serde_json::json!({
                "request_id": req.request_id,
                "tool_name": req.tool_name,
                "sender": sender,
                "channel": source_channel,
                "expires_at": req.expires_at,
            }),
        );
        format!(
            "Approval request created.\nRequest ID: `{}`\nTool: `{}`\nExpires: `{}`\nConfirm with `/approve-confirm {}` (must be the same sender in this chat/channel).",
            req.request_id, req.tool_name, req.expires_at, req.request_id
        )
    }
}

/// 允许一个待审批请求用于当前工具调用。
///
/// 参数包含请求 ID 与发送者作用域，用于确认同一聊天上下文。
///
/// 返回值：批准结果或失败原因。
///
/// 错误处理：未找到、过期和请求者不匹配都转换为用户可读回复。
pub fn handle_approve_pending_request(
    ctx: &ChannelRuntimeContext,
    sender: &str,
    source_channel: &str,
    reply_target: &str,
    raw_request_id: String,
) -> String {
    let request_id = raw_request_id.trim().to_string();
    if request_id.is_empty() {
        "Usage: `/approve-allow <request-id>`".to_string()
    } else {
        match ctx.approval_manager.confirm_non_cli_pending_request(
            &request_id,
            sender,
            source_channel,
            reply_target,
        ) {
            Ok(req) => {
                // 本路径只记录当前 invocation 的批准，不写入长期配置。
                ctx.approval_manager
                    .record_non_cli_pending_resolution(&request_id, ApprovalResponse::Yes);
                runtime_trace::record_event(
                    "approval_request_allowed",
                    Some(source_channel),
                    None,
                    None,
                    None,
                    Some(true),
                    Some("pending request allowed for current tool invocation"),
                    serde_json::json!({
                        "request_id": request_id,
                        "tool_name": req.tool_name,
                        "sender": sender,
                        "channel": source_channel,
                    }),
                );
                format!(
                    "Approved pending request `{}` for this invocation of `{}`.",
                    req.request_id, req.tool_name
                )
            }
            Err(PendingApprovalError::NotFound) => {
                format!("Pending approval request `{request_id}` was not found.")
            }
            Err(PendingApprovalError::Expired) => {
                format!("Pending approval request `{request_id}` has expired.")
            }
            Err(PendingApprovalError::RequesterMismatch) => format!(
                "Pending approval request `{request_id}` can only be approved by the same sender in the same chat/channel that created it."
            ),
        }
    }
}

/// 确认待授权请求，并按目标类型授予运行时或持久化授权。
///
/// 参数包含请求 ID 与发送者作用域，用于保证只有创建者能确认。
///
/// 返回值：确认结果、持久化结果和必要的策略提醒。
///
/// 错误处理：pending 状态错误转换为回复；配置写入失败不会撤销内存授权，但会明确告知用户。
pub async fn handle_confirm_tool_approval(
    ctx: &ChannelRuntimeContext,
    sender: &str,
    source_channel: &str,
    reply_target: &str,
    raw_request_id: String,
) -> String {
    let request_id = raw_request_id.trim().to_string();
    if request_id.is_empty() {
        return "Usage: `/approve-confirm <request-id>`".to_string();
    }

    match ctx.approval_manager.confirm_non_cli_pending_request(
        &request_id,
        sender,
        source_channel,
        reply_target,
    ) {
        Ok(req) => {
            ctx.approval_manager
                .record_non_cli_pending_resolution(&request_id, ApprovalResponse::Yes);
            let tool_name = req.tool_name;
            let approval_message = if tool_name == APPROVAL_ALL_TOOLS_ONCE_TOKEN {
                // 全工具放行只允许一次且不持久化，避免扩大长期权限边界。
                let remaining = ctx.approval_manager.grant_non_cli_allow_all_once();
                format!(
                    "Approved one-time all-tools bypass from request `{request_id}`.\nApplies to the next non-CLI agent tool-execution turn only.\nThis bypass is runtime-only and does not persist to config.\nChannel exclusions from `autonomy.non_cli_excluded_tools` still apply.\nQueued one-time all-tools bypass tokens: `{remaining}`."
                )
            } else {
                ctx.approval_manager.grant_non_cli_session(&tool_name);
                ctx.approval_manager.apply_persistent_runtime_grant(&tool_name);
                match persist_non_cli_approval_to_config(ctx, &tool_name).await {
                    Ok(Some(path)) => format!(
                        "Approved supervised execution for `{tool_name}` from request `{request_id}`.\nPersisted to `{}` so future channel sessions (including after restart) remain approved.",
                        path.display()
                    ),
                    Ok(None) => format!(
                        "Approved supervised execution for `{tool_name}` from request `{request_id}`.\nNo runtime config path was found, so this approval is active for the current daemon runtime only."
                    ),
                    Err(err) => format!(
                        "Approved supervised execution for `{tool_name}` from request `{request_id}` in-memory.\nFailed to persist this approval to config: {err}"
                    ),
                }
            };
            runtime_trace::record_event(
                "approval_request_confirmed",
                Some(source_channel),
                None,
                None,
                None,
                Some(true),
                Some("pending request confirmed"),
                serde_json::json!({
                    "request_id": request_id,
                    "tool_name": tool_name,
                    "sender": sender,
                    "channel": source_channel,
                }),
            );

            // 即使批准成功，通道排除列表仍是更高优先级的安全边界，需要显式提示。
            if tool_name != APPROVAL_ALL_TOOLS_ONCE_TOKEN
                && is_non_cli_tool_excluded(ctx, &tool_name)
            {
                format!(
                    "{approval_message}\nNote: `{tool_name}` is currently listed in `autonomy.non_cli_excluded_tools` for this runtime. Remove it from config; the channel runtime auto-reloads this list from disk."
                )
            } else {
                approval_message
            }
        }
        Err(PendingApprovalError::NotFound) => {
            runtime_trace::record_event(
                "approval_request_confirmed",
                Some(source_channel),
                None,
                None,
                None,
                Some(false),
                Some("pending request not found"),
                serde_json::json!({
                    "request_id": request_id,
                    "sender": sender,
                    "channel": source_channel,
                }),
            );
            format!(
                "Pending approval request `{request_id}` was not found. Create one with `/approve-request <tool-name>` or `/approve-all-once`."
            )
        }
        Err(PendingApprovalError::Expired) => {
            runtime_trace::record_event(
                "approval_request_confirmed",
                Some(source_channel),
                None,
                None,
                None,
                Some(false),
                Some("pending request expired"),
                serde_json::json!({
                    "request_id": request_id,
                    "sender": sender,
                    "channel": source_channel,
                }),
            );
            format!("Pending approval request `{request_id}` has expired.")
        }
        Err(PendingApprovalError::RequesterMismatch) => {
            runtime_trace::record_event(
                "approval_request_confirmed",
                Some(source_channel),
                None,
                None,
                None,
                Some(false),
                Some("pending request confirmer mismatch"),
                serde_json::json!({
                    "request_id": request_id,
                    "sender": sender,
                    "channel": source_channel,
                }),
            );
            format!(
                "Pending approval request `{request_id}` can only be confirmed by the same sender in the same chat/channel that created it."
            )
        }
    }
}

/// 拒绝一个待授权请求。
///
/// 参数包含请求 ID 与发送者作用域，用于保证只有创建者所在上下文能拒绝。
///
/// 返回值：拒绝结果或失败原因。
///
/// 错误处理：pending 状态错误转换为用户可读回复。
pub fn handle_deny_tool_approval(
    ctx: &ChannelRuntimeContext,
    sender: &str,
    source_channel: &str,
    reply_target: &str,
    raw_request_id: String,
) -> String {
    let request_id = raw_request_id.trim().to_string();
    if request_id.is_empty() {
        return "Usage: `/approve-deny <request-id>`".to_string();
    }

    match ctx.approval_manager.reject_non_cli_pending_request(
        &request_id,
        sender,
        source_channel,
        reply_target,
    ) {
        Ok(req) => {
            ctx.approval_manager
                .record_non_cli_pending_resolution(&request_id, ApprovalResponse::No);
            runtime_trace::record_event(
                "approval_request_denied",
                Some(source_channel),
                None,
                None,
                None,
                Some(true),
                Some("pending request denied"),
                serde_json::json!({
                    "request_id": request_id,
                    "tool_name": req.tool_name,
                    "sender": sender,
                    "channel": source_channel,
                }),
            );
            format!(
                "Denied pending approval request `{}` for tool `{}`.",
                req.request_id, req.tool_name
            )
        }
        Err(PendingApprovalError::NotFound) => {
            runtime_trace::record_event(
                "approval_request_denied",
                Some(source_channel),
                None,
                None,
                None,
                Some(false),
                Some("pending request not found"),
                serde_json::json!({
                    "request_id": request_id,
                    "sender": sender,
                    "channel": source_channel,
                }),
            );
            format!("Pending approval request `{request_id}` was not found.")
        }
        Err(PendingApprovalError::Expired) => {
            runtime_trace::record_event(
                "approval_request_denied",
                Some(source_channel),
                None,
                None,
                None,
                Some(false),
                Some("pending request expired"),
                serde_json::json!({
                    "request_id": request_id,
                    "sender": sender,
                    "channel": source_channel,
                }),
            );
            format!("Pending approval request `{request_id}` has expired.")
        }
        Err(PendingApprovalError::RequesterMismatch) => {
            runtime_trace::record_event(
                "approval_request_denied",
                Some(source_channel),
                None,
                None,
                None,
                Some(false),
                Some("pending request denier mismatch"),
                serde_json::json!({
                    "request_id": request_id,
                    "sender": sender,
                    "channel": source_channel,
                }),
            );
            format!(
                "Pending approval request `{request_id}` can only be denied by the same sender in the same chat/channel that created it."
            )
        }
    }
}

/// 列出当前发送者和聊天作用域内的待授权请求。
///
/// 参数用于筛选 sender、channel 和 reply target。
///
/// 返回值：待审批请求列表或空状态说明。
///
/// 错误处理：该函数只读内存状态，不产生错误。
pub fn handle_list_pending_approvals(
    ctx: &ChannelRuntimeContext,
    sender: &str,
    source_channel: &str,
    reply_target: &str,
) -> String {
    let rows = ctx.approval_manager.list_non_cli_pending_requests(
        Some(sender),
        Some(source_channel),
        Some(reply_target),
    );
    if rows.is_empty() {
        "No pending approval requests for your current sender+chat/channel scope.".to_string()
    } else {
        let mut response = String::new();
        response.push_str("Pending approval requests (sender+chat/channel scoped):\n");
        for req in rows {
            let reason =
                req.reason.as_deref().filter(|text| !text.trim().is_empty()).unwrap_or("n/a");
            let _ = writeln!(
                response,
                "- {}: tool={}, expires_at={}, reason={}",
                req.request_id,
                approval_target_label(&req.tool_name),
                req.expires_at,
                reason
            );
        }
        response
    }
}

/// 直接批准单个工具并尝试持久化。
///
/// 参数：`raw_tool_name` 来自用户输入，函数内部会裁剪并校验工具名。
///
/// 返回值：批准、持久化和清理 pending 的结果说明。
///
/// 错误处理：配置写入失败会保留内存授权并在回复中说明。
pub async fn handle_approve_tool(
    ctx: &ChannelRuntimeContext,
    raw_tool_name: String,
) -> String {
    let tool_name = raw_tool_name.trim().to_string();
    if tool_name.is_empty() {
        return "Usage: `/approve <tool-name>`".to_string();
    }
    if !ctx.tools_registry.iter().any(|tool| tool.spec().id == tool_name.as_str()) {
        let preview = available_tools_preview(ctx);
        return format!(
            "Unknown tool `{tool_name}`.\nKnown tools (top 12): {preview}\nUse `/approve <tool-name>` with an exact tool name."
        );
    }

    // 直接批准会清理同工具 pending，避免之后旧请求再次被确认造成混淆。
    let cleared_pending = ctx.approval_manager.clear_non_cli_pending_requests_for_tool(&tool_name);
    ctx.approval_manager.grant_non_cli_session(&tool_name);
    ctx.approval_manager.apply_persistent_runtime_grant(&tool_name);
    let persistence_message = match persist_non_cli_approval_to_config(ctx, &tool_name).await {
        Ok(Some(path)) => format!(
            "Approved supervised execution for `{tool_name}`.\nPersisted to `{}` so future channel sessions (including after restart) remain approved.",
            path.display()
        ),
        Ok(None) => format!(
            "Approved supervised execution for `{tool_name}`.\nNo runtime config path was found, so this approval is active for the current daemon runtime only."
        ),
        Err(err) => format!(
            "Approved supervised execution for `{tool_name}` in-memory.\nFailed to persist this approval to config: {err}"
        ),
    };

    if is_non_cli_tool_excluded(ctx, &tool_name) {
        format!(
            "{persistence_message}\nRuntime pending requests cleared: {cleared_pending}.\nNote: `{tool_name}` is currently listed in `autonomy.non_cli_excluded_tools` for this runtime. Remove it from config; the channel runtime auto-reloads this list from disk."
        )
    } else {
        format!("{persistence_message}\nRuntime pending requests cleared: {cleared_pending}.")
    }
}

/// 撤销单个工具的非 CLI 授权。
///
/// 参数：`raw_tool_name` 来自用户输入，函数内部会裁剪空白。
///
/// 返回值：运行时会话授权、运行时持久化授权、pending 和配置移除结果。
///
/// 错误处理：配置移除失败会转换为用户可读回复，并保留已完成的运行时撤销结果。
pub async fn handle_unapprove_tool(
    ctx: &ChannelRuntimeContext,
    raw_tool_name: String,
) -> String {
    let tool_name = raw_tool_name.trim().to_string();
    if tool_name.is_empty() {
        return "Usage: `/unapprove <tool-name>`".to_string();
    }

    // 撤销同时清理 session grant、运行时持久化视图和 pending，保证后续状态单调收紧。
    let removed_session = ctx.approval_manager.revoke_non_cli_session(&tool_name);
    let removed_runtime_persistent =
        ctx.approval_manager.apply_persistent_runtime_revoke(&tool_name);
    let removed_pending = ctx.approval_manager.clear_non_cli_pending_requests_for_tool(&tool_name);
    match remove_non_cli_approval_from_config(ctx, &tool_name).await {
        Ok(Some((path, removed_persistent))) => format!(
            "Persistent approval removed for `{tool_name}`: {}.\nRuntime effective auto_approve removed: {}.\nRuntime pending requests cleared: {}.\nConfig path: `{}`.\nRuntime session grant removed: {}.",
            if removed_persistent { "yes" } else { "no (not present)" },
            if removed_runtime_persistent { "yes" } else { "no (not present)" },
            removed_pending,
            path.display(),
            if removed_session { "yes" } else { "no" }
        ),
        Ok(None) => format!(
            "Runtime config path was not found.\nRuntime session grant removed for `{tool_name}`: {}.",
            if removed_session { "yes" } else { "no" }
        ),
        Err(err) => format!(
            "Removed runtime session grant for `{tool_name}`: {}.\nFailed to persist removal to config: {err}",
            if removed_session { "yes" } else { "no" }
        ),
    }
}

/// 列出当前非 CLI 授权状态。
///
/// 参数用于限定 pending 请求作用域并展示通道级策略。
///
/// 返回值：授权摘要或读取失败信息。
///
/// 错误处理：底层摘要读取失败会转换为用户可见字符串。
pub async fn handle_list_approvals(
    ctx: &ChannelRuntimeContext,
    sender: &str,
    source_channel: &str,
    reply_target: &str,
) -> String {
    match describe_non_cli_approvals(ctx, sender, source_channel, reply_target).await {
        Ok(summary) => summary,
        Err(err) => format!("Failed to read approval state: {err}"),
    }
}

#[cfg(test)]
#[path = "approval_commands_tests.rs"]
mod approval_commands_tests;
