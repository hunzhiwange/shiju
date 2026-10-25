//! 运行时命令的执行入口。
//!
//! 本模块在通道消息进入模型前拦截控制命令，并统一完成权限检查、自然语言授权
//! 策略处理和响应发送。这里是非 CLI 授权路径的重要边界：命令可被解析并不代表
//! 发送者一定有权执行。

use super::super::*;
use super::approval_commands::{
    handle_approve_pending_request, handle_approve_tool, handle_confirm_tool_approval,
    handle_deny_tool_approval, handle_list_approvals, handle_list_pending_approvals,
    handle_request_all_tools_once, handle_request_tool_approval, handle_unapprove_tool,
};
use super::command::{
    ChannelRuntimeCommand, is_approval_management_command, parse_runtime_command,
};
use super::non_cli_natural_language_mode_label;
use super::session_commands::{
    handle_new_session, handle_set_model, handle_set_provider, handle_show_model,
    handle_show_providers, handle_task_mode,
};
use super::task_mode::handle_task_mode_message_if_needed;

async fn send_runtime_command_response(
    channel: &Arc<dyn Channel>,
    msg: &traits::ChannelMessage,
    response: String,
) {
    if let Err(err) = channel
        .send(&SendMessage::new(response, &msg.reply_target).in_thread(msg.thread_ts.clone()))
        .await
    {
        tracing::warn!("Failed to send runtime command response on {}: {err}", channel.name());
    }
}

