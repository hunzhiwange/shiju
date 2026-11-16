//! History assertions driven by the real inbound host and real message storage.
use channel_support::runtime_state::ChannelRuntimeContext;
use channel_types as traits;
use model_types::ChatMessage;
use session_engine::message;
use channel_support::session::channel_session_ownership;
use std::{future::Future, pin::Pin};
pub type Dispatch = for<'a> fn(&'a ChannelRuntimeContext, &'a traits::ChannelMessage, &'a str) -> Pin<Box<dyn Future<Output = String> + Send + 'a>>;
fn lifecycle_id(ctx: &ChannelRuntimeContext) -> String {
    channel_support::session::sender_session_store().lock().unwrap().get(&ctx.message_snapshot.as_ref().unwrap().scope).cloned().expect("bound session")
}
fn channel_chat_gate(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> std::sync::Arc<tokio::sync::RwLock<()>> {
    channel_support::scope::channel_chat_gate(&ctx.project_state, msg)
}
#[allow(unused_variables)]
pub async fn channel_session_history_lists_real_anchors_without_mutation(ctx: ChannelRuntimeContext, msg: traits::ChannelMessage, id: String, ids: Vec<String>, lifecycle_dispatch: Dispatch) {
    for name in ["fork", "compact", "reset"] {
        let response = lifecycle_dispatch(&ctx, &msg, &format!("/{name}")).await;
        assert!(response.contains(&format!("/{name} {}", ids[1])), "{response}");
        assert_eq!(lifecycle_id(&ctx), id);
    }

}

#[allow(unused_variables)]
pub async fn channel_session_history_fork_switches_and_preserves_source(ctx: ChannelRuntimeContext, msg: traits::ChannelMessage, id: String, ids: Vec<String>, lifecycle_dispatch: Dispatch) {
    let response = lifecycle_dispatch(&ctx, &msg, &format!("/fork {}", ids[1])).await;
    assert!(response.contains("已分叉"), "{response}");
    let next = lifecycle_id(&ctx);
    assert_ne!(id, next);
    let snapshot = ctx.message_snapshot.as_ref().unwrap();
    channel_session_ownership(snapshot, &next).await.unwrap();
    assert_eq!(message::messages(&id, None).await.unwrap().len(), 3);
    assert_eq!(message::messages(&next, None).await.unwrap().len(), 2);
    assert_eq!(ctx.conversation_histories.lock().unwrap()[&snapshot.scope.key()].len(), 2);

}

#[allow(unused_variables)]
pub async fn channel_session_history_reset_requires_confirmation_and_reloads_cache(ctx: ChannelRuntimeContext, msg: traits::ChannelMessage, id: String, ids: Vec<String>, lifecycle_dispatch: Dispatch) {
    let confirm = format!("/reset confirm {}", ids[1]);
    assert!(lifecycle_dispatch(&ctx, &msg, &confirm).await.contains("失败"));
    let prompt = lifecycle_dispatch(&ctx, &msg, &format!("/rewind {}", ids[1])).await;
    assert!(prompt.contains(&confirm), "{prompt}");
    assert_eq!(message::messages(&id, None).await.unwrap().len(), 3);
    let result = lifecycle_dispatch(&ctx, &msg, &confirm).await;
    assert!(result.contains("已回退"), "{result}");
    assert_eq!(lifecycle_id(&ctx), id);
    assert_eq!(message::messages(&id, None).await.unwrap().len(), 2);
    assert_eq!(
        ctx.conversation_histories.lock().unwrap()
            [&ctx.message_snapshot.as_ref().unwrap().scope.key()]
            .len(),
        2
    );
    assert!(lifecycle_dispatch(&ctx, &msg, &confirm).await.contains("失败"));

}

#[allow(unused_variables)]
pub async fn channel_session_history_stale_confirmation_is_rejected(ctx: ChannelRuntimeContext, msg: traits::ChannelMessage, id: String, ids: Vec<String>, lifecycle_dispatch: Dispatch) {
    for command in ["reset", "compact"] {
        let prompt = lifecycle_dispatch(&ctx, &msg, &format!("/{command} {}", ids[1])).await;
        assert!(prompt.contains("confirm"), "{prompt}");
        seed(&ctx, &id).await;
        let response =
            lifecycle_dispatch(&ctx, &msg, &format!("/{command} confirm {}", ids[1])).await;
        assert!(response.contains("失败") && response.contains("变化"), "{response}");
        assert_eq!(lifecycle_id(&ctx), id);
    }

}

#[allow(unused_variables)]
pub async fn channel_session_history_cross_scope_and_busy_are_rejected(ctx: ChannelRuntimeContext, msg: traits::ChannelMessage, id: String, ids: Vec<String>, lifecycle_dispatch: Dispatch) {
    assert!(
        lifecycle_dispatch(&ctx, &msg, &format!("/reset {}", ids[1])).await.contains("confirm")
    );
    for dimension in 0..5 {
        let mut other = msg.clone();
        match dimension {
            0 => other.channel = "qq".into(),
            1 => other.instance_id = "other".into(),
            2 => other.reply_target = "other".into(),
            3 => other.thread_ts = Some("other".into()),
            _ => other.sender = "other".into(),
        }
        let mut candidate = ctx.clone();
        candidate.message_snapshot = None;
        let result =
            lifecycle_dispatch(&candidate, &other, &format!("/reset confirm {}", ids[1])).await;
        assert!(result.contains("失败"), "{result}");
    }
    let gate = channel_chat_gate(&ctx, &msg);
    let guard = gate.read().await;
    assert!(lifecycle_dispatch(&ctx, &msg, &format!("/fork {}", ids[1])).await.contains("繁忙"));
    drop(guard);
    assert_eq!(lifecycle_id(&ctx), id);
    assert_eq!(message::messages(&id, None).await.unwrap().len(), 3);

}

#[allow(unused_variables)]
pub async fn channel_session_history_switching_sessions_invalidates_confirmation(ctx: ChannelRuntimeContext, msg: traits::ChannelMessage, id: String, ids: Vec<String>, lifecycle_dispatch: Dispatch) {
    lifecycle_dispatch(&ctx, &msg, &format!("/reset {}", ids[1])).await;
    lifecycle_dispatch(&ctx, &msg, "/new").await;
    assert!(lifecycle_dispatch(&ctx, &msg, &format!("/resume {id}")).await.contains("已恢复"));
    assert!(
        lifecycle_dispatch(&ctx, &msg, &format!("/reset confirm {}", ids[1]))
            .await
            .contains("失败")
    );
    assert_eq!(message::messages(&id, None).await.unwrap().len(), 3);

}

#[allow(unused_variables)]
pub async fn channel_session_history_part_changes_invalidate_confirmation(ctx: ChannelRuntimeContext, msg: traits::ChannelMessage, id: String, ids: Vec<String>, lifecycle_dispatch: Dispatch) {
    lifecycle_dispatch(&ctx, &msg, &format!("/reset {}", ids[1])).await;
    let mut parts = message::parts(&id, &ids[0]).await.unwrap();
    if let message::Part::Text(text) = &mut parts[0] {
        text.text = "edited in place".into();
    }
    message::update_part(&parts[0]).await.unwrap();
    let response = lifecycle_dispatch(&ctx, &msg, &format!("/reset confirm {}", ids[1])).await;
    assert!(response.contains("失败") && response.contains("变化"), "{response}");
    assert_eq!(message::messages(&id, None).await.unwrap().len(), 3);

}

pub async fn seed(ctx: &ChannelRuntimeContext, id: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for content in ["first turn", "second turn", "last turn"] {
        let mid = ordered_id::ascending(ordered_id::Prefix::Message, None).unwrap();
        let info = message::Info::User(Box::new(message::UserInfo {
            id: mid.clone(),
            session_id: id.into(),
            time: message::UserTime { created: 1 },
            summary: None,
            agent: "build".into(),
            model: message::ModelRef {
                provider_id: "default-provider".into(),
                model_id: "default-model".into(),
            },
            system: None,
            tools: None,
            variant: None,
        }));
        let tenant = ctx.message_snapshot.as_ref().unwrap().scope.chat.tenant.clone();
        session_store::ui_store::with_tenant_scope(tenant, async {
            message::update_message(&info).await.unwrap();
            message::update_part(&message::Part::Text(message::TextPart {
                base: message::PartBase {
                    id: ordered_id::ascending(ordered_id::Prefix::Part, None).unwrap(),
                    session_id: id.into(),
                    message_id: mid.clone(),
                },
                text: content.into(),
                synthetic: None,
                ignored: None,
                time: None,
                metadata: None,
            }))
            .await
            .unwrap();
        })
        .await;
        ids.push(mid);
    }
    ids
}
