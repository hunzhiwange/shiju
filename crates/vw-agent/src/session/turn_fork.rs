//! Prepare a fork from a consistent snapshot of native messages.

use std::collections::HashMap;
use std::fmt::Write;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use super::{CreateInput, Error, Info, event, forked_title};
use crate::project::instance;
use crate::session::{agent_binding, compaction_checkpoint, message};
use crate::{bus, id, storage};
use serde_json::{Value, json};
use vw_shared::session::ui_types::{ChatMessage, ChatRole, ChatSession};

const FORK_SLUG_PREFIX: &str = "turn-fork-";

pub(crate) async fn forked_session(session_id: &str) -> Result<Option<Info>, Error> {
    if session_id.trim().is_empty() {
        return Ok(None);
    }
    let tenant = crate::session::ui_store::current_tenant_scope();
    let session_id = session_id.to_owned();
    let info = crate::global_database::run_isolated(move || {
        crate::session::ui_store::load_agent_session_any_for_tenant_fallible(&tenant, &session_id)
    })
    .await??;
    Ok(info.filter(|info| info.slug.starts_with(FORK_SLUG_PREFIX)))
}

pub(crate) async fn load_fork_history(
    info: &Info,
) -> Result<(Vec<ChatMessage>, Vec<Value>), Error> {
    ensure_directory(info, &instance::directory()).await?;
    if super::project_id()? != info.project_id {
        return Err(anyhow::anyhow!("fork belongs to another project").into());
    }
    let _history = message::lock_history(&info.id).await;
    let mut messages = message::fork_snapshot(&info.id).await?;
    // Streams preallocate the assistant ID first. Order by user turn, then
    // by message ID within that turn, with the user always before its response.
    let users = messages
        .iter()
        .filter_map(|msg| match &msg.info {
            message::Info::User(user) => Some((user.id.clone(), user.time.created)),
            message::Info::Assistant(_) => None,
        })
        .collect::<HashMap<_, _>>();
    messages.sort_by(|left, right| {
        let key = |msg: &message::WithParts| match &msg.info {
            message::Info::User(user) => {
                (user.time.created, user.id.clone(), false, user.id.clone())
            }
            message::Info::Assistant(assistant) => (
                users.get(&assistant.parent_id).copied().unwrap_or_default(),
                assistant.parent_id.clone(),
                true,
                assistant.id.clone(),
            ),
        };
        key(left).cmp(&key(right))
    });
    Ok((messages.iter().map(|msg| project_message(msg, false)).collect(), model_history(&messages)))
}

async fn ensure_directory(info: &Info, directory: &str) -> Result<(), Error> {
    if info.directory == directory {
        return Ok(());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let expected = tokio::fs::canonicalize(Path::new(&info.directory)).await.ok();
        let actual = tokio::fs::canonicalize(Path::new(directory)).await.ok();
        if expected.is_some() && expected == actual {
            return Ok(());
        }
    }
    Err(anyhow::anyhow!("session belongs to another directory").into())
}

fn tool_result(tool: &message::ToolPart) -> Option<(&serde_json::Map<String, Value>, &str, &str)> {
    match &tool.state {
        message::ToolState::Completed(state) => Some((&state.input, &state.output, "completed")),
        message::ToolState::Error(state) => Some((&state.input, &state.error, "failed")),
        message::ToolState::Pending(_) | message::ToolState::Running(_) => None,
    }
}

fn project_message(msg: &message::WithParts, include_tools: bool) -> ChatMessage {
    let (role, model) = match &msg.info {
        message::Info::User(_) => (ChatRole::User, None),
        message::Info::Assistant(info) => {
            let model = (!info.model_id.is_empty()).then(|| {
                if info.provider_id.is_empty() {
                    info.model_id.clone()
                } else {
                    format!("{}/{}", info.provider_id, info.model_id)
                }
            });
            (ChatRole::Assistant, model)
        }
    };
    let mut content = String::new();
    for part in &msg.parts {
        match part {
            message::Part::Text(text)
                if text.ignored != Some(true) && text.synthetic != Some(true) =>
            {
                content.push_str(&text.text);
            }
            message::Part::Tool(tool) if include_tools => {
                if let Some((input, output, status)) = tool_result(tool) {
                    if !content.is_empty() && !content.ends_with('\n') {
                        content.push('\n');
                    }
                    let payload = json!({
                        "tool_call_id": tool.call_id, "status": status,
                        "input": Value::Object(input.clone()).to_string(), "output": output,
                    });
                    let _ = writeln!(content, "tool {}\n{payload}", tool.tool);
                }
            }
            _ => {}
        }
    }
    ChatMessage { role, content, model, think_timing: Vec::new(), processing_timing: None }
}