/// 如消息是运行时命令则执行它，并返回是否已消费该消息。
///
/// 参数：
/// - `ctx`：通道运行时上下文。
/// - `msg`：入站通道消息。
/// - `target_channel`：用于回复的通道实例，缺失时命令会被静默消费。
///
/// 返回值：消息被运行时命令或任务模式处理时返回 `true`；应继续进入模型时返回 `false`。
///
/// 错误处理：各子处理器把结果格式化为用户回复；发送失败只记录警告，避免监听循环中断。
pub(crate) async fn handle_runtime_command_if_needed(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    target_channel: Option<&Arc<dyn Channel>>,
) -> bool {
    let sender_key = scoped_history_key(ctx, msg);
    if parse_runtime_command(&msg.channel, &msg.content).is_none() {
        return handle_task_mode_message_if_needed(ctx, msg, &sender_key, target_channel).await;
    }

    let is_slash_command = msg.content.trim_start().starts_with('/');
    let Some(mut command) = parse_runtime_command(&msg.channel, &msg.content) else {
        return false;
    };

    let Some(channel) = target_channel else {
        return true;
    };

    let mut current = get_route_selection(ctx, &sender_key);
    let sender = msg.sender.as_str();
    let source_channel = msg.channel.as_str();
    let reply_target = msg.reply_target.as_str();
    let is_natural_language_approval_command =
        !is_slash_command && is_approval_management_command(&command);

    // 非 CLI 授权命令必须先校验发送者，防止普通聊天成员通过自然语言或斜杠命令放权。
    if is_approval_management_command(&command)
        && !ctx.approval_manager.is_non_cli_approval_actor_allowed(source_channel, sender)
    {
        let mut approvers =
            ctx.approval_manager.non_cli_approval_approvers().into_iter().collect::<Vec<_>>();
        approvers.sort();
        let allowed = if approvers.is_empty() {
            "(any channel-allowed sender)".to_string()
        } else {
            approvers.join(", ")
        };
        let response = format!(
            "Approval-management command denied for sender `{sender}` on channel `{source_channel}`.\nAllowed approvers: {allowed}\nConfigure `[autonomy].non_cli_approval_approvers` to adjust this policy."
        );
        runtime_trace::record_event(
            "approval_management_denied",
            Some(source_channel),
            None,
            None,
            None,
            Some(false),
            Some("sender not allowed to manage non-cli approvals"),
            serde_json::json!({
                "sender": sender,
                "channel": source_channel,
                "allowed_approvers": approvers,
            }),
        );
        send_runtime_command_response(channel, msg, response).await;
        return true;
    }

    if is_natural_language_approval_command {
        let mode =
            ctx.approval_manager.non_cli_natural_language_approval_mode_for_channel(source_channel);
        match mode {
            NonCliNaturalLanguageApprovalMode::Disabled => {
                let response = "Natural-language approval commands are disabled by runtime policy.\nUse explicit slash commands such as `/approve <tool-name>`, `/approve-request <tool-name>`, `/approve-all-once`, `/approve-allow <request-id>`, `/approve-confirm <request-id>`, `/approve-deny <request-id>`, `/unapprove <tool-name>`, and `/approvals`.".to_string();
                runtime_trace::record_event(
                    "approval_management_natural_language_denied",
                    Some(source_channel),
                    None,
                    None,
                    None,
                    Some(false),
                    Some("natural-language approval commands disabled by policy"),
                    serde_json::json!({
                        "sender": sender,
                        "channel": source_channel,
                        "mode": non_cli_natural_language_mode_label(mode),
                    }),
                );
                send_runtime_command_response(channel, msg, response).await;
                return true;
            }
            NonCliNaturalLanguageApprovalMode::RequestConfirm => {}
            NonCliNaturalLanguageApprovalMode::Direct => {
                if let ChannelRuntimeCommand::RequestToolApproval(tool_name) = &command {
                    // direct 模式仍只提升“请求工具授权”这一窄意图，其他敏感命令保持原语义。
                    command = ChannelRuntimeCommand::ApproveTool(tool_name.clone());
                    runtime_trace::record_event(
                        "approval_management_natural_language_promoted_to_direct",
                        Some(source_channel),
                        None,
                        None,
                        None,
                        Some(true),
                        Some("natural-language request promoted to direct approval"),
                        serde_json::json!({
                            "sender": sender,
                            "channel": source_channel,
                            "mode": non_cli_natural_language_mode_label(mode),
                        }),
                    );
                }
            }
        }
    }

    let response = match command {
        ChannelRuntimeCommand::ListProjects
        | ChannelRuntimeCommand::ShowProject
        | ChannelRuntimeCommand::SelectProject(_) => {
            super::project_commands::handle_project_command(ctx, msg).await
        }
        ChannelRuntimeCommand::ShowProviders => handle_show_providers(&current),
        ChannelRuntimeCommand::SetProvider(raw_provider) => {
            handle_set_provider(ctx, &sender_key, &mut current, raw_provider).await
        }
        ChannelRuntimeCommand::ShowModel => {
            handle_show_model(&current, ctx.workspace_dir.as_path())
        }
        ChannelRuntimeCommand::SetModel(raw_model) => {
            handle_set_model(ctx, &sender_key, &mut current, raw_model)
        }
        ChannelRuntimeCommand::NewSession => handle_new_session(ctx, msg, &sender_key).await,
        ChannelRuntimeCommand::TaskMode => handle_task_mode(ctx, msg, &sender_key).await,
        ChannelRuntimeCommand::RequestAllToolsOnce => {
            handle_request_all_tools_once(ctx, sender, source_channel, reply_target)
        }
        ChannelRuntimeCommand::RequestToolApproval(raw_tool_name) => {
            handle_request_tool_approval(ctx, sender, source_channel, reply_target, raw_tool_name)
        }
        ChannelRuntimeCommand::ApprovePendingRequest(raw_request_id) => {
            handle_approve_pending_request(
                ctx,
                sender,
                source_channel,
                reply_target,
                raw_request_id,
            )
        }
        ChannelRuntimeCommand::ConfirmToolApproval(raw_request_id) => {
            handle_confirm_tool_approval(ctx, sender, source_channel, reply_target, raw_request_id)
                .await
        }
        ChannelRuntimeCommand::DenyToolApproval(raw_request_id) => {
            handle_deny_tool_approval(ctx, sender, source_channel, reply_target, raw_request_id)
        }
        ChannelRuntimeCommand::ListPendingApprovals => {
            handle_list_pending_approvals(ctx, sender, source_channel, reply_target)
        }
        ChannelRuntimeCommand::ApproveTool(raw_tool_name) => {
            handle_approve_tool(ctx, raw_tool_name).await
        }
        ChannelRuntimeCommand::UnapproveTool(raw_tool_name) => {
            handle_unapprove_tool(ctx, raw_tool_name).await
        }
        ChannelRuntimeCommand::ListApprovals => {
            handle_list_approvals(ctx, sender, source_channel, reply_target).await
        }
    };

    send_runtime_command_response(channel, msg, response).await;
    true
}
