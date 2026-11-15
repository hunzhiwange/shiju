//! 通道聊天身份、项目快照与取消完成状态的唯一所有者。
use channel_types as traits;
use std::{collections::HashMap, path::{Path, PathBuf}, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}};
use anyhow::{Context, Result};
use serde::Serialize;
use vw_api_transport::id::{ProjectId, ProjectScope, TenantScope};
use tokio_util::sync::CancellationToken;
/// 正在处理中的发送者任务状态。
///
/// 跟踪每个消息处理任务的生命周期，
/// 支持取消和完成通知。
#[derive(Clone)]
pub struct InFlightSenderTaskState {
    /// 任务唯一标识符
    pub task_id: u64,
    /// 用于取消任务的令牌
    pub cancellation: CancellationToken,
    /// 任务完成状态通知器
    pub completion: Arc<InFlightTaskCompletion>,
}

/// 正在处理中的任务完成状态。
///
/// 提供原子完成标志和异步等待机制，
/// 允许其他任务等待当前任务完成。
pub struct InFlightTaskCompletion {
    /// 完成标志（原子布尔值）
    pub done: AtomicBool,
    /// 用于通知等待者的异步原语
    pub notify: tokio::sync::Notify,
}

impl InFlightTaskCompletion {
    /// 创建新的未完成状态实例。
    pub fn new() -> Self {
        Self { done: AtomicBool::new(false), notify: tokio::sync::Notify::new() }
    }

    /// 标记任务为已完成。
    ///
    /// 设置完成标志并通知所有等待者。
    pub fn mark_done(&self) {
        self.done.store(true, Ordering::Release);
        self.notify.notify_waiters();
    }

    /// 异步等待任务完成。
    ///
    /// 如果任务已完成则立即返回，否则阻塞等待通知。
    pub async fn wait(&self) {
        // 先创建通知 Future，再检查完成状态，避免 notify_waiters 丢失唤醒。
        let notified = self.notify.notified();
        if !self.done.load(Ordering::Acquire) {
            notified.await;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ChannelChatScope {
    pub tenant: TenantScope,
    pub channel: String,
    pub instance: String,
    pub reply_target: String,
    pub thread: Option<String>,
    pub sender: String,
}

impl ChannelChatScope {
    pub fn from_message(msg: &traits::ChannelMessage, tenant: TenantScope) -> Self {
        Self {
            tenant,
            channel: msg.channel.clone(),
            instance: msg.instance_id.clone(),
            reply_target: msg.reply_target.clone(),
            // QQ 的 thread_ts 是被动回复所需的消息 ID，并不表示聊天主题。
            // 保留原消息用于发回 msg_id，但不把每条回复引用拆成独立会话。
            thread: if msg.channel == "qq" { None } else { msg.thread_ts.clone() },
            sender: msg.sender.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ChannelMessageScope {
    pub chat: ChannelChatScope,
    pub project: String,
}

impl ChannelMessageScope {
    pub fn key(&self) -> String {
        // JSON 编码保留字段边界，用户 ID 内的分隔符不会造成身份碰撞。
        serde_json::to_string(self).expect("channel scope contains only serializable fields")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelProjectSource {
    Chat,
    Instance,
    Workspace,
}

#[derive(Debug, Clone)]
pub struct ChannelMessageSnapshot {
    pub scope: ChannelMessageScope,
    pub directory: PathBuf,
    pub project: agent_project::Info,
    pub source: ChannelProjectSource,
    pub execution: Option<config_channels::ChannelExecutionConfig>,
}

#[derive(Default, Clone)]
pub struct ChannelProjectState {
    pub tenant: TenantScope,
    pub config: agent_config::schema::ChannelsConfig,
    pub selections: HashMap<ChannelChatScope, String>,
    pub active: HashMap<ChannelChatScope, InFlightSenderTaskState>,
    pub gates: HashMap<ChannelChatScope, std::sync::Weak<tokio::sync::RwLock<()>>>,
}


pub async fn resolve_channel_message_snapshot(
    project_state: &Mutex<ChannelProjectState>,
    message_snapshot: Option<&ChannelMessageSnapshot>,
    workspace_dir: &Path,
    msg: &traits::ChannelMessage,
) -> Result<ChannelMessageSnapshot> {
    let chat = ChannelChatScope::from_message(msg, project_state.lock().unwrap_or_else(|e| e.into_inner()).tenant.clone());
    if let Some(snapshot) = message_snapshot {
        anyhow::ensure!(snapshot.scope.chat == chat, "channel message snapshot identity mismatch");
        return Ok(snapshot.clone());
    }
    let (selected, source, fallback, execution) = {
        let state = project_state.lock().unwrap_or_else(|e| e.into_inner());
        let instance_key = format!("{}:{}", chat.channel, chat.instance);
        let (selected, source) = if let Some(id) = state.selections.get(&chat) {
            (Some(id.clone()), ChannelProjectSource::Chat)
        } else if let Some(id) = state.config.instance_default_projects.get(&instance_key) {
            (Some(id.clone()), ChannelProjectSource::Instance)
        } else {
            (None, ChannelProjectSource::Workspace)
        };
        (
            selected,
            source,
            state.config.project_dir.clone(),
            state.config.instance_execution.get(&instance_key).cloned(),
        )
    };
    let (project, directory) = if let Some(id) = selected {
        let scope = ProjectScope::new(chat.tenant.clone(), ProjectId::from(id.clone()))?;
        let info = agent_project::get(&scope).await?.ok_or_else(|| {
            anyhow::anyhow!("channel project is not registered in this tenant: {id}")
        })?;
        let directory = std::fs::canonicalize(&info.worktree)
            .with_context(|| format!("channel project directory unavailable: {id}"))?;
        (info, directory)
    } else {
        let directory = std::fs::canonicalize(fallback.as_deref().unwrap_or(workspace_dir))
            .context("channel workspace directory unavailable")?;
        let projects = agent_project::list(&chat.tenant).await?;
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
        execution,
    })
}

pub fn channel_instance_key(channel: &str, instance: &str) -> String {
    serde_json::to_string(&(channel, instance)).expect("serializable channel identity")
}

pub fn channel_chat_gate(
    project_state: &Mutex<ChannelProjectState>,
    msg: &traits::ChannelMessage,
) -> Arc<tokio::sync::RwLock<()>> {
    let tenant = project_state.lock().unwrap_or_else(|e| e.into_inner()).tenant.clone();
    let chat = ChannelChatScope::from_message(msg, tenant);
    let mut state = project_state.lock().unwrap_or_else(|e| e.into_inner());
    state.gates.retain(|_, gate| gate.strong_count() > 0);
    if let Some(gate) = state.gates.get(&chat).and_then(std::sync::Weak::upgrade) {
        return gate;
    }
    let gate = Arc::new(tokio::sync::RwLock::new(()));
    state.gates.insert(chat, Arc::downgrade(&gate));
    gate
}

use crate::runtime_state::ChannelRuntimeContext;
use channel_types::Channel;

pub fn scoped_history_key(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    tenant: TenantScope,
) -> String {
    ctx.message_snapshot
        .as_ref()
        .map_or_else(|| crate::history::conversation_history_key(msg, tenant), |snapshot| snapshot.scope.key())
}

pub fn channel_project_directory(ctx: &ChannelRuntimeContext) -> PathBuf {
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

pub fn channel_for_message(
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

pub fn channel_chat_scope(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
) -> ChannelChatScope {
    let tenant = ctx.project_state.lock().unwrap_or_else(|e| e.into_inner()).tenant.clone();
    ChannelChatScope::from_message(msg, tenant)
}
