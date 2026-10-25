// 真实 manager、项目/会话存储和 processor；只替换模型流与平台外部发送。
async fn command_flow_send(
    ctx: &Arc<ChannelRuntimeContext>,
    channel: &Arc<ScopeRecordingChannel>,
    msg: &traits::ChannelMessage,
    content: &str,
) -> String {
    let mut inbound = msg.clone();
    inbound.content = content.into();
    if msg.channel == "qq" {
        let adapter = qq::QQChannel::new("fixture".into(), "fixture".into(), vec![msg.sender.clone()]);
        let payload = serde_json::json!({"op":0, "t":"GROUP_AT_MESSAGE_CREATE", "d":{
            "id":uuid::Uuid::new_v4().to_string(), "content":content,
            "group_openid":msg.reply_target.trim_start_matches("group:"),
            "author":{"member_openid":msg.sender}
        }});
        inbound = if msg.id == "webhook-flow" {
            adapter.parse_webhook_payload(&payload).await.into_iter().next().unwrap()
        } else {
            adapter.command_flow_websocket_frame(&payload).await.unwrap()
        };
    }
    #[cfg(feature = "channel-lark")]
    if msg.channel == "feishu" {
        let config = serde_json::from_value(serde_json::json!({"app_id":"fixture", "app_secret":"fixture", "allowed_users":[msg.sender]})).unwrap();
        let adapter = lark::LarkChannel::from_feishu_config(&config);
        let payload = serde_json::json!({"header":{"event_type":"im.message.receive_v1"},"event":{
            "sender":{"sender_id":{"open_id":msg.sender}},
            "message":{"message_type":"text", "chat_type":"group", "chat_id":msg.reply_target, "thread_id":msg.thread_ts, "content":serde_json::json!({"text":content}).to_string()}
        }});
        inbound = adapter.parse_event_payload_async(&payload).await.into_iter().next().unwrap();
        // 与 ConfiguredChannel 的实例标记一致，外部传输替身不更改解析结果。
        inbound.instance_id = msg.instance_id.clone();
        assert_eq!(inbound.sender, msg.sender);
        assert_eq!(inbound.thread_ts, msg.thread_ts);
    }
    let recipient = inbound.reply_target.clone();
    let thread = inbound.thread_ts.clone();
    let before = channel.0.lock().await.len();
    Box::pin(process_channel_message(ctx.clone(), inbound, CancellationToken::new())).await;
    let sent = channel.0.lock().await;
    assert!(sent.len() > before, "无回复：{content}");
    assert!(!sent[before..].iter().any(|m| m.content.contains("持久化失败")), "{content}: {:?}", &sent[before..]);
    assert!(sent[before..].iter().all(|m| m.recipient == recipient && m.thread_ts == thread));
    sent[before..].iter().map(|m| m.content.as_str()).collect::<Vec<_>>().join("\n")
}

async fn command_flow_fixture(platform: &str, instance: &str)
-> (tempfile::TempDir, Arc<ChannelRuntimeContext>, Arc<ScopeRecordingChannel>, traits::ChannelMessage) {
    let (temp, mut ctx, mut msg) = channel_command_catalog_tests::catalog_context().await;
    msg.channel = platform.into();
    msg.instance_id = instance.into();
    if platform == "qq" {
        msg.reply_target = "group:room-1".into();
        msg.thread_ts = None;
        msg.id = if instance == "webhook" { "webhook-flow" } else { "websocket-flow" }.into();
        msg.instance_id = "default".into();
    }
    let channel = Arc::new(ScopeRecordingChannel::default());
    ctx.channels_by_name = Arc::new(HashMap::from([(
        channel_instance_key(platform, &msg.instance_id), channel.clone() as Arc<dyn Channel>,
    )]));
    let project = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap().scope.project;
    ctx.project_state.lock().unwrap().config.instance_default_projects.insert(
        channel_instance_key(platform, &msg.instance_id), project,
    );
    (temp, Arc::new(ctx), channel, msg)
}

