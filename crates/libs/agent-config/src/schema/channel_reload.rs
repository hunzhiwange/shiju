//! 仅在数据库内容变化时加载通道配置；支持网关与通道运行于不同进程。

use super::{ChannelsConfig, Config};

use secret_store::secrets::SecretStore;

use anyhow::{Context, Result, bail};
use serde_json::Value;
use vw_api_transport::id::TenantScope;

/// 数据库读取窄端口；宿主实现维持原数据库后端与错误语义。
pub trait ChannelConfigStore: Clone + Send + 'static {
    fn get(&self, tenant: &TenantScope) -> Result<Option<String>>;
    fn get_project(&self, tenant: &TenantScope, project: &str) -> Result<Option<String>>;
}

pub struct ChannelConfigWatcher<S> {
    store: S,
    tenant: TenantScope,
    project: Option<String>,
    last_payload: Option<Value>,
    apply_env: fn(&mut Config),
    validate: fn(&Config) -> Result<()>,
    merge: fn(&mut Value, Value),
}

impl<S: ChannelConfigStore> ChannelConfigWatcher<S> {
    pub fn from_store(store: S, config: &Config, include_project: bool, tenant: TenantScope,
        apply_env: fn(&mut Config), validate: fn(&Config) -> Result<()>,
        merge: fn(&mut Value, Value)) -> Result<Self> {
        let project = if include_project {
            Some(config.workspace_dir.to_str().context("Project root must be UTF-8")?.to_owned())
        } else { None };
        Ok(Self { store, tenant, project, last_payload: None, apply_env, validate, merge })
    }

    /// 读取创建时捕获租户的持久化配置；读取或校验失败时不推进快照；渠道解密失败只隔离故障实例。
    pub async fn poll(&mut self, current: &Config) -> Result<Option<ChannelsConfig>> {
        self.poll_inner(current, false).await
    }

    /// 投递必须有持久化配置；记录删除或读取失败不能继续使用启动时的凭据。
    pub async fn load_for_delivery(&mut self, current: &Config) -> Result<ChannelsConfig> {
        Ok(self.poll_inner(current, true).await?.unwrap_or_else(|| current.channels_config.clone()))
    }

    async fn poll_inner(
        &mut self,
        current: &Config,
        required: bool,
    ) -> Result<Option<ChannelsConfig>> {
        let store = self.store.clone();
        let merge_config_payload = self.merge;
        let project = self.project.clone();
        let tenant = self.tenant.clone();
        let payload = tokio::task::spawn_blocking(move || {
            let Some(raw) = store.get(&tenant)? else {
                if required {
                    bail!("Persisted channel configuration is missing");
                }
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
        (self.apply_env)(&mut next);
        (self.validate)(&next)?;
        let changed = next.channels_config.unavailable_channels
            != current.channels_config.unavailable_channels
            || serde_json::to_value(&next.channels_config)?
                != serde_json::to_value(&current.channels_config)?;
        self.last_payload = Some(payload);
        Ok(changed.then_some(next.channels_config))
    }
}
