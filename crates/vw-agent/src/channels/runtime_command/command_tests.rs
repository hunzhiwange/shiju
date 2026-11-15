//! 使用真实宿主的命令目录集成测试。
use super::*;

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
        vw_api_transport::TenantScope::new("command-catalog", uuid::Uuid::new_v4().to_string())
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