async fn command_flow_id(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> String {
    let snapshot = resolve_channel_message_snapshot(ctx, msg).await.unwrap();
    sender_session_store().lock().unwrap()[&snapshot.scope].clone()
}

fn command_flow_model_answer() {
    crate::session::processor::enqueue_test_llm_stream(vec![
        crate::session::llm::StreamEvent::Delta("flow answer".into()),
        crate::session::llm::StreamEvent::Done { finish_reason: Some("stop".into()), usage: Default::default() },
    ]);
}

#[allow(clippy::await_holding_lock)] // 现有模型流队列要求跨异步调用串行持有。
async fn command_flow_happy(platform: &str, instance: &str) {
    let _guard = crate::session::processor::test_llm_stream_guard();
    let (_temp, ctx, channel, msg) = command_flow_fixture(platform, instance).await;
    let help = command_flow_send(&ctx, &channel, &msg, "/help").await;
    let doc = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/vibewindow/通道命令.md")).unwrap();
    for meta in runtime_command::catalog::COMMANDS {
        assert!(help.contains(meta.usage));
        assert!(doc.replace("\\|", "|").contains(meta.usage));
    }
    assert!(command_flow_send(&ctx, &channel, &msg, "/help project").await.contains("default"));
    command_flow_model_answer();
    assert!(command_flow_send(&ctx, &channel, &msg, "first question").await.contains("flow answer"));
    let first = command_flow_id(&ctx, &msg).await;
    let captured = crate::session::processor::take_test_llm_messages(&first);
    assert_eq!(captured.len(), 1);
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_eq!(Path::new(&captured[0].directory), snapshot.directory);
    assert_eq!(captured[0].tenant, snapshot.scope.chat.tenant);
    assert!(command_flow_send(&ctx, &channel, &msg, "/new").await.contains("已创建"));
    let second = command_flow_id(&ctx, &msg).await;
    assert_ne!(first, second);
    assert!(command_flow_send(&ctx, &channel, &msg, "/sessions").await.contains(&first));
    assert!(command_flow_send(&ctx, &channel, &msg, &format!("/resume {first}")).await.contains(&first));
    assert_eq!(command_flow_id(&ctx, &msg).await, first);
    assert!(command_flow_send(&ctx, &channel, &msg, "/projects").await.contains(&snapshot.scope.project));
    assert!(command_flow_send(&ctx, &channel, &msg, "/model known").await.contains("Model switched"));
    assert!(command_flow_send(&ctx, &channel, &msg, "/effort high").await.contains("已切换"));
    let (_other_temp, _, other_project) = session::session_tests::registered_context().await;
    let other_project = other_project.id;
    assert!(command_flow_send(&ctx, &channel, &msg, &format!("/project {other_project}")).await.contains("已切换"));
    assert_eq!(resolve_channel_message_snapshot(&ctx, &msg).await.unwrap().scope.project, other_project);
    assert!(command_flow_send(&ctx, &channel, &msg, "/project default").await.contains("已切换"));
    command_flow_send(&ctx, &channel, &msg, &format!("/resume {first}")).await;
    command_flow_model_answer();
    assert!(command_flow_send(&ctx, &channel, &msg, "second question").await.contains("flow answer"));
    let second_requests = crate::session::processor::take_test_llm_messages(&first);
    assert_eq!(second_requests.len(), 1);
    let prior = serde_json::to_string(&second_requests[0].messages).unwrap();
    assert!(prior.contains("first question"), "恢复必须从真实存储加载历史：{prior}");
    let mut persisted = crate::session::message::messages(&first, None).await.unwrap();
    persisted.sort_by(|a, b| a.info.id().cmp(b.info.id()));
    let anchor = persisted.iter().rev().find(|m| matches!(m.info, crate::session::message::Info::User(_))).expect("普通聊天必须持久化用户消息，供历史命令使用真实锚点").info.id().to_string();
    assert!(command_flow_send(&ctx, &channel, &msg, &format!("/fork {anchor}")).await.contains("已分叉"));
    let fork = command_flow_id(&ctx, &msg).await;
    assert_ne!(fork, first);
    let mut fork_messages = crate::session::message::messages(&fork, None).await.unwrap();
    fork_messages.sort_by(|a, b| a.info.id().cmp(b.info.id()));
    let anchor = fork_messages.iter().rev().find(|m| matches!(m.info, crate::session::message::Info::User(_))).unwrap().info.id().to_string();
    let server = command_flow_summary_server(&ctx, &msg).await;
    assert!(command_flow_send(&ctx, &channel, &msg, &format!("/compact {anchor}")).await.contains("confirm"));
    let compacted = command_flow_send(&ctx, &channel, &msg, &format!("/compact confirm {anchor}")).await;
    assert!(compacted.contains("已压缩"), "{compacted}");
    assert_eq!(server.1.lock().unwrap().len(), 1);
    assert!(command_flow_send(&ctx, &channel, &msg, &format!("/reset {anchor}")).await.contains("confirm"));
    assert!(command_flow_send(&ctx, &channel, &msg, &format!("/reset confirm {anchor}")).await.contains("已回退"));
    command_flow_send(&ctx, &channel, &msg, "/stop").await;
    assert!(crate::session::processor::take_test_llm_messages(&first).is_empty());
    assert!(crate::session::processor::take_test_llm_messages(&fork).is_empty());
    command_flow_model_answer();
    assert!(command_flow_send(&ctx, &channel, &msg, "next question").await.contains("flow answer"));
    assert_eq!(crate::session::processor::take_test_llm_messages(&fork).len(), 1);
    assert_eq!(crate::session::session::get_any(&fork).await.unwrap().project_id, snapshot.scope.project);
}

#[tokio::test]
async fn channel_command_flow_feishu_named_session_roundtrip() {
    Box::pin(command_flow_happy("feishu", "engineering")).await;
}
#[tokio::test]
async fn channel_command_flow_qq_session_roundtrip() {
    Box::pin(command_flow_happy("qq", "default")).await;
}

#[tokio::test]
async fn channel_command_flow_errors_preserve_project_session_and_history() {
    let (_temp, ctx, channel, msg) = command_flow_fixture("qq", "default").await;
    command_flow_send(&ctx, &channel, &msg, "/new").await;
    let before = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let id = command_flow_id(&ctx, &msg).await;
    append_sender_turn(&ctx, &before.scope.key(), ChatMessage::user("keep"));
    let route = get_route_selection(&ctx, &before.scope.key());
    for content in ["/project missing", "/resume missing", "/model missing", "/effort imaginary", "/fork missing", "/reset confirm missing", "/new extra"] {
        command_flow_send(&ctx, &channel, &msg, content).await;
        assert_eq!(command_flow_id(&ctx, &msg).await, id);
        assert_eq!(resolve_channel_message_snapshot(&ctx, &msg).await.unwrap().scope, before.scope);
        assert_eq!(get_route_selection(&ctx, &before.scope.key()), route);
        assert_eq!(ctx.conversation_histories.lock().unwrap()[&before.scope.key()][0].content, "keep");
    }
    assert!(crate::session::processor::take_test_llm_messages(&id).is_empty());
}

#[tokio::test]
async fn channel_command_flow_concurrent_instances_rooms_threads_and_tenant_reject() {
    let (_temp, ctx, channel, msg) = command_flow_fixture("feishu", "engineering").await;
    command_flow_send(&ctx, &channel, &msg, "/new").await;
    let id = command_flow_id(&ctx, &msg).await;
    for dimension in 0..3 {
        let mut other = msg.clone();
        match dimension {
            0 => other.instance_id = "support".into(),
            1 => other.reply_target = "another-group".into(),
            _ => other.thread_ts = Some("another-topic".into()),
        }
        let mut other_ctx = ctx.as_ref().clone();
        other_ctx.channels_by_name = Arc::new(HashMap::from([(
            channel_instance_key("feishu", &other.instance_id), channel.clone() as Arc<dyn Channel>,
        )]));
        let other_ctx = Arc::new(other_ctx);
        let ((), ()) = tokio::join!(
            process_channel_message(ctx.clone(), { let mut m=msg.clone(); m.content="/status".into(); m }, CancellationToken::new()),
            process_channel_message(other_ctx.clone(), { let mut m=other.clone(); m.content=format!("/resume {id}"); m }, CancellationToken::new())
        );
        let scope = resolve_channel_message_snapshot(&other_ctx, &other).await.unwrap().scope;
        assert!(!sender_session_store().lock().unwrap().contains_key(&scope));
        assert_eq!(command_flow_id(&ctx, &msg).await, id);
    }
    let tenant = vw_api_types::id::TenantScope::new("flow", uuid::Uuid::new_v4().to_string()).unwrap();
    let mut foreign = ctx.as_ref().clone();
    foreign.project_state = Arc::new(Mutex::new(Default::default()));
    foreign.project_state.lock().unwrap().tenant = tenant;
    let foreign = Arc::new(foreign);
    assert!(command_flow_send(&foreign, &channel, &msg, &format!("/resume {id}")).await.contains("失败"));
    assert_eq!(command_flow_id(&ctx, &msg).await, id);
}

struct FlowSummaryServer(tokio::task::JoinHandle<()>, Arc<Mutex<Vec<serde_json::Value>>>, String);
impl Drop for FlowSummaryServer {
    fn drop(&mut self) { self.0.abort(); crate::env::remove(&self.2); }
}
async fn command_flow_summary_server(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> FlowSummaryServer {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let app = axum::Router::new().route("/v1/chat/completions", axum::routing::post(move |axum::Json(body): axum::Json<serde_json::Value>| {
        captured.lock().unwrap().push(body);
        async {
            let delta = serde_json::json!({"id":"flow", "object":"chat.completion.chunk", "created":1, "model":"known", "choices":[{"index":0,"delta":{"content":"flow summary"},"finish_reason":null}]});
            let done = serde_json::json!({"id":"flow", "object":"chat.completion.chunk", "created":1, "model":"known", "choices":[{"index":0,"delta":{},"finish_reason":"stop"}]});
            ([("content-type", "text/event-stream")], format!("data: {delta}\n\ndata: {done}\n\ndata: [DONE]\n\n"))
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
    let snapshot = resolve_channel_message_snapshot(ctx, msg).await.unwrap();
    let key = format!("VW_FLOW_FIXTURE_{}", snapshot.scope.project);
    crate::env::set(&key, "fixture-only");
    crate::config::update_project_for_tenant(&snapshot.scope.chat.tenant, &snapshot.directory, serde_json::json!({
        "providers": {"catalog": {"api": format!("http://{address}/v1"), "env":[key.clone()]}}
    })).await.unwrap();
    FlowSummaryServer(task, requests, key)
}

#[tokio::test]
async fn channel_command_flow_qq_webhook_session_roundtrip() {
    Box::pin(command_flow_happy("qq", "webhook")).await;
}

#[cfg(feature = "channel-lark")]
#[tokio::test]
async fn channel_command_flow_feishu_parser_preserves_actor_and_topic() {
    let config = serde_json::from_value(serde_json::json!({"app_id":"fixture", "app_secret":"fixture", "allowed_users":["actor"]})).unwrap();
    let adapter = lark::LarkChannel::from_feishu_config(&config);
    let payload = serde_json::json!({"header":{"event_type":"im.message.receive_v1"},"event":{
        "sender":{"sender_id":{"open_id":"actor"}},
        "message":{"message_type":"text", "chat_type":"group", "chat_id":"room", "thread_id":"topic", "content":"{\"text\":\"/help\"}"}
    }});
    let messages = adapter.parse_event_payload_async(&payload).await;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].sender, "actor");
    assert_eq!(messages[0].reply_target, "room");
    assert_eq!(messages[0].thread_ts.as_deref(), Some("topic"));
}
