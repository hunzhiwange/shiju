//! Catalog assertions with real host dispatch and shared route/session storage.
use channel_support::{runtime_state::ChannelRuntimeContext, routing::get_route_selection, session::sender_session_store};
use channel_types as traits;
use model_types::ChatMessage;
use std::{future::Future, pin::Pin};
pub type Dispatch = for<'a> fn(&'a ChannelRuntimeContext, &'a traits::ChannelMessage) -> Pin<Box<dyn Future<Output = String> + Send + 'a>>;
async fn resolve_channel_message_snapshot(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> anyhow::Result<channel_support::scope::ChannelMessageSnapshot> {
    channel_support::scope::resolve_channel_message_snapshot(&ctx.project_state, ctx.message_snapshot.as_ref(), &ctx.workspace_dir, msg).await
}
fn append_sender_turn(ctx: &ChannelRuntimeContext, key: &str, message: ChatMessage) {
    channel_support::history::append_sender_turn(&ctx.conversation_histories, key, message)
}
pub async fn channel_command_catalog_real_provider_model_and_effort_success(ctx: ChannelRuntimeContext, mut msg: traits::ChannelMessage, catalog_dispatch: Dispatch) {
    msg.content = "/models".into();
    let listing = catalog_dispatch(&ctx, &msg).await;
    assert!(listing.contains("known"), "{listing}");
    msg.content = "/model known".into();
    assert!(catalog_dispatch(&ctx, &msg).await.contains("Model switched"));
    msg.content = "/effort high".into();
    assert!(catalog_dispatch(&ctx, &msg).await.contains("已切换"));
    let key = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap().scope.key();
    assert_eq!(get_route_selection(&ctx, &key).model, "known");
    assert_eq!(get_route_selection(&ctx, &key).reasoning_effort.as_deref(), Some("high"));

}

pub async fn channel_command_catalog_invalid_inputs_preserve_route_and_history(ctx: ChannelRuntimeContext, mut msg: traits::ChannelMessage, catalog_dispatch: Dispatch) {
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
        catalog_dispatch(&ctx, &msg).await;
        assert_eq!(get_route_selection(&ctx, &key), before, "{command}");
        assert_eq!(ctx.conversation_histories.lock().unwrap()[&key].len(), 1);
    }

}

pub async fn channel_command_catalog_model_selection_isolated_across_domains(ctx: ChannelRuntimeContext, mut msg: traits::ChannelMessage, catalog_dispatch: Dispatch) {
    msg.content = "/model known".into();
    catalog_dispatch(&ctx, &msg).await;
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

pub async fn channel_command_catalog_status_is_read_only_and_scoped(ctx: ChannelRuntimeContext, mut msg: traits::ChannelMessage, catalog_dispatch: Dispatch) {
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    sender_session_store().lock().unwrap().insert(snapshot.scope.clone(), "owned-session".into());
    session_engine::status::set("owned-session", session_engine::status::Info::Busy);
    msg.content = "/status".into();
    let text = catalog_dispatch(&ctx, &msg).await;
    for expected in [&snapshot.project.id, "owned-session", "other", "运行中", "default"] {
        assert!(text.contains(expected));
    }
    assert!(ctx.route_overrides.lock().unwrap().is_empty());
    msg.reply_target = "another-chat".into();
    assert!(!catalog_dispatch(&ctx, &msg).await.contains("owned-session"));
    sender_session_store().lock().unwrap().remove(&snapshot.scope);
    session_engine::status::set("owned-session", session_engine::status::Info::Idle);

}
