#[derive(Default)]
struct ScopeRecordingChannel(tokio::sync::Mutex<Vec<SendMessage>>);

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Channel for ScopeRecordingChannel {
    fn name(&self) -> &str {
        "telegram"
    }
    async fn send(&self, message: &SendMessage) -> Result<()> {
        self.0.lock().await.push(message.clone());
        Ok(())
    }
    async fn listen(&self, _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn channel_project_scope_instance_reply_and_route_stay_with_sender() {
    let (_temp, mut ctx, _) = session::session_tests::registered_context().await;
    let first = Arc::new(ScopeRecordingChannel::default());
    let second = Arc::new(ScopeRecordingChannel::default());
    ctx.channels_by_name = Arc::new(HashMap::from([
        (channel_instance_key("telegram", "default"), first.clone() as Arc<dyn Channel>),
        (channel_instance_key("telegram", "second"), second.clone() as Arc<dyn Channel>),
    ]));
    let mut msg = session::session_tests::channel_message(Some("thread"));
    msg.instance_id = "second".into();
    msg.content = "/model selected-model".into();
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let mut first_msg = msg.clone();
    first_msg.instance_id = "default".into();
    let first_scope = resolve_channel_message_snapshot(&ctx, &first_msg).await.unwrap();
    let ctx = Arc::new(ctx);
    process_channel_message(ctx.clone(), msg.clone(), CancellationToken::new()).await;
    assert!(first.0.lock().await.is_empty());
    let sent = second.0.lock().await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].recipient, msg.reply_target);
    assert_eq!(sent[0].thread_ts, msg.thread_ts);
    assert!(sent[0].content.contains("selected-model"));
    assert_eq!(get_route_selection(&ctx, &snapshot.scope.key()).model, "selected-model");
    assert_ne!(get_route_selection(&ctx, &first_scope.scope.key()).model, "selected-model");
}
