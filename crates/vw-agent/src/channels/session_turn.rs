//! 通道成功回复的 canonical 消息持久化，供恢复和历史锚点命令共用。
use super::*;
use crate::session::message;

pub(super) async fn persist_channel_turn(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    route: &ChannelRouteSelection,
    response: &str,
) -> Result<()> {
    let Some(snapshot) = &ctx.message_snapshot else { return Ok(()) };
    let id = sender_session_store()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&snapshot.scope)
        .cloned()
        .context("缺少当前通道会话")?;
    channel_session_ownership(snapshot, &id).await?;
    let session = crate::session::session::get_any(&id).await?;
    anyhow::ensure!(
        session.project_id == snapshot.scope.project
            && Path::new(&session.directory) == snapshot.directory,
        "会话项目已变化"
    );
    let now = crate::session::session::now_ms();
    let user_id = crate::id::ascending(crate::id::Prefix::Message, None)?;
    let assistant_id = crate::id::ascending(crate::id::Prefix::Message, None)?;
    let user = message::Info::User(Box::new(message::UserInfo {
        id: user_id.clone(),
        session_id: id.clone(),
        time: message::UserTime { created: now },
        summary: None,
        agent: msg.channel.clone(),
        model: message::ModelRef {
            provider_id: route.provider.clone(),
            model_id: route.model.clone(),
        },
        system: None,
        tools: None,
        variant: route.reasoning_effort.clone(),
    }));
    let assistant = message::Info::Assistant(Box::new(message::AssistantInfo {
        id: assistant_id.clone(),
        session_id: id.clone(),
        time: message::AssistantTime {
            created: now,
            completed: Some(now),
            processing_elapsed_ms: None,
            llm_elapsed_ms: None,
        },
        error: None,
        parent_id: user_id.clone(),
        model_id: route.model.clone(),
        provider_id: route.provider.clone(),
        mode: "chat".into(),
        agent: msg.channel.clone(),
        path: message::PathInfo {
            cwd: snapshot.directory.to_string_lossy().into(),
            root: snapshot.project.worktree.clone(),
        },
        summary: None,
        cost: 0.0,
        tokens: message::TokenInfo {
            total: None,
            input: 0,
            output: 0,
            reasoning: 0,
            cache: message::TokenCacheInfo { read: 0, write: 0 },
        },
        variant: route.reasoning_effort.clone(),
        finish: Some("stop".into()),
    }));
    for (info, text) in [(&user, msg.content.as_str()), (&assistant, response)] {
        message::update_message(info).await?;
        message::update_part(&message::Part::Text(message::TextPart {
            base: message::PartBase {
                id: crate::id::ascending(crate::id::Prefix::Part, None)?,
                session_id: id.clone(),
                message_id: info.id().into(),
            },
            text: text.into(),
            synthetic: None,
            ignored: None,
            time: Some(message::PartTime { start: now, end: Some(now) }),
            metadata: None,
        }))
        .await?;
    }
    Ok(())
}
