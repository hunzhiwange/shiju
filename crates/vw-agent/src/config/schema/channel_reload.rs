//! 仅在数据库内容变化时加载通道配置；支持网关与通道运行于不同进程。

use super::{ChannelsConfig, Config, apply_env_overrides, config_io::merge_config_payload};
use crate::app::agent::config::SystemConfigStore;
use crate::app::agent::security::SecretStore;
use crate::global_database::GlobalDatabase;
use anyhow::{Context, Result, bail};
use directories::UserDirs;
use serde_json::Value;
use vw_api_types::id::TenantScope;

pub(crate) struct ChannelConfigWatcher {
    store: SystemConfigStore,
    project: Option<String>,
    last_payload: Option<Value>,
}

impl ChannelConfigWatcher {
    pub(crate) async fn new(config: &Config, include_project: bool) -> Result<Self> {
        let home =
            UserDirs::new().context("Could not find home directory")?.home_dir().to_path_buf();
        let store = tokio::task::spawn_blocking(move || {
            SystemConfigStore::new(GlobalDatabase::for_home(home)?)
        })
        .await
        .context("join channel config store initialization")??;
        Self::from_store(store, config, include_project)
    }

    pub(crate) fn from_store(
        store: SystemConfigStore,
        config: &Config,
        include_project: bool,
    ) -> Result<Self> {
        let project = if include_project {
            Some(config.workspace_dir.to_str().context("Project root must be UTF-8")?.to_owned())
        } else {
            None
        };
        Ok(Self { store, project, last_payload: None })
    }

    /// 读取默认租户的持久化配置；读取或校验失败时不推进快照；渠道解密失败只隔离故障实例。
    pub(crate) async fn poll(&mut self, current: &Config) -> Result<Option<ChannelsConfig>> {
        let store = self.store.clone();
        let project = self.project.clone();
        let payload = tokio::task::spawn_blocking(move || {
            let tenant = TenantScope::default();
            let Some(raw) = store.get(&tenant)? else {
                return Ok::<_, anyhow::Error>(None);
            };
            let mut payload: Value = serde_json::from_str(&raw)?;
            if !payload.is_object() {
                bail!("System config root must be an object");
            }
            if let Some(project) = project
                && let Some(raw) = store.get_project(&tenant, &project)?
            {
                let patch: Value = serde_json::from_str(&raw)?;
                if !patch.is_object() {
                    bail!("Project config root must be an object");
                }
                merge_config_payload(&mut payload, patch);
            }
            // 其它设置和随机加密后的非通道密钥不触发通道重连。
            Ok(Some(serde_json::json!({
                "channels_config": payload.get("channels_config"),
                "encrypt": payload.pointer("/secrets/encrypt"),
            })))
        })
        .await
        .context("join channel config read")??;
        let Some(payload) = payload else { return Ok(None) };
        if self.last_payload.as_ref() == Some(&payload) {
            return Ok(None);
        }

        let mut next = current.clone();
        next.channels_config = match payload.get("channels_config").filter(|value| !value.is_null())
        {
            Some(value) => serde_json::from_value(value.clone())?,
            None => ChannelsConfig::default(),
        };
        let config_dir = current.config_path.parent().context("Config path must have a parent")?;
        let encrypt =
            payload.get("encrypt").and_then(Value::as_bool).unwrap_or(current.secrets.encrypt);
        let secrets = SecretStore::new(config_dir, encrypt);
        super::channels::decrypt_channel_secrets_isolated(&secrets, &mut next.channels_config)?;
        apply_env_overrides(&mut next);
        super::validate_config(&next)?;
        let changed = next.channels_config.unavailable_channels
            != current.channels_config.unavailable_channels
            || serde_json::to_value(&next.channels_config)?
                != serde_json::to_value(&current.channels_config)?;
        self.last_payload = Some(payload);
        Ok(changed.then_some(next.channels_config))
    }
}

#[cfg(test)]
#[path = "channel_reload_tests.rs"]
mod tests;
