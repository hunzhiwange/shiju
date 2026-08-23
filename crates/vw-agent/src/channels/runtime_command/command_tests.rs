// 本文件验证运行时命令解析、渠道能力约束与自然语言审批意图识别。

use super::*;

#[test]
fn supports_runtime_model_switch_allows_only_interactive_channels() {
    assert!(supports_runtime_model_switch("telegram"));
    assert!(supports_runtime_model_switch("discord"));
    assert!(!supports_runtime_model_switch("slack"));
    assert!(!supports_runtime_model_switch("Telegram"));
}

#[test]
fn parse_runtime_command_respects_channel_model_support_and_payloads() {
    assert_eq!(
        parse_runtime_command("telegram", "/model gpt-5"),
        Some(ChannelRuntimeCommand::SetModel("gpt-5".to_string()))
    );
    assert_eq!(parse_runtime_command("qq", "/model gpt-5"), None);
    assert_eq!(
        parse_runtime_command("discord", "/models"),
        Some(ChannelRuntimeCommand::ShowProviders)
    );
    assert_eq!(
        parse_runtime_command("discord", "/models openai"),
        Some(ChannelRuntimeCommand::SetProvider("openai".to_string()))
    );
    assert_eq!(parse_runtime_command("telegram", "/model"), Some(ChannelRuntimeCommand::ShowModel));
    assert_eq!(parse_runtime_command("qq", "/models"), None);
}

#[test]
fn parse_runtime_command_handles_session_task_and_telegram_bot_suffixes() {
    assert_eq!(parse_runtime_command("telegram", "/new"), Some(ChannelRuntimeCommand::NewSession));
    assert_eq!(
        parse_runtime_command("telegram", "/clear@vibe_bot trailing ignored"),
        Some(ChannelRuntimeCommand::NewSession)
    );
    assert_eq!(
        parse_runtime_command("telegram", "/session"),
        Some(ChannelRuntimeCommand::NewSession)
    );
    assert_eq!(parse_runtime_command("telegram", "/task"), Some(ChannelRuntimeCommand::TaskMode));
    assert_eq!(parse_runtime_command("telegram", "/unknown"), None);
    assert_eq!(parse_runtime_command("telegram", " /unknown "), None);
}

#[test]
fn parse_runtime_command_handles_all_approval_slash_commands() {
    assert_eq!(
        parse_runtime_command("telegram", "/approve-all-once"),
        Some(ChannelRuntimeCommand::RequestAllToolsOnce)
    );
    assert_eq!(
        parse_runtime_command("telegram", "/approve-request shell tool"),
        Some(ChannelRuntimeCommand::RequestToolApproval("shell tool".to_string()))
    );
    assert_eq!(
        parse_runtime_command("telegram", "/approve-confirm apr-1"),
        Some(ChannelRuntimeCommand::ConfirmToolApproval("apr-1".to_string()))
    );
    assert_eq!(
        parse_runtime_command("telegram", "/approve-allow apr-2"),
        Some(ChannelRuntimeCommand::ApprovePendingRequest("apr-2".to_string()))
    );
    assert_eq!(
        parse_runtime_command("telegram", "/approve-deny apr-3"),
        Some(ChannelRuntimeCommand::DenyToolApproval("apr-3".to_string()))
    );
    assert_eq!(
        parse_runtime_command("telegram", "/approve-pending"),
        Some(ChannelRuntimeCommand::ListPendingApprovals)
    );
    assert_eq!(
        parse_runtime_command("telegram", "/approve shell"),
        Some(ChannelRuntimeCommand::ApproveTool("shell".to_string()))
    );
    assert_eq!(
        parse_runtime_command("telegram", "/unapprove shell"),
        Some(ChannelRuntimeCommand::UnapproveTool("shell".to_string()))
    );
    assert_eq!(
        parse_runtime_command("telegram", "/approvals"),
        Some(ChannelRuntimeCommand::ListApprovals)
    );
    assert_eq!(
        parse_runtime_command("telegram", "/approve   shell   "),
        Some(ChannelRuntimeCommand::ApproveTool("shell".to_string()))
    );
}

#[test]
// 运行时参数只接受受限令牌，危险字符不能进入配置操作。
fn runtime_token_parser_rejects_free_text_and_unsafe_chars() {
    assert!(is_runtime_token("tool:name-1"));
    assert!(is_runtime_token(" tool_name.2 "));
    assert!(!is_runtime_token("tool name"));
    assert!(!is_runtime_token("tool/name"));
    assert!(!is_runtime_token(""));
    assert!(!is_runtime_token("   "));
    assert!(!is_runtime_token("工具"));
    assert_eq!(
        extract_runtime_tail_token("approve tool:name-1", &["approve "]),
        Some("tool:name-1".to_string())
    );
    assert_eq!(extract_runtime_tail_token("approve tool name", &["approve "]), None);
    assert_eq!(extract_runtime_tail_token("deny apr-1", &["approve "]), None);
}

#[test]
fn contains_any_fragment_reports_presence_and_absence() {
    assert!(contains_any_fragment("approve all tools once", &["all tools", "全部工具"]));
    assert!(!contains_any_fragment("approve shell", &["all tools", "全部工具"]));
}

