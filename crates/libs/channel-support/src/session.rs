//! 通道会话绑定与所有权存储；执行动作经具体会话操作端口提供。
use crate::scope::{ChannelMessageScope, ChannelMessageSnapshot, ChannelChatScope};
use crate::routing::ChannelRouteSelection;
use std::{collections::HashMap, path::Path, sync::{Mutex, OnceLock}, future::Future, pin::Pin};
use anyhow::{Context, Result};
use channel_types as traits;

pub trait ChannelSessions {
    fn get<'a>(&'a self, id: &'a str) -> Pin<Box<dyn Future<Output=Result<session_info::Info, session_info::Error>> + Send + 'a>>;
    #[cfg(not(target_arch="wasm32"))]
    fn reserve<'a>(&'a self, snapshot: &'a ChannelMessageSnapshot) -> Pin<Box<dyn Future<Output=Result<session_info::Info>> + Send + 'a>>;
    #[cfg(not(target_arch="wasm32"))]
    fn discard<'a>(&'a self, snapshot: &'a ChannelMessageSnapshot, info: session_info::Info) -> Pin<Box<dyn Future<Output=Result<()>> + Send + 'a>>;
}
pub fn sender_session_store() -> &'static Mutex<HashMap<ChannelMessageScope, String>> {
    static STORE: OnceLock<Mutex<HashMap<ChannelMessageScope, String>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}
/// 所有权只随新建登记；作用域键哈希避免把外部聊天 ID 当作文件路径。
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChannelSessionOwnership {
    pub scope: String,
    pub route: Option<ChannelRouteSelection>,
}

pub fn ownership_scope_key(snapshot: &ChannelMessageSnapshot) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(snapshot.scope.key().as_bytes()))
}


pub async fn resolve_or_create_sender_session_id(
    host: &impl ChannelSessions,
    snapshot: &ChannelMessageSnapshot,
    msg: &traits::ChannelMessage,
    route: Option<ChannelRouteSelection>,
) -> Result<String> {
    anyhow::ensure!(
        snapshot.scope.chat == {
            let mut chat = ChannelChatScope::from_message(msg, snapshot.scope.chat.tenant.clone());
            chat.tenant = snapshot.scope.chat.tenant.clone();
            chat
        },
        "channel message snapshot identity mismatch"
    );
    anyhow::ensure!(snapshot.directory.is_dir(), "channel project directory unavailable");
    {
        let existing = sender_session_store()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&snapshot.scope)
            .cloned();
        if let Some(existing) = existing {
            if let Ok(info) = host.get(&existing).await {
                if info.project_id == snapshot.scope.project
                    && info.time.archived.is_none()
                    && Path::new(&info.directory) == snapshot.directory
                {
                    return Ok(existing);
                }
            }
        }
        create_channel_session(host, snapshot, route).await
    }
}

pub async fn channel_session_ownership(
    snapshot: &ChannelMessageSnapshot,
    id: &str,
) -> Result<ChannelSessionOwnership> {
    anyhow::ensure!(
        id.starts_with("ses_") && id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_'),
        "无效会话 ID"
    );
    let record: ChannelSessionOwnership =
        agent_storage::read(&["channel_session", &ownership_scope_key(snapshot), id])
            .await
            .context("会话不属于当前聊天和项目")?;
    anyhow::ensure!(record.scope == snapshot.scope.key(), "会话作用域不匹配");
    Ok(record)
}

pub async fn save_channel_session_route(
    snapshot: &ChannelMessageSnapshot,
    id: &str,
    route: &ChannelRouteSelection,
) -> Result<()> {
    let mut record = channel_session_ownership(snapshot, id).await?;
    if record.route.as_ref() == Some(route) {
        return Ok(());
    }
    record.route = Some(route.clone());
    agent_storage::write_atomic(&["channel_session", &ownership_scope_key(snapshot), id], &record)
        .await?;
    Ok(())
}

pub async fn owned_channel_sessions(
    host: &impl ChannelSessions,
    snapshot: &ChannelMessageSnapshot,
) -> Result<Vec<session_info::Info>> {
    let mut sessions = Vec::new();
    for key in agent_storage::list(&["channel_session", &ownership_scope_key(snapshot)]).await? {
        let Some(id) = key.last() else { continue };
        channel_session_ownership(snapshot, id).await?;
        match host.get(id).await {
            Ok(info)
                if info.project_id == snapshot.scope.project
                    && Path::new(&info.directory) == snapshot.directory
                    && info.time.archived.is_none()
                    && !info.is_first_response_pending() =>
            {
                sessions.push(info)
            }
            Ok(_)
            | Err(session_info::Error::Storage(agent_storage::Error::NotFound(_))) => {}
            Err(err) => return Err(err.into()),
        }
    }
    sessions.sort_by(|a, b| b.time.updated.cmp(&a.time.updated).then(a.id.cmp(&b.id)));
    Ok(sessions)
}

pub async fn create_channel_session(
    host: &impl ChannelSessions,
    snapshot: &ChannelMessageSnapshot,
    route: Option<ChannelRouteSelection>,
) -> Result<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (host, snapshot, route);
        anyhow::bail!("通道会话所有权持久化不支持 WASM");
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        anyhow::ensure!(snapshot.directory.is_dir(), "channel project directory unavailable");
        let info = host.reserve(snapshot).await.context("failed to create channel session")?;
        let record = ChannelSessionOwnership { scope: snapshot.scope.key(), route };
        if let Err(err) = agent_storage::write_atomic(
            &["channel_session", &ownership_scope_key(snapshot), &info.id],
            &record,
        )
        .await
        {
            // 登记失败的孤立新会话不可恢复；旧绑定未动。
            host.discard(snapshot, info).await.context("failed to discard unowned channel reservation")?;
            return Err(err.into());
        }
        sender_session_store()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(snapshot.scope.clone(), info.id.clone());
        Ok(info.id)
    }
}

pub fn clear_sender_session_scope(scope: &ChannelMessageScope) {
    sender_session_store().lock().unwrap_or_else(|e| e.into_inner()).remove(scope);
}
