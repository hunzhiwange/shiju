//! Commit a completed channel turn before attempting external delivery.
use super::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::session::{message, ui_store};

#[derive(serde::Deserialize, serde::Serialize)]
struct ChannelReplyReceipt {
    session_id: String,
    assistant_id: String,
    retryable_reply: Option<String>,
}

fn receipt_key(msg: &traits::ChannelMessage) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(msg.id.as_bytes()))
}

/// A receipt may be staged before the history commit. Only a reply present in
/// the committed snapshot can suppress execution or be delivered again.
pub(super) async fn take_committed_channel_reply(
    snapshot: &ChannelMessageSnapshot,
    msg: &traits::ChannelMessage,
) -> Result<Option<(String, Option<String>)>> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (snapshot, msg);
        Ok(None)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if msg.id.is_empty() {
            return Ok(None);
        }
        let scope_key = super::session::ownership_scope_key(snapshot);
        let key = receipt_key(msg);
        let path = ["channel_reply", &scope_key, &key];
        let mut receipt: ChannelReplyReceipt = match crate::storage::read(&path).await {
            Ok(receipt) => receipt,
            Err(crate::storage::Error::NotFound(_)) => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        let id = receipt.session_id.clone();
        channel_session_ownership(snapshot, &id).await?;
        let tenant = snapshot.scope.chat.tenant.clone();
        let project = snapshot.scope.project.clone();
        let session_id = id.clone();
        let assistant_id = receipt.assistant_id.clone();
        let committed = crate::global_database::run_isolated(move || {
            Ok::<_, anyhow::Error>(
                ui_store::load_session_scoped_for_tenant_fallible(
                    &tenant,
                    &session_id,
                    Some(&project),
                )?
                .is_some_and(|chat| {
                    chat.message_ids.iter().flatten().any(|id| id == &assistant_id)
                }),
            )
        })
        .await??;
        if !committed {
            return Ok(None);
        }
        let reply = receipt.retryable_reply.take();
        if reply.is_some() {
            // Claim before sending. An interrupted/unknown delivery must not be resent.
            crate::storage::write_atomic(&path, &receipt).await?;
        }
        Ok(Some((id, reply)))
    }
}

pub(super) async fn mark_channel_reply_retryable(
    snapshot: &ChannelMessageSnapshot,
    session_id: &str,
    msg: &traits::ChannelMessage,
    reply: String,
) -> Result<()> {
    if msg.id.is_empty() {
        return Ok(());
    }
    let scope_key = super::session::ownership_scope_key(snapshot);
    let key = receipt_key(msg);
    let path = ["channel_reply", &scope_key, &key];
    let mut receipt: ChannelReplyReceipt = crate::storage::read(&path).await?;
    anyhow::ensure!(receipt.session_id == session_id, "channel reply receipt changed");
    receipt.retryable_reply = Some(reply);
    crate::storage::write_atomic(&path, &receipt).await?;
    Ok(())
}

