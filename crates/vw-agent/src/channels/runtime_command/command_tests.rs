// 本文件验证运行时命令解析、渠道能力约束与自然语言审批意图识别。

use super::*;

#[test]
fn supports_runtime_model_switch_allows_all_text_channels() {
    for platform in ["telegram", "discord", "slack", "qq", "lark", "Telegram"] {
        assert_eq!(
            parse_runtime_command(platform, "/model"),
            Some(ChannelRuntimeCommand::ShowModel)
        );
    }
}

#[test]
fn parse_runtime_command_respects_channel_model_support_and_payloads() {
    assert_eq!(
        parse_runtime_command("telegram", "/model gpt-5"),
        Some(ChannelRuntimeCommand::SetModel("gpt-5".to_string()))
    );
    assert_eq!(
        parse_runtime_command("qq", "/model gpt-5"),
        Some(ChannelRuntimeCommand::SetModel("gpt-5".into()))
    );
    assert_eq!(
        parse_runtime_command("discord", "/models"),
        Some(ChannelRuntimeCommand::ShowProviders)
    );
    assert_eq!(
        parse_runtime_command("discord", "/models openai"),
        Some(ChannelRuntimeCommand::SetProvider("openai".to_string()))
    );
    assert_eq!(parse_runtime_command("telegram", "/model"), Some(ChannelRuntimeCommand::ShowModel));
    assert_eq!(parse_runtime_command("qq", "/models"), Some(ChannelRuntimeCommand::ShowProviders));
}

#[test]
fn parse_runtime_command_handles_session_task_and_telegram_bot_suffixes() {
    assert_eq!(parse_runtime_command("telegram", "/new"), Some(ChannelRuntimeCommand::NewSession));
    assert_eq!(
        parse_runtime_command("telegram", "/clear@vibe_bot"),
        Some(ChannelRuntimeCommand::ClearSession)
    );
    assert_eq!(
        parse_runtime_command("telegram", "/session"),
        Some(ChannelRuntimeCommand::NewSession)
    );
    assert_eq!(parse_runtime_command("telegram", "/task"), Some(ChannelRuntimeCommand::TaskMode));
    assert!(matches!(
        parse_runtime_command("telegram", "/unknown"),
        Some(ChannelRuntimeCommand::Error(_))
    ));
    assert!(matches!(
        parse_runtime_command("telegram", " /unknown "),
        Some(ChannelRuntimeCommand::Error(_))
    ));
}

