//! Process-local authenticated WeCom connections. Never opens a second socket for delivery.
use super::{OutboundItem, outbound};
use crate::config::Config;
use anyhow::{Context, bail};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};
use tokio::sync::mpsc;
use uuid::Uuid;
use vw_api_types::id::TenantScope;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct WeComRuntimeScope {
    pub(super) tenant: TenantScope,
    pub(super) config_store: PathBuf,
    pub(super) workspace: PathBuf,
    config_source: PathBuf,
    instance: &'static str,
}

impl WeComRuntimeScope {
    /// Capture the trusted tenant and runtime-resolved storage source before spawning listeners.
    pub fn from_config(config: &Config, tenant: TenantScope) -> anyhow::Result<Self> {
        let home = crate::global::paths().home.clone();
        anyhow::ensure!(home.is_absolute(), "Could not resolve an absolute runtime home directory");
        Ok(Self {
            tenant,
            config_store: vw_config_types::paths::global_database_config_path(home),
            config_source: config.config_path.clone(),
            workspace: config.workspace_dir.clone(),
            instance: "default",
        })
    }
}

#[derive(Clone)]
pub struct WeComRuntimeHandle {
    sender: mpsc::Sender<OutboundItem>,
}

impl WeComRuntimeHandle {
    pub async fn send_proactive(&self, recipient: &str, content: &str) -> anyhow::Result<()> {
        if recipient.trim().is_empty() || content.trim().is_empty() {
            return Err(outbound::delivery_error(super::DeliveryFailureKind::NotSent));
        }
        outbound::send_proactive(&self.sender, recipient, content).await
    }
}

pub(super) struct Entry {
    generation: Uuid,
    sender: Option<mpsc::Sender<OutboundItem>>,
    credentials: Option<[u8; 32]>,
}

pub(super) fn registry() -> &'static Mutex<HashMap<WeComRuntimeScope, Entry>> {
    static REGISTRY: OnceLock<Mutex<HashMap<WeComRuntimeScope, Entry>>> = OnceLock::new();
    REGISTRY.get_or_init(Mutex::default)
}

pub fn connected(scope: &WeComRuntimeScope) -> anyhow::Result<WeComRuntimeHandle> {
    let entries = registry().lock().unwrap_or_else(|e| e.into_inner());
    let sender = entries
        .get(scope)
        .and_then(|entry| entry.sender.as_ref())
        .filter(|sender| !sender.is_closed())
        .context(
            "当前进程没有已认证的企微连接，请启动拥有此配置和工作区的 daemon 并等待企微连接成功",
        )?;
    Ok(WeComRuntimeHandle { sender: sender.clone() })
}

// 长度分隔，避免不同 ID/Secret 组合产生相同输入；不存储额外明文凭据。
fn credential_digest(bot_id: &str, secret: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(bot_id.len().to_be_bytes());
    hash.update(bot_id.as_bytes());
    hash.update(secret.as_bytes());
    hash.finalize().into()
}

/// 配置保存与监听器重连之间也不能借旧连接发送。
pub(crate) fn connected_with_credentials(
    scope: &WeComRuntimeScope,
    bot_id: &str,
    secret: &str,
) -> anyhow::Result<WeComRuntimeHandle> {
    let entries = registry().lock().unwrap_or_else(|e| e.into_inner());
    let entry = entries
        .get(scope)
        .context("当前进程没有已认证的企微连接，请启动同一配置和工作区的 daemon")?;
    if entry.credentials != Some(credential_digest(bot_id, secret)) {
        bail!("企微凭据已更新或尚未认证，请等待 daemon 重新连接");
    }
    let sender = entry
        .sender
        .as_ref()
        .filter(|sender| !sender.is_closed())
        .context("当前进程没有已认证的企微连接，请等待 daemon 重新连接")?;
    Ok(WeComRuntimeHandle { sender: sender.clone() })
}

pub(super) struct Registration {
    scope: WeComRuntimeScope,
    generation: Uuid,
}

impl Registration {
    pub(super) fn claim(scope: WeComRuntimeScope) -> anyhow::Result<Self> {
        let mut entries = registry().lock().unwrap_or_else(|e| e.into_inner());
        if entries.contains_key(&scope) {
            bail!("此作用域已有企微监听器，请先停止旧监听器");
        }
        let generation = Uuid::new_v4();
        entries.insert(scope.clone(), Entry { generation, sender: None, credentials: None });
        Ok(Self { scope, generation })
    }

    pub(super) fn publish(&self, sender: mpsc::Sender<OutboundItem>, bot_id: &str, secret: &str) {
        let mut entries = registry().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = entries.get_mut(&self.scope)
            && entry.generation == self.generation
        {
            entry.sender = Some(sender);
            entry.credentials = Some(credential_digest(bot_id, secret));
        }
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        let mut entries = registry().lock().unwrap_or_else(|e| e.into_inner());
        if entries.get(&self.scope).is_some_and(|entry| entry.generation == self.generation) {
            entries.remove(&self.scope);
        }
    }
}
