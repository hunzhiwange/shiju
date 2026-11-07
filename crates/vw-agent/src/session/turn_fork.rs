//! Prepare a fork from a consistent snapshot of native messages.

use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use super::{CreateInput, Error, Info, event, forked_title};
use crate::project::instance;
use crate::session::{agent_binding, message};
use crate::{bus, id, storage};

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
        let directory = instance::directory();
        if source.directory != directory {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let expected = tokio::fs::canonicalize(Path::new(&source.directory)).await.ok();
                let actual = tokio::fs::canonicalize(Path::new(&directory)).await.ok();
                if expected.is_none() || expected != actual {
                    return Err(
                        anyhow::anyhow!("source session belongs to another directory").into()
                    );
                }
            }
            #[cfg(target_arch = "wasm32")]
            return Err(anyhow::anyhow!("source session belongs to another directory").into());
        }
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
        let target = super::new_info(CreateInput {
            directory: instance::directory(),
            parent_id: None,
            title: Some(forked_title(&source.title)),
            permission: source.permission,
        })?;

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

        let result = persist_history_and_session(&target, &messages, &settings).await;
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
    // Publishing the session row last keeps partially copied history out of session listings.
    super::persist_info(target).await
}