fn model_history(messages: &[message::WithParts]) -> Vec<Value> {
    let mut history = Vec::new();
    for msg in messages {
        let projected = project_message(msg, false);
        if projected.role == ChatRole::Assistant {
            for part in &msg.parts {
                if let message::Part::Tool(tool) = part
                    && let Some((input, output, _)) = tool_result(tool)
                    && !tool.call_id.is_empty()
                    && !tool.tool.is_empty()
                {
                    history.push(json!({"role":"assistant", "tool_calls":[{
                        "id":tool.call_id, "type":"function",
                        "function":{"name":tool.tool, "arguments":Value::Object(input.clone()).to_string()},
                    }]}));
                    history.push(
                        json!({"role":"tool", "tool_call_id":tool.call_id, "content":output}),
                    );
                }
            }
        }
        if !projected.content.is_empty() {
            history.push(json!({"role": match projected.role {
                ChatRole::User => "user",
                _ => "assistant",
            }, "content": projected.content}));
        }
    }
    history
}

pub(crate) struct TurnFork {
    source: Info,
    messages: Vec<message::WithParts>,
    settings: agent_binding::ForkSettings,
}

impl TurnFork {
    pub(crate) fn source_directory(&self) -> &str {
        &self.source.directory
    }

    pub(crate) async fn prepare(session_id: &str, boundary: &str) -> Result<Self, Error> {
        let _history = message::lock_history(session_id).await;
        let source = super::get(session_id).await?;
        ensure_directory(&source, &instance::directory()).await?;
        if source.revert.is_some() {
            return Err(anyhow::anyhow!("source session has an unresolved rollback").into());
        }

        let all = message::fork_snapshot(session_id).await?;
        let mut users = Vec::new();
        let mut assistants: HashMap<String, Vec<message::WithParts>> = HashMap::new();
        for msg in all {
            let owner = match &msg.info {
                message::Info::User(info) => &info.session_id,
                message::Info::Assistant(info) => &info.session_id,
            };
            if owner != session_id
                || msg.parts.iter().any(|part| {
                    part.session_id() != session_id || part.message_id() != msg.info.id()
                })
            {
                return Err(anyhow::anyhow!(
                    "source history contains an invalid message association"
                )
                .into());
            }
            match &msg.info {
                message::Info::User(_) => users.push(msg),
                message::Info::Assistant(info) => {
                    assistants.entry(info.parent_id.clone()).or_default().push(msg);
                }
            }
        }
        users.sort_by(|left, right| {
            (created_at(&left.info), left.info.id())
                .cmp(&(created_at(&right.info), right.info.id()))
        });
        let boundary_index = users
            .iter()
            .position(|msg| {
                msg.info.id() == boundary
                    && !msg.parts.is_empty()
                    && !msg.parts.iter().any(|part| matches!(part, message::Part::Compaction(_)))
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "turn {boundary} is not a persisted user turn of the source session"
                )
            })?;

        // A boundary without a completed response may be part of an unfinished write.
        if !assistants.get(boundary).is_some_and(|children| is_complete(children)) {
            return Err(
                anyhow::anyhow!("turn {boundary} has no persisted completed response").into()
            );
        }
        let known_users =
            users.iter().map(|msg| msg.info.id()).collect::<std::collections::HashSet<_>>();
        let boundary_time = created_at(&users[boundary_index].info);
        if assistants.iter().any(|(parent, children)| {
            !known_users.contains(parent.as_str())
                && children.iter().any(|child| created_at(&child.info) <= boundary_time)
        }) {
            return Err(anyhow::anyhow!(
                "source history has an assistant message without a user turn"
            )
            .into());
        }

        let mut messages = Vec::new();
        for user in users.into_iter().take(boundary_index) {
            let mut children = assistants.remove(user.info.id()).unwrap_or_default();
            let compaction =
                user.parts.iter().any(|part| matches!(part, message::Part::Compaction(_)));
            if user.parts.is_empty() || (!compaction && !is_complete(&children)) {
                return Err(anyhow::anyhow!("earlier turn {} is incomplete", user.info.id()).into());
            }
            children.sort_by(|left, right| {
                (created_at(&left.info), left.info.id())
                    .cmp(&(created_at(&right.info), right.info.id()))
            });
            messages.push(user);
            messages.extend(children);
        }