#[test]
fn parse_runtime_command_handles_all_approval_slash_commands() {
    assert_eq!(
        parse_runtime_command("telegram", "/approve-all-once"),
        Some(ChannelRuntimeCommand::RequestAllToolsOnce)
    );
    assert_eq!(
        parse_runtime_command("telegram", "/approve-request shell tool"),
        Some(ChannelRuntimeCommand::Error(
            "参数错误。用法：/approve-request <tool-name>。使用 /help approve-request 查看详情。"
                .into()
        ))
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

#[test]
fn channel_command_catalog_lark_model_is_consumed() {
    assert!(parse_runtime_command("lark", "/model gpt-5").is_some());
}
#[test]
fn channel_command_catalog_qq_models_is_consumed() {
    assert!(parse_runtime_command("qq", "/models").is_some());
}
#[test]
fn channel_command_catalog_named_instance_is_consumed() {
    assert!(parse_runtime_command("lark:work", "/model gpt-5").is_some());
}
#[test]
fn channel_command_catalog_unknown_slash_is_consumed() {
    assert!(parse_runtime_command("slack", "/not-a-command").is_some());
}
#[test]
fn channel_command_catalog_help_is_consumed() {
    assert!(parse_runtime_command("lark", "/help model").is_some());
}
#[test]
fn channel_command_catalog_settings_effort_status_are_consumed() {
    for command in ["/settings", "/effort high", "/status"] {
        assert!(parse_runtime_command("qq", command).is_some(), "{command}");
    }
}

#[test]
fn channel_command_catalog_every_entry_and_alias_reaches_dispatch() {
    for meta in super::runtime_command::catalog::COMMANDS {
        let arg = if meta.min_args > 0 { " test" } else { "" };
        for name in std::iter::once(&meta.name).chain(meta.aliases.iter()) {
            assert!(
                !matches!(
                    parse_runtime_command("lark", &format!("/{name}{arg}")),
                    None | Some(ChannelRuntimeCommand::Error(_))
                ),
                "{name}"
            );
        }
    }
}
#[test]
fn channel_command_catalog_wrong_arguments_are_errors() {
    for text in [
        "/new extra",
        "/approve",
        "/status extra",
        "/help model extra",
        "/model a b",
        "/projects extra",
    ] {
        assert!(
            matches!(parse_runtime_command("qq", text), Some(ChannelRuntimeCommand::Error(_))),
            "{text}"
        );
    }
    assert_eq!(parse_runtime_command("lark", "普通聊天"), None);
}

#[derive(Default)]
struct CatalogChannel(tokio::sync::Mutex<Vec<SendMessage>>);
#[async_trait::async_trait]
impl Channel for CatalogChannel {
    fn name(&self) -> &str {
        "lark"
    }
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        self.0.lock().await.push(message.clone());
        Ok(())
    }
    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

pub(super) async fn catalog_context()
-> (tempfile::TempDir, ChannelRuntimeContext, traits::ChannelMessage) {
    let (temp, mut ctx, _) = super::session::session_tests::registered_context().await;
    crate::config::update_project_for_tenant(&crate::session::ui_store::current_tenant_scope(), &std::fs::canonicalize(temp.path()).unwrap(), serde_json::json!({
        "providers": {"catalog": {"api": "https://example.test/v1", "adapter": "openai-compatible", "models": {
            "known": {"id": "gpt-5", "status": "active", "reasoning": true, "reasoning_options": [{"type": "effort", "values": ["low", "high"]}]},
            "other": {"id": "gpt-4.1", "status": "active", "reasoning": false}
        }}}
    })).await.unwrap();
    ctx.default_provider = Arc::new("catalog".into());
    ctx.model = Arc::new("other".into());
    let mut msg = super::session::session_tests::channel_message(Some("topic"));
    msg.channel = "lark".into();
    (temp, ctx, msg)
}

async fn catalog_dispatch(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    channel: &Arc<CatalogChannel>,
) -> String {
    let mut scoped = ctx.clone();
    scoped.message_snapshot = Some(resolve_channel_message_snapshot(ctx, msg).await.unwrap());
    let target: Arc<dyn Channel> = channel.clone();
    assert!(handle_runtime_command_if_needed(&scoped, msg, Some(&target)).await);
    channel.0.lock().await.last().unwrap().content.clone()
}

#[tokio::test]
async fn channel_command_catalog_real_provider_model_and_effort_success() {
    let (_temp, ctx, mut msg) = catalog_context().await;
    let channel = Arc::new(CatalogChannel::default());
    msg.content = "/models".into();
    let listing = catalog_dispatch(&ctx, &msg, &channel).await;
    assert!(listing.contains("known"), "{listing}");
    msg.content = "/model known".into();
    assert!(catalog_dispatch(&ctx, &msg, &channel).await.contains("Model switched"));
    msg.content = "/effort high".into();
    assert!(catalog_dispatch(&ctx, &msg, &channel).await.contains("已切换"));
    let key = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap().scope.key();
    assert_eq!(get_route_selection(&ctx, &key).model, "known");
    assert_eq!(get_route_selection(&ctx, &key).reasoning_effort.as_deref(), Some("high"));
}

#[tokio::test]
async fn channel_command_catalog_invalid_inputs_preserve_route_and_history() {
    let (_temp, ctx, mut msg) = catalog_context().await;
    let channel = Arc::new(CatalogChannel::default());
    let key = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap().scope.key();
    append_sender_turn(&ctx, &key, ChatMessage::user("keep"));
    let before = get_route_selection(&ctx, &key);
    for command in [
        "/model absent",
        "/effort high",
        "/effort imaginary",
        "/new extra",
        "/approve",
        "/not-supported",
    ] {
        msg.content = command.into();
        catalog_dispatch(&ctx, &msg, &channel).await;
        assert_eq!(get_route_selection(&ctx, &key), before, "{command}");
        assert_eq!(ctx.conversation_histories.lock().unwrap()[&key].len(), 1);
    }
}

#[tokio::test]
async fn channel_command_catalog_model_selection_isolated_across_domains() {
    let (_temp, ctx, mut msg) = catalog_context().await;
    let channel = Arc::new(CatalogChannel::default());
    msg.content = "/model known".into();
    catalog_dispatch(&ctx, &msg, &channel).await;
    for dimension in 0..5 {
        let mut other = msg.clone();
        match dimension {
            0 => other.instance_id = "second".into(),
            1 => other.reply_target = "other-room".into(),
            2 => other.thread_ts = Some("other-topic".into()),
            3 => other.sender = "other-sender".into(),
            _ => other.channel = "qq".into(),
        }
        let key = resolve_channel_message_snapshot(&ctx, &other).await.unwrap().scope.key();
        assert_eq!(get_route_selection(&ctx, &key).model, "other");
    }
}

#[tokio::test]
async fn channel_command_catalog_status_is_read_only_and_scoped() {
    let (_temp, ctx, mut msg) = catalog_context().await;
    let channel = Arc::new(CatalogChannel::default());
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    sender_session_store().lock().unwrap().insert(snapshot.scope.clone(), "owned-session".into());
    crate::session::status::set("owned-session", crate::session::status::Info::Busy);
    msg.content = "/status".into();
    let text = catalog_dispatch(&ctx, &msg, &channel).await;
    for expected in [&snapshot.project.id, "owned-session", "other", "运行中", "default"] {
        assert!(text.contains(expected));
    }
    assert!(ctx.route_overrides.lock().unwrap().is_empty());
    msg.reply_target = "another-chat".into();
    assert!(!catalog_dispatch(&ctx, &msg, &channel).await.contains("owned-session"));
    sender_session_store().lock().unwrap().remove(&snapshot.scope);
    crate::session::status::set("owned-session", crate::session::status::Info::Idle);
}

#[tokio::test]
async fn channel_command_catalog_provider_catalog_isolated_by_tenant() {
    let (temp, ctx, mut msg) = catalog_context().await;
    let channel = Arc::new(CatalogChannel::default());
    msg.content = "/model known".into();
    assert!(catalog_dispatch(&ctx, &msg, &channel).await.contains("Model switched"));
    let tenant =
        vw_api_types::id::TenantScope::new("command-catalog", uuid::Uuid::new_v4().to_string())
            .unwrap();
    crate::session::ui_store::with_tenant_scope(tenant.clone(), async {
        let (project, _) = crate::project::from_directory(&tenant, temp.path()).await.unwrap();
        crate::config::update_project_for_tenant(&tenant, &std::fs::canonicalize(temp.path()).unwrap(), serde_json::json!({"providers": {"catalog": {"api": "https://example.test/v1", "models": {"tenant-model": {"status": "active"}}}}})).await.unwrap();
        let context = crate::project::instance::Context { directory: std::fs::canonicalize(temp.path()).unwrap().to_string_lossy().into(), worktree: project.worktree.clone(), project };
        crate::project::instance::provide_context(context, async {
            assert!(provider::get_model("catalog", "known").await.is_err());
            let loaded = crate::config::get_project_for_tenant(&tenant, &std::fs::canonicalize(temp.path()).unwrap()).await.unwrap();
            assert!(loaded.providers.get("catalog").and_then(|p| p.get("models")).and_then(|m| m.get("tenant-model")).is_some(), "isolated fixture configuration missing model");
            let result = provider::get_model("catalog", "tenant-model").await;
            assert!(result.is_ok(), "{result:?}");
        }).await;
    }).await;
}