#[test]
fn natural_language_approval_intents_are_narrow() {
    assert!(is_natural_language_all_tools_once_intent("approve all tools once"));
    assert!(is_natural_language_all_tools_once_intent("allow all commands one time"));
    assert!(is_natural_language_all_tools_once_intent("请允许所有工具这次执行"));
    assert!(!is_natural_language_all_tools_once_intent("approve tool_x"));
    assert!(!is_natural_language_all_tools_once_intent(""));
    assert!(!is_natural_language_all_tools_once_intent("allow all tools"));
}

#[test]
fn approval_target_label_humanizes_all_tools_token() {
    assert_eq!(
        approval_target_label(APPROVAL_ALL_TOOLS_ONCE_TOKEN),
        "all tools/commands (one-time bypass token)"
    );
    assert_eq!(approval_target_label("shell"), "shell");
}

#[test]
fn parse_natural_language_runtime_command_handles_list_and_all_tools_intents() {
    assert_eq!(parse_natural_language_runtime_command(""), None);
    assert_eq!(
        parse_natural_language_runtime_command("pending approvals"),
        Some(ChannelRuntimeCommand::ListPendingApprovals)
    );
    assert_eq!(
        parse_natural_language_runtime_command("show pending approvals"),
        Some(ChannelRuntimeCommand::ListPendingApprovals)
    );
    assert_eq!(
        parse_natural_language_runtime_command("查看授权"),
        Some(ChannelRuntimeCommand::ListApprovals)
    );
    assert_eq!(
        parse_natural_language_runtime_command("list approvals"),
        Some(ChannelRuntimeCommand::ListApprovals)
    );
    assert_eq!(
        parse_natural_language_runtime_command("approve all once"),
        Some(ChannelRuntimeCommand::RequestAllToolsOnce)
    );
    assert_eq!(
        parse_natural_language_runtime_command("请授权所有命令一次"),
        Some(ChannelRuntimeCommand::RequestAllToolsOnce)
    );
}

#[test]
fn parse_natural_language_runtime_command_accepts_only_single_safe_tail_token() {
    assert_eq!(
        parse_natural_language_runtime_command("confirm apr-123"),
        Some(ChannelRuntimeCommand::ConfirmToolApproval("apr-123".to_string()))
    );
    assert_eq!(
        parse_natural_language_runtime_command("确认授权 apr-456"),
        Some(ChannelRuntimeCommand::ConfirmToolApproval("apr-456".to_string()))
    );
    assert_eq!(
        parse_natural_language_runtime_command("revoke tool shell.exec"),
        Some(ChannelRuntimeCommand::UnapproveTool("shell.exec".to_string()))
    );
    assert_eq!(
        parse_natural_language_runtime_command("取消授权 shell"),
        Some(ChannelRuntimeCommand::UnapproveTool("shell".to_string()))
    );
    assert_eq!(
        parse_natural_language_runtime_command("approve tool file_read"),
        Some(ChannelRuntimeCommand::RequestToolApproval("file_read".to_string()))
    );
    assert_eq!(
        parse_natural_language_runtime_command("请放开 browser.open"),
        Some(ChannelRuntimeCommand::RequestToolApproval("browser.open".to_string()))
    );
    assert_eq!(parse_natural_language_runtime_command("approve tool with spaces"), None);
    assert_eq!(parse_natural_language_runtime_command("confirm apr/123"), None);
    assert_eq!(parse_natural_language_runtime_command("please approve shell"), None);
}

#[test]
fn parse_runtime_command_delegates_plain_text_to_natural_language_parser() {
    assert_eq!(
        parse_runtime_command("telegram", " approve tool shell "),
        Some(ChannelRuntimeCommand::RequestToolApproval("shell".to_string()))
    );
    assert_eq!(parse_runtime_command("telegram", "ordinary chat"), None);
}

#[test]
fn approval_management_command_classifier_covers_all_sensitive_commands() {
    let sensitive = [
        ChannelRuntimeCommand::RequestAllToolsOnce,
        ChannelRuntimeCommand::RequestToolApproval("shell".to_string()),
        ChannelRuntimeCommand::ConfirmToolApproval("apr-1".to_string()),
        ChannelRuntimeCommand::ApprovePendingRequest("apr-2".to_string()),
        ChannelRuntimeCommand::DenyToolApproval("apr-3".to_string()),
        ChannelRuntimeCommand::ListPendingApprovals,
        ChannelRuntimeCommand::ApproveTool("shell".to_string()),
        ChannelRuntimeCommand::UnapproveTool("shell".to_string()),
        ChannelRuntimeCommand::ListApprovals,
    ];
    for command in sensitive {
        assert!(is_approval_management_command(&command), "{command:?}");
    }

    assert!(is_approval_management_command(&ChannelRuntimeCommand::RequestAllToolsOnce));
    assert!(!is_approval_management_command(&ChannelRuntimeCommand::ShowModel));
    assert!(!is_approval_management_command(&ChannelRuntimeCommand::SetModel("gpt-5".to_string())));
}
