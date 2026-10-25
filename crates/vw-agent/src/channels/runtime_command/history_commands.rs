//! 真实消息锚点驱动的历史操作；调用方持有聊天写锁和租户/项目上下文。
use super::super::*;
use crate::session::{compaction, message, session};

#[derive(Clone)]
struct Confirmation {
    command: &'static str,
    session: String,
    anchor: String,
    version: String,
}

fn confirmations() -> &'static Mutex<HashMap<ChannelMessageScope, Confirmation>> {
    static STORE: OnceLock<Mutex<HashMap<ChannelMessageScope, Confirmation>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn invalidate_history(snapshot: &ChannelMessageSnapshot) {
    confirmations().lock().unwrap_or_else(|e| e.into_inner()).remove(&snapshot.scope);
}

pub(super) async fn handle_history_command(
    ctx: &ChannelRuntimeContext,
    command: ChannelRuntimeCommand,
) -> Result<String> {
    let snapshot = ctx.message_snapshot.as_ref().context("缺少消息作用域")?;
    let (name, args) = match command {
        ChannelRuntimeCommand::ForkSession(args) => ("fork", args),
        ChannelRuntimeCommand::CompactSession(args) => ("compact", args),
        ChannelRuntimeCommand::ResetSession(args) => ("reset", args),
        _ => anyhow::bail!("未知历史命令"),
    };
    let id = sender_session_store()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&snapshot.scope)
        .cloned()
        .context("尚未创建会话，请先使用 /new")?;
    channel_session_ownership(snapshot, &id).await?;
    let info = session::get(&id).await?;
    anyhow::ensure!(
        info.project_id == snapshot.scope.project
            && Path::new(&info.directory) == snapshot.directory
            && info.time.archived.is_none(),
        "会话项目不匹配或已归档"
    );
    anyhow::ensure!(
        matches!(crate::session::status::get(&id), crate::session::status::Info::Idle),
        "当前会话繁忙"
    );
    let mut messages = message::messages(&id, None).await?;
    messages.sort_by(|a, b| a.info.id().cmp(b.info.id()));
    if args.is_empty() {
        let mut response = String::from("当前会话真实消息锚点（使用完整命令）：\n");
        for msg in messages
            .iter()
            .rev()
            .filter(|msg| name != "compact" || matches!(msg.info, message::Info::User(_)))
            .take(20)
        {
            response.push_str(&format!("/{name} {}\n", msg.info.id()));
        }
        if messages.is_empty() {
            response.push_str("暂无持久化消息锚点。\n");
        }
        return Ok(response);
    }
    let tokens: Vec<_> = args.split_whitespace().collect();
    let (confirm, anchor) = match tokens.as_slice() {
        [anchor] => (false, *anchor),
        ["confirm", anchor] if name != "fork" => (true, *anchor),
        _ => anyhow::bail!("参数错误，请使用 /{name} 查看锚点"),
    };
    let boundary = messages
        .iter()
        .position(|msg| msg.info.id() == anchor)
        .context("消息锚点不属于当前会话")?;
    if name == "compact" {
        anyhow::ensure!(
            matches!(messages[boundary].info, message::Info::User(_)),
            "压缩锚点必须是用户消息"
        );
    }
    // 包含所有元数据与消息片段，避免仅靠毫秒更新时间遗漏同一毫秒内的变更。
    use sha2::{Digest, Sha256};
    let mut snapshot_value = serde_json::to_value((&info, &messages))?;
    snapshot_value.sort_all_objects();
    let version = hex::encode(Sha256::digest(serde_json::to_vec(&snapshot_value)?));
    if name != "fork" {
        if !confirm {
            confirmations().lock().unwrap_or_else(|e| e.into_inner()).insert(
                snapshot.scope.clone(),
                Confirmation { command: name, session: id, anchor: anchor.into(), version },
            );
            let effect = if name == "reset" {
                "历史和生成的文件变更回退到锚点"
            } else {
                "压缩锚点之前的历史"
            };
            return Ok(format!("将{effect}。确认请输入 /{name} confirm {anchor}"));
        }
        let pending = confirmations()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&snapshot.scope)
            .context("请先请求历史操作确认")?;
        anyhow::ensure!(
            pending.command == name
                && pending.session == id
                && pending.anchor == anchor
                && pending.version == version,
            "会话快照已变化或确认不匹配，请重新确认"
        );
    }
    let key = snapshot.scope.key();
    let (target, history) = match name {
        "fork" => {
            let route = get_route_selection(ctx, &key);
            super::session_lifecycle::persist_current_route(ctx, &route).await?;
            let fork = session::fork(&id, Some(anchor)).await?;
            let result = async {
                let history = message::messages(&fork.id, None).await?;
                let record =
                    ChannelSessionOwnership { scope: snapshot.scope.key(), route: Some(route) };
                crate::storage::write(
                    &[
                        "channel_session",
                        &super::super::session::ownership_scope_key(snapshot),
                        &fork.id,
                    ],
                    &record,
                )
                .await?;
                Ok::<_, anyhow::Error>(history)
            }
            .await;
            match result {
                Ok(history) => (fork.id, history),
                Err(err) => {
                    let _ = session::remove_any(&fork.id).await;
                    return Err(err);
                }
            }
        }
        "reset" => {
            let reverted = crate::session::revert::revert(crate::session::revert::RevertInput {
                session_id: id.clone(),
                message_id: anchor.into(),
                part_id: None,
            })
            .await?;
            crate::session::revert::cleanup(&reverted).await?;
            (id.clone(), message::messages(&id, None).await?)
        }
        "compact" => match Box::pin(compaction::process_session(&id, anchor, false)).await? {
            compaction::CompactionOutcome::Compacted { history, .. } => (id.clone(), history),
            compaction::CompactionOutcome::NotNeeded { .. } => {
                return Ok("无需压缩，历史保持不变。".into());
            }
            compaction::CompactionOutcome::Failed { error, .. } => {
                anyhow::bail!("压缩失败：{error}")
            }
        },
        _ => unreachable!(),
    };
    let history = cached_history(history);
    // 所有持久化操作成功后再同时更新缓存和绑定，中间不再 await。
    ctx.conversation_histories.lock().unwrap_or_else(|e| e.into_inner()).insert(key, history);
    sender_session_store()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(snapshot.scope.clone(), target.clone());
    confirmations().lock().unwrap_or_else(|e| e.into_inner()).remove(&snapshot.scope);
    super::session_lifecycle::invalidate_archive(snapshot);
    let action = match name {
        "fork" => "已分叉并切换到",
        "compact" => "已压缩",
        _ => "已回退",
    };
    Ok(format!("{action}会话 {target}。"))
}

fn cached_history(mut messages: Vec<message::WithParts>) -> Vec<ChatMessage> {
    messages.sort_by(|a, b| a.info.id().cmp(b.info.id()));
    messages
        .into_iter()
        .map(|msg| {
            let role = match msg.info {
                message::Info::User(_) => "user",
                message::Info::Assistant(_) => "assistant",
            };
            let content = msg
                .parts
                .iter()
                .filter_map(|part| match part {
                    message::Part::Text(text) if text.ignored != Some(true) => {
                        Some(text.text.clone())
                    }
                    message::Part::Tool(_) | message::Part::File(_) => {
                        serde_json::to_string(part).ok()
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            ChatMessage { role: role.into(), content }
        })
        .collect()
}