pub(super) async fn persist_channel_turn(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    route: &ChannelRouteSelection,
    response: &str,
    processed: ChannelProcessorResponse,
) -> Result<Option<String>> {
    let Some(snapshot) = &ctx.message_snapshot else { return Ok(None) };
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (snapshot, msg, route, response, processed);
        anyhow::bail!("channel session persistence is unavailable on WASM");
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        commit_channel_turn(snapshot, msg, route, response, processed).await.map(Some)
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn commit_channel_turn(
    snapshot: &ChannelMessageSnapshot,
    msg: &traits::ChannelMessage,
    route: &ChannelRouteSelection,
    response: &str,
    processed: ChannelProcessorResponse,
) -> Result<String> {
    use crate::session::store::{FirstResponseCommit, LlmRawStepKey};
    use crate::session::ui_types::ChatSession;

    let id = sender_session_store()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&snapshot.scope)
        .cloned()
        .context("缺少当前通道会话")?;
    let _history = message::lock_history(&id).await;
    channel_session_ownership(snapshot, &id).await?;
    let session = crate::session::session::get_any(&id).await?;
    anyhow::ensure!(
        session.project_id == snapshot.scope.project
            && Path::new(&session.directory) == snapshot.directory
            && session.time.archived.is_none(),
        "会话项目已变化或会话已归档"
    );
    let pending = session.is_first_response_pending();
    let now = crate::session::session::now_ms();
    // Canonical history is ordered by ID. The processor preallocates the
    // assistant ID, so the ingress user ID must also be allocated before it.
    let user_id = processed.user_id.context("缺少通道用户消息 ID")?;
    let assistant_id = processed.assistant_id.context("缺少通道助手消息 ID")?;
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

    let part = |info: &message::Info, text: &str| -> Result<message::Part> {
        Ok(message::Part::Text(message::TextPart {
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
    };
    let parts = [part(&user, &msg.content)?, part(&assistant, response)?];
    let tenant = snapshot.scope.chat.tenant.clone();
    let project = snapshot.scope.project.clone();
    let preview = crate::session::title::first_response_source(&msg.content);
    let expected_steps = processed
        .steps
        .iter()
        .filter(|step| step.model.is_some())
        .map(|step| LlmRawStepKey::new(assistant_id.clone(), step.index))
        .collect::<Result<Vec<_>>>()?;
    let mut messages =
        to_session_history(&[ChatMessage::user(&msg.content), ChatMessage::assistant(response)]);
    messages[1].model = Some(channel_model_request_model(&route.provider, &route.model));

    if !msg.id.is_empty() {
        crate::storage::write_atomic(
            &["channel_reply", &super::session::ownership_scope_key(snapshot), &receipt_key(msg)],
            &ChannelReplyReceipt {
                session_id: id.clone(),
                assistant_id: assistant_id.clone(),
                retryable_reply: None,
            },
        )
        .await?;
    }

    let published = async {
        for (info, part) in [&user, &assistant].into_iter().zip(&parts) {
            // Keep native history silent until the corresponding snapshot is committed.
            crate::storage::write_atomic(&["message", &id, info.id()], info).await?;
            crate::storage::write_atomic(&["part", part.message_id(), part.id()], part).await?;
        }
        let preview = preview.clone();
        let tenant = tenant.clone();
        let id = id.clone();
        crate::global_database::run_isolated(move || {
            let scope = Some(project.as_str());
            let mut chat = ui_store::load_session_scoped_for_tenant_fallible(&tenant, &id, scope)?
                .unwrap_or_else(|| ChatSession {
                    id,
                    title: session.title,
                    messages: Vec::new(),
                    message_ids: Vec::new(),
                    calls: Vec::new(),
                    steps: Vec::new(),
                    created_ms: session.time.created,
                    updated_ms: now,
                });
            chat.message_ids.resize(chat.messages.len(), None);
            chat.message_ids.extend([Some(user_id), Some(assistant_id)]);
            chat.messages.extend(messages);
            chat.steps.extend(processed.steps);
            chat.updated_ms = chat.updated_ms.max(now);
            if pending {
                anyhow::ensure!(
                    ui_store::commit_first_response(
                        &tenant,
                        &project,
                        FirstResponseCommit {
                            snapshot: &chat,
                            raw_artifacts: &processed.raw_artifacts,
                            expected_finished_steps: &expected_steps,
                            preview: Some(&preview),
                            task_title: None,
                        },
                    )?,
                    "channel reservation changed before its first response was committed"
                );
            } else {
                ui_store::commit_session_ui_snapshot_scoped_for_tenant(
                    &tenant,
                    &chat,
                    scope,
                    &processed.raw_artifacts,
                    &expected_steps,
                )?;
            }
            Ok::<_, anyhow::Error>(())
        })
        .await?
    }
    .await;
    if let Err(error) = published {
        for part in &parts {
            let _ = crate::storage::remove(&["part", part.message_id(), part.id()]).await;
            let _ = crate::storage::remove(&["message", &id, part.message_id()]).await;
        }
        return Err(error);
    }
    if !pending {
        // The reply is already committed. Refresh recency without turning a
        // metadata refresh failure into a failed or duplicate turn.
        if crate::session::session::update_any(&id, |_| {}).await.is_err() {
            tracing::warn!(channel = %msg.channel, "Could not update channel session recency");
        }
    } else if !preview.is_empty() {
        crate::session::title::spawn_first_response_title(
            &tokio::runtime::Handle::current(),
            tenant,
            crate::project::instance::Context {
                directory: snapshot.directory.to_string_lossy().into(),
                worktree: snapshot.project.worktree.clone(),
                project: snapshot.project.clone(),
            },
            id.clone(),
            preview,
            Some(channel_model_request_model(&route.provider, &route.model)),
        );
    }
    Ok(id)
}