        let settings = agent_binding::ForkSettings::load(&source.project_id, session_id).await?;
        Ok(Self { source, messages, settings })
    }

    pub(crate) async fn persist(self) -> Result<Info, Error> {
        let Self { source, mut messages, settings } = self;
        if super::project_id()? != source.project_id {
            return Err(anyhow::anyhow!("fork target belongs to another project").into());
        }
        let mut target = super::new_info(CreateInput {
            directory: instance::directory(),
            parent_id: None,
            title: Some(forked_title(&source.title)),
            permission: source.permission.clone(),
        })?;

        target.slug = format!("{FORK_SLUG_PREFIX}{}", target.slug);
        let mut current_user = String::new();
        for msg in &mut messages {
            let msg_id = id::ascending(id::Prefix::Message, None)?;
            if let message::Info::Assistant(info) = &mut msg.info {
                info.parent_id.clone_from(&current_user);
            } else {
                current_user.clone_from(&msg_id);
            }
            msg.info.set_session_id(&target.id);
            msg.info.set_id(&msg_id);
            for part in &mut msg.parts {
                #[cfg(not(target_arch = "wasm32"))]
                crate::session::dynamic_tool_store::interrupt_part(part);
                part.set_session_id(&target.id);
                part.set_message_id(&msg_id);
                part.set_id(&id::ascending(id::Prefix::Part, None)?);
            }
        }

        let result = persist_history_and_session(&source, &target, &messages, &settings).await;
        if let Err(error) = result {
            for msg in &messages {
                for part in &msg.parts {
                    if let Err(error) = storage::remove(&["part", msg.info.id(), part.id()]).await {
                        tracing::warn!(session_id = %target.id, %error, "failed to remove staged fork part");
                    }
                }
                if let Err(error) = storage::remove(&["message", &target.id, msg.info.id()]).await {
                    tracing::warn!(session_id = %target.id, %error, "failed to remove staged fork message");
                }
            }
            if let Err(error) = settings.remove(&target.project_id, &target.id).await {
                tracing::warn!(session_id = %target.id, %error, "failed to remove staged fork settings");
            }
            if let Err(error) = compaction_checkpoint::remove(&target.id, &target.directory).await {
                tracing::warn!(session_id = %target.id, %error, "failed to remove staged fork checkpoints");
            }
            let id = target.id.clone();
            let project = target.project_id.clone();
            let tenant = crate::session::ui_store::current_tenant_scope();
            crate::global_database::run_isolated(move || {
                crate::session::ui_store::with_tenant_scope_sync(tenant, || {
                    crate::session::ui_store::delete_session_scoped(&id, Some(&project));
                })
            })
            .await?;
            return Err(error);
        }

        let _ = bus::publish(
            event::CREATED,
            serde_json::json!({ "info": target }),
            Some(target.directory.clone()),
        );
        Ok(target)
    }
}

fn is_complete(children: &[message::WithParts]) -> bool {
    children.iter().all(|msg| !msg.parts.is_empty())
        && children.iter().any(|msg| {
            matches!(&msg.info, message::Info::Assistant(info) if info.time.completed.is_some())
        })
}

fn created_at(info: &message::Info) -> u64 {
    match info {
        message::Info::User(info) => info.time.created,
        message::Info::Assistant(info) => info.time.created,
    }
}

async fn persist_history_and_session(
    source: &Info,
    target: &Info,
    messages: &[message::WithParts],
    settings: &agent_binding::ForkSettings,
) -> Result<(), Error> {
    for msg in messages {
        message::write_message(&msg.info).await?;
        for part in &msg.parts {
            message::write_part(part).await?;
        }
    }
    settings.save(&target.project_id, &target.id).await?;
    compaction_checkpoint::copy_fork(
        &source.directory,
        &target.directory,
        &source.id,
        &target.id,
        &model_history(messages),
    )
    .await
    .map_err(anyhow::Error::msg)?;
    let tenant = crate::session::ui_store::current_tenant_scope();
    let project = target.project_id.clone();
    let snapshot = ChatSession {
        id: target.id.clone(),
        title: target.title.clone(),
        messages: messages.iter().map(|msg| project_message(msg, true)).collect(),
        message_ids: messages.iter().map(|msg| Some(msg.info.id().to_owned())).collect(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: target.time.created,
        updated_ms: target.time.updated,
    };
    crate::global_database::run_isolated(move || {
        crate::session::ui_store::save_session_ui_scoped_for_tenant(
            &tenant,
            &snapshot,
            Some(&project),
            false,
        )
    })
    .await??;
    // Publishing the session row last keeps partially copied history out of session listings.
    super::persist_info(target).await
}
