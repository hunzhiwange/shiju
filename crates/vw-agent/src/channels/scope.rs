//! 消息身份、项目解析和不可变执行快照。绑定失败时拒绝执行，不发现或创建项目。
use super::*;
use serde::Serialize;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub(crate) struct ChannelChatScope {
    pub(crate) tenant: TenantScope,
    pub(crate) channel: String,
    pub(crate) instance: String,
    pub(crate) reply_target: String,
    pub(crate) thread: Option<String>,
    pub(crate) sender: String,
}

impl ChannelChatScope {
    pub(crate) fn from_message(msg: &traits::ChannelMessage) -> Self {
        Self {
            tenant: crate::session::ui_store::current_tenant_scope(),
            channel: msg.channel.clone(),
            instance: msg.instance_id.clone(),
            reply_target: msg.reply_target.clone(),
            thread: msg.thread_ts.clone(),
            sender: msg.sender.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub(crate) struct ChannelMessageScope {
    pub(crate) chat: ChannelChatScope,
    pub(crate) project: String,
}

impl ChannelMessageScope {
    pub(crate) fn key(&self) -> String {
        // JSON 编码保留字段边界，用户 ID 内的分隔符不会造成身份碰撞。
        serde_json::to_string(self).expect("channel scope contains only serializable fields")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChannelProjectSource {
    Chat,
    Instance,
    Workspace,
}

#[derive(Debug, Clone)]
pub(crate) struct ChannelMessageSnapshot {
    pub(crate) scope: ChannelMessageScope,
    pub(crate) directory: PathBuf,
    pub(crate) project: crate::project::Info,
    pub(crate) source: ChannelProjectSource,
}

#[derive(Default)]
pub(crate) struct ChannelProjectState {
    pub(crate) tenant: TenantScope,
    pub(crate) config: crate::app::agent::config::ChannelsConfig,
    pub(crate) selections: HashMap<ChannelChatScope, String>,
}

pub(crate) async fn resolve_channel_message_snapshot(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
) -> Result<ChannelMessageSnapshot> {
    let mut chat = ChannelChatScope::from_message(msg);
    chat.tenant = ctx.project_state.lock().unwrap_or_else(|e| e.into_inner()).tenant.clone();
    if let Some(snapshot) = &ctx.message_snapshot {
        anyhow::ensure!(snapshot.scope.chat == chat, "channel message snapshot identity mismatch");
        return Ok(snapshot.clone());
    }
    let (selected, source, fallback) = {
        let state = ctx.project_state.lock().unwrap_or_else(|e| e.into_inner());
        let instance_key = format!("{}:{}", chat.channel, chat.instance);
        let (selected, source) = if let Some(id) = state.selections.get(&chat) {
            (Some(id.clone()), ChannelProjectSource::Chat)
        } else if let Some(id) = state.config.instance_default_projects.get(&instance_key) {
            (Some(id.clone()), ChannelProjectSource::Instance)
        } else {
            (None, ChannelProjectSource::Workspace)
        };
        (selected, source, state.config.project_dir.clone())
    };
    let (project, directory) = if let Some(id) = selected {
        let scope = ProjectScope::new(chat.tenant.clone(), ProjectId::from(id.clone()))?;
        let info = crate::project::get(&scope).await?.ok_or_else(|| {
            anyhow::anyhow!("channel project is not registered in this tenant: {id}")
        })?;
        let directory = std::fs::canonicalize(&info.worktree)
            .with_context(|| format!("channel project directory unavailable: {id}"))?;
        (info, directory)
    } else {
        let directory = std::fs::canonicalize(fallback.as_deref().unwrap_or(&ctx.workspace_dir))
            .context("channel workspace directory unavailable")?;
        let projects = crate::project::list(&chat.tenant).await?;
        let info = projects
            .into_iter()
            .find(|info| {
                std::iter::once(&info.worktree)
                    .chain(info.sandboxes.iter())
                    .any(|path| std::fs::canonicalize(path).is_ok_and(|path| path == directory))
            })
            .ok_or_else(|| anyhow::anyhow!("channel workspace is not a registered project"))?;
        (info, directory)
    };
    anyhow::ensure!(directory.is_dir(), "channel project directory is not a directory");
    Ok(ChannelMessageSnapshot {
        scope: ChannelMessageScope { chat, project: project.id.clone() },
        directory,
        project,
        source,
    })
}

pub(crate) fn scoped_history_key(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
) -> String {
    ctx.message_snapshot
        .as_ref()
        .map_or_else(|| conversation_history_key(msg), |snapshot| snapshot.scope.key())
}

pub(crate) fn channel_project_directory(ctx: &ChannelRuntimeContext) -> PathBuf {
    ctx.message_snapshot.as_ref().map_or_else(
        || {
            ctx.project_state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .config
                .project_dir
                .clone()
                .unwrap_or_else(|| ctx.workspace_dir.as_ref().clone())
        },
        |snapshot| snapshot.directory.clone(),
    )
}

/// 以结构化编码索引出站实例，避免同平台多机器人互相覆盖。
pub(crate) fn channel_instance_key(channel: &str, instance: &str) -> String {
    serde_json::to_string(&(channel, instance)).expect("serializable channel identity")
}

pub(crate) fn channel_for_message(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
) -> Option<Arc<dyn Channel>> {
    ctx.channels_by_name
        .get(&channel_instance_key(&msg.channel, &msg.instance_id))
        .or_else(|| {
            (msg.instance_id == "default").then(|| ctx.channels_by_name.get(&msg.channel)).flatten()
        })
        .cloned()
}
