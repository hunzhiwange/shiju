use super::*;
use runtime_command::project_commands::handle_project_command;
use session::session_tests::{channel_message, registered_context};

#[tokio::test]
async fn channel_project_select_success_starts_fresh_and_preserves_old_session() {
    let (_a, ctx, _) = registered_context().await;
    let (_b, _, b) = registered_context().await;
    let mut msg = channel_message(None);
    let old = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let old_session = resolve_or_create_sender_session_id(&old, &msg).await.unwrap();
    msg.content = format!("/project {}", b.id);
    assert!(handle_project_command(&ctx, &msg).await.contains("已切换"));
    let selected = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_eq!(selected.scope.project, b.id);
    assert_eq!(sender_session_store().lock().unwrap().get(&old.scope), Some(&old_session));
    assert!(!sender_session_store().lock().unwrap().contains_key(&selected.scope));
    assert!(crate::session::session::get_any(&old_session).await.is_ok());
}

#[tokio::test]
async fn channel_project_select_invalid_preserves_project_session_and_history() {
    let (_a, ctx, _) = registered_context().await;
    let mut msg = channel_message(None);
    let old = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let session = resolve_or_create_sender_session_id(&old, &msg).await.unwrap();
    append_sender_turn(&ctx, &old.scope.key(), ChatMessage::user("keep"));
    for invalid in ["missing-id", "/tmp", "模糊名称", "two ids"] {
        msg.content = format!("/project {invalid}");
        assert!(handle_project_command(&ctx, &msg).await.contains("失败"));
        assert_eq!(resolve_channel_message_snapshot(&ctx, &msg).await.unwrap().scope, old.scope);
        assert_eq!(sender_session_store().lock().unwrap().get(&old.scope), Some(&session));
        assert_eq!(ctx.conversation_histories.lock().unwrap()[&old.scope.key()][0].content, "keep");
    }
}

#[tokio::test]
async fn channel_project_select_default_recovers_invalid_override() {
    let (_a, ctx, a) = registered_context().await;
    let mut msg = channel_message(None);
    ctx.project_state
        .lock()
        .unwrap()
        .selections
        .insert(ChannelChatScope::from_message(&msg), "missing".into());
    msg.content = "/project default".into();
    assert!(handle_project_command(&ctx, &msg).await.contains("已切换"));
    assert_eq!(resolve_channel_message_snapshot(&ctx, &msg).await.unwrap().scope.project, a.id);
    assert!(ctx.project_state.lock().unwrap().selections.is_empty());
}

#[tokio::test]
async fn channel_project_select_isolates_chat_identity() {
    let (_a, ctx, a) = registered_context().await;
    let (_b, _, b) = registered_context().await;
    let mut msg = channel_message(None);
    msg.content = format!("/project {}", b.id);
    assert!(handle_project_command(&ctx, &msg).await.contains("已切换"));
    let mut others = vec![msg.clone(); 4];
    others[0].sender = "other".into();
    others[1].reply_target = "other".into();
    others[2].instance_id = "other".into();
    others[3].thread_ts = Some("other".into());
    for other in others {
        assert_eq!(
            resolve_channel_message_snapshot(&ctx, &other).await.unwrap().scope.project,
            a.id
        );
    }
}

#[tokio::test]
async fn channel_project_select_unavailable_directory_preserves_selection() {
    let (_a, ctx, a) = registered_context().await;
    let (b_dir, _, b) = registered_context().await;
    drop(b_dir);
    let mut msg = channel_message(None);
    msg.content = format!("/project {}", b.id);
    assert!(handle_project_command(&ctx, &msg).await.contains("失败"));
    assert_eq!(resolve_channel_message_snapshot(&ctx, &msg).await.unwrap().scope.project, a.id);
}

#[tokio::test]
async fn channel_project_select_lists_ids_and_current_source() {
    let (_a, ctx, a) = registered_context().await;
    let mut msg = channel_message(None);
    msg.content = "/projects".into();
    assert!(handle_project_command(&ctx, &msg).await.contains(&a.id));
    msg.content = "/project".into();
    let response = handle_project_command(&ctx, &msg).await;
    assert!(response.contains(&a.id), "{response}");
    assert!(response.contains("工作目录"));
    assert!(response.contains("/project default"));
}

#[tokio::test]
async fn channel_project_select_busy_rejects_and_does_not_queue() {
    let (_a, ctx, a) = registered_context().await;
    let (_b, _, b) = registered_context().await;
    let mut msg = channel_message(None);
    let gate = channel_chat_gate(&ctx, &msg);
    let guard = gate.read().await;
    msg.content = format!("/project {}", b.id);
    let response = handle_project_command(&ctx, &msg).await;
    assert!(response.contains("/stop"), "{response}");
    drop(guard);
    assert_eq!(resolve_channel_message_snapshot(&ctx, &msg).await.unwrap().scope.project, a.id);
}

#[tokio::test]
async fn channel_project_select_returning_to_project_does_not_resume_old_context() {
    let (_a, ctx, a) = registered_context().await;
    let (_b, _, b) = registered_context().await;
    let mut msg = channel_message(None);
    let old = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let session = resolve_or_create_sender_session_id(&old, &msg).await.unwrap();
    append_sender_turn(&ctx, &old.scope.key(), ChatMessage::user("old context"));
    for id in [&b.id, &a.id] {
        msg.content = format!("/project {id}");
        assert!(handle_project_command(&ctx, &msg).await.contains("已切换"));
    }
    assert!(!ctx.conversation_histories.lock().unwrap().contains_key(&old.scope.key()));
    assert_ne!(resolve_or_create_sender_session_id(&old, &msg).await.unwrap(), session);
    assert!(crate::session::session::get_any(&session).await.is_ok());
}

