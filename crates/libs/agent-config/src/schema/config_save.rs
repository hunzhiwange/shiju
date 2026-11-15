use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use config_aggregate::Config;
use super::config_io::merge_config_payload;
use super::config_secrets::{decrypt_config_secrets, encrypt_config_secrets};
use super::config_validate::validate_config;
#[cfg(not(target_arch = "wasm32"))]
pub fn encode_config_for_storage(config: &Config) -> Result<String> {
    let mut config_to_save = config.clone();
    config_to_save.gateway.skeys.clear();
    validate_config(&config_to_save)?;
    encrypt_config_secrets(&mut config_to_save)?;
    serde_json::to_string(&config_to_save).context("Failed to serialize system config")
}

/// 递归合并原始项目补丁；对象递归合并，其他值整体替换。
pub fn merge_raw_project_patch(target: &mut Value, patch: Value) {
    match (target, patch) {
        (Value::Object(target_object), Value::Object(patch_object)) => {
            for (key, patch_value) in patch_object {
                if let Some(current) = target_object.get_mut(&key)
                    && current.is_object()
                    && patch_value.is_object()
                {
                    merge_raw_project_patch(current, patch_value);
                } else {
                    target_object.insert(key, patch_value);
                }
            }
        }
        (target_value, patch_value) => *target_value = patch_value,
    }
}

/// 按覆盖模板选择加密后的有效值，避免把未覆盖字段写入项目配置。
pub fn select_override_shape(shape: &Value, encrypted_effective: Option<&Value>) -> Value {
    match shape {
        Value::Object(shape_object) => {
            let encrypted_object = encrypted_effective.and_then(Value::as_object);
            let selected = shape_object
                .iter()
                .map(|(key, child_shape)| {
                    let encrypted_child = encrypted_object.and_then(|object| object.get(key));
                    (key.clone(), select_override_shape(child_shape, encrypted_child))
                })
                .collect();
            Value::Object(selected)
        }
        Value::Null => Value::Null,
        _ => encrypted_effective.cloned().unwrap_or_else(|| shape.clone()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// 合并并验证项目覆盖；拒绝 agents 修改，保留旧记录，并剔除运行期 skey 字段。
pub fn prepare_project_override(
    base: &Config,
    mut raw_override: Value,
    mut patch: Value,
    catalog: &std::collections::HashMap<String, config_agent::AgentDefinitionConfig>,
) -> Result<Value> {
    if patch.get("worktree_settings").is_some() {
        bail!("worktree_settings are global; update tenant global config instead");
    }
    if let Some(object) = raw_override.as_object_mut() {
        object.remove("worktree_settings");
    }
    if patch.get("agents").is_some() {
        bail!("agents are global; update tenant global config instead");
    }
    // Preserve legacy records without letting them affect validation or encryption.
    let legacy_agents = raw_override.as_object_mut().and_then(|object| object.remove("agents"));
    super::config_validate::validate_agent_profile_patch(&patch)?;
    let mut current_payload = serde_json::to_value(base)?;
    merge_config_payload(&mut current_payload, raw_override.clone());
    let current = serde_json::from_value(current_payload)?;
    super::config_validate::seed_agent_profile_patch(&current, &mut patch, catalog)?;
    let mut next_override = raw_override;
    merge_raw_project_patch(&mut next_override, patch);
    if let Some(override_object) = next_override.as_object_mut() {
        override_object.remove(super::CONFIG_AGENT_KEY);
        if let Some(gateway) = override_object.get_mut("gateway").and_then(Value::as_object_mut) {
            gateway.remove("skeys");
        }
    }

    let mut effective_payload =
        serde_json::to_value(base).context("Failed to serialize tenant base config")?;
    merge_config_payload(&mut effective_payload, next_override.clone());
    let mut effective: Config = serde_json::from_value(effective_payload)
        .context("Failed to deserialize effective project config")?;
    effective.config_path = base.config_path.clone();
    effective.workspace_dir = base.workspace_dir.clone();
    effective.gateway.skeys.clear();
    let config_dir = base
        .config_path
        .parent()
        .context("Tenant base config path must have a parent directory")?;
    decrypt_config_secrets(&mut effective, config_dir)?;
    validate_config(&effective)?;
    encrypt_config_secrets(&mut effective)?;
    let encrypted_effective =
        serde_json::to_value(effective).context("Failed to serialize effective project config")?;
    let mut selected = select_override_shape(&next_override, Some(&encrypted_effective));
    if let Some(agents) = legacy_agents {
        selected
            .as_object_mut()
            .context("project config payload root must be an object")?
            .insert("agents".into(), agents);
    }
    Ok(selected)
}

/// 计算从 `before` 变为 `after` 的递归合并补丁；无变化时返回 `None`。
pub fn merge_patch_between(before: &Value, after: &Value) -> Option<Value> {
    if before == after {
        return None;
    }
    match (before, after) {
        (Value::Object(before_object), Value::Object(after_object)) => {
            let keys =
                before_object.keys().chain(after_object.keys()).cloned().collect::<BTreeSet<_>>();
            let mut patch = Map::new();
            for key in keys {
                match (before_object.get(&key), after_object.get(&key)) {
                    (Some(before_value), Some(after_value)) => {
                        if let Some(value) = merge_patch_between(before_value, after_value) {
                            patch.insert(key, value);
                        }
                    }
                    (Some(_), None) => {
                        patch.insert(key, Value::Null);
                    }
                    (None, Some(after_value)) => {
                        patch.insert(key, after_value.clone());
                    }
                    (None, None) => {}
                }
            }
            (!patch.is_empty()).then_some(Value::Object(patch))
        }
        _ => Some(after.clone()),
    }
}

use std::path::Path;
use vw_api_transport::id::TenantScope;

/// 项目覆盖使用的租户隔离读写端口。
pub trait ProjectConfigStore: Clone + Send + 'static {
    fn get_project(&self, tenant: &TenantScope, project: &str) -> Result<Option<String>>;
    fn upsert_project(&self, tenant: &TenantScope, project: &str, payload: &str) -> Result<()>;
}
/// 组合层提供现有数据库和完整配置加载入口。
pub trait ProjectConfigHost {
    type Store: ProjectConfigStore;
    fn open_store() -> impl std::future::Future<Output = Result<Self::Store>> + Send;
    fn load(tenant: &TenantScope, project: Option<&Path>) -> impl std::future::Future<Output = Result<Config>> + Send;
    fn catalog(config: &Config) -> std::collections::HashMap<String, config_agent::AgentDefinitionConfig>;
}
#[cfg(not(target_arch = "wasm32"))]
/// 校验项目根目录和 JSON 对象形状后保存租户项目覆盖。
pub fn save_project_config_to_store<S: ProjectConfigStore>(
    store: &S,
    tenant: &TenantScope,
    project_root: &Path,
    payload: &Value,
) -> Result<()> {
    if !payload.is_object() {
        bail!("project config payload root must be an object");
    }
    if !project_root.is_dir() {
        bail!("project root must be an existing directory");
    }
    let project_key = project_root.to_str().context("Project root must be valid UTF-8")?;
    let payload =
        serde_json::to_string(payload).context("Failed to serialize project config payload")?;
    store.upsert_project(tenant, project_key, &payload)
}

/// 更新指定租户的项目覆盖，并返回重新加载后的有效配置。
pub async fn update_project_config_for_tenant<H: ProjectConfigHost>(
    tenant: &TenantScope,
    project_root: &Path,
    patch: Value,
) -> Result<Config> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (tenant, project_root, patch);
        bail!("Project config overrides are not supported in WASM");
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let Some(patch_object) = patch.as_object() else {
            bail!("config patch must be a JSON object");
        };
        if patch_object.contains_key("agents") {
            bail!("agents are global; update tenant global config instead");
        }
        if !project_root.is_dir() {
            bail!("project root must be an existing directory");
        }
        if patch_object.is_empty() {
            return H::load(tenant, Some(project_root)).await;
        }
        let base = H::load(tenant, None).await?;
        let store = H::open_store().await?;
        let project_key =
            project_root.to_str().context("Project root must be valid UTF-8")?.to_owned();
        let read_store = store.clone();
        let read_tenant = tenant.clone();
        let raw_override =
            tokio::task::spawn_blocking(move || read_store.get_project(&read_tenant, &project_key))
                .await
                .context("join project config load task")??
                .map(|payload| {
                    serde_json::from_str(&payload)
                        .context("Failed to parse tenant project config payload")
                })
                .transpose()?
                .unwrap_or_else(|| Value::Object(Map::new()));
        let next_override = prepare_project_override(&base, raw_override, patch, &H::catalog(&base))?;
        let write_store = store;
        let write_tenant = tenant.clone();
        let write_project_root = project_root.to_path_buf();
        tokio::task::spawn_blocking(move || {
            save_project_config_to_store(
                &write_store,
                &write_tenant,
                &write_project_root,
                &next_override,
            )
        })
        .await
        .context("join project config save task")??;
        H::load(tenant, Some(project_root)).await
    }
}

/// 更新默认租户的项目覆盖。
pub async fn update_project_config<H: ProjectConfigHost>(project_root: &Path, patch: Value) -> Result<Config> {
    update_project_config_for_tenant::<H>(&TenantScope::default(), project_root, patch).await
}

/// 根据更新前后的完整配置计算最小项目补丁并持久化。
pub async fn update_project_config_from_configs<H: ProjectConfigHost>(
    project_root: &Path,
    before: &Config,
    after: &Config,
) -> Result<Config> {
    let before_value =
        serde_json::to_value(before).context("Failed to serialize previous config")?;
    let after_value = serde_json::to_value(after).context("Failed to serialize updated config")?;
    let Some(patch) = merge_patch_between(&before_value, &after_value) else {
        return Ok(after.clone());
    };
    update_project_config::<H>(project_root, patch).await
}