#[tokio::test]
async fn channel_project_select_other_tenant_cannot_select_project() {
    let (_a, ctx, a) = registered_context().await;
    let mut msg = channel_message(None);
    ctx.project_state.lock().unwrap().tenant =
        vw_api_types::id::TenantScope::new("select-isolation", "other").unwrap();
    msg.content = format!("/project {}", a.id);
    assert!(handle_project_command(&ctx, &msg).await.contains("失败"));
    assert!(ctx.project_state.lock().unwrap().selections.is_empty());
}

#[derive(Default)]
struct ProjectReplyChannel(Mutex<Vec<SendMessage>>);

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for ProjectReplyChannel {
    fn name(&self) -> &str {
        "telegram"
    }
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        self.0.lock().unwrap().push(message.clone());
        Ok(())
    }
    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn channel_project_select_message_entry_recovers_invalid_default() {
    let (_a, mut ctx, a) = registered_context().await;
    let channel = Arc::new(ProjectReplyChannel::default());
    ctx.channels_by_name =
        Arc::new(HashMap::from([("telegram".into(), channel.clone() as Arc<dyn Channel>)]));
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("telegram:default".into(), "missing".into());
    let mut msg = channel_message(Some("topic"));
    msg.content = format!("/project {}", a.id);
    manager::process_channel_message(Arc::new(ctx.clone()), msg.clone(), CancellationToken::new())
        .await;
    assert_eq!(resolve_channel_message_snapshot(&ctx, &msg).await.unwrap().scope.project, a.id);
    let sent = channel.0.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert!(sent[0].content.contains("已切换"));
    assert_eq!(sent[0].recipient, msg.reply_target);
    assert_eq!(sent[0].thread_ts, msg.thread_ts);
    assert!(ctx.conversation_histories.lock().unwrap().is_empty());
}

#[tokio::test]
async fn channel_project_select_message_entry_busy_does_not_execute() {
    let (_a, mut ctx, _) = registered_context().await;
    let channel = Arc::new(ProjectReplyChannel::default());
    ctx.channels_by_name =
        Arc::new(HashMap::from([("telegram".into(), channel.clone() as Arc<dyn Channel>)]));
    let msg = channel_message(None);
    let gate = channel_chat_gate(&ctx, &msg);
    let _guard = gate.write().await;
    manager::process_channel_message(Arc::new(ctx.clone()), msg, CancellationToken::new()).await;
    assert!(channel.0.lock().unwrap()[0].content.contains("/stop"));
    assert!(ctx.conversation_histories.lock().unwrap().is_empty());
}

#[test]
fn channel_project_select_commands_parse_on_all_platforms() {
    use runtime_command::{ChannelRuntimeCommand, parse_runtime_command};
    for platform in ["telegram", "discord", "qq", "lark", "dingtalk"] {
        assert_eq!(
            parse_runtime_command(platform, "/projects"),
            Some(ChannelRuntimeCommand::ListProjects)
        );
        assert_eq!(
            parse_runtime_command(platform, "/project@bot"),
            Some(ChannelRuntimeCommand::ShowProject)
        );
        assert_eq!(
            parse_runtime_command(platform, "/project default"),
            Some(ChannelRuntimeCommand::SelectProject("default".into()))
        );
    }
}

#[tokio::test]
async fn channel_project_select_invalid_default_preserves_override_and_session() {
    let (_a, ctx, a) = registered_context().await;
    let mut msg = channel_message(None);
    let chat = channel_chat_scope(&ctx, &msg);
    {
        let mut state = ctx.project_state.lock().unwrap();
        state.selections.insert(chat.clone(), a.id.clone());
        state
            .config
            .instance_default_projects
            .insert("telegram:default".into(), "missing-default".into());
    }
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let session = resolve_or_create_sender_session_id(&snapshot, &msg).await.unwrap();
    msg.content = "/project default".into();
    assert!(handle_project_command(&ctx, &msg).await.contains("失败"));
    assert_eq!(ctx.project_state.lock().unwrap().selections.get(&chat), Some(&a.id));
    assert_eq!(sender_session_store().lock().unwrap().get(&snapshot.scope), Some(&session));
}

#[tokio::test]
async fn channel_project_select_lists_chinese_names_and_instance_source() {
    let (_a, ctx, a) = registered_context().await;
    crate::project::update(
        &vw_api_types::id::ProjectScope::new(
            channel_chat_scope(&ctx, &channel_message(None)).tenant,
            vw_api_types::id::ProjectId::from(a.id.clone()),
        )
        .unwrap(),
        crate::project::UpdateInput {
            project_id: a.id.clone(),
            name: Some(Some("中文项目".into())),
            icon: None,
            commands: None,
        },
    )
    .await
    .unwrap();
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("telegram:default".into(), a.id);
    let mut msg = channel_message(None);
    msg.content = "/projects".into();
    assert!(handle_project_command(&ctx, &msg).await.contains("中文项目"));
    msg.content = "/project".into();
    assert!(handle_project_command(&ctx, &msg).await.contains("实例默认"));
}
