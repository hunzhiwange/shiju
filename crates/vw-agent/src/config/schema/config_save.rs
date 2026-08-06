use anyhow::{Context, Result, bail};
#[cfg(not(target_arch = "wasm32"))]
use directories::UserDirs;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::Path;
use vw_api_types::id::TenantScope;
use vw_config_types::config::Config;

#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::config::SystemConfigStore;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::config::schema::config_io::merge_config_payload;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::config::schema::config_load::load_or_init_config_for_tenant;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::config::schema::config_secrets::decrypt_config_secrets;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::config::schema::config_secrets::encrypt_config_secrets;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::config::schema::config_validate::validate_config;
#[cfg(not(target_arch = "wasm32"))]
use crate::global_database::GlobalDatabase;

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn save_config_to_store(
    config: &Config,
    tenant: &TenantScope,
    store: &SystemConfigStore,
) -> Result<()> {
    let mut config_to_save = config.clone();
    config_to_save.gateway.skeys.clear();
    validate_config(&config_to_save)?;
    encrypt_config_secrets(&mut config_to_save)?;
    let payload =
        serde_json::to_string(&config_to_save).context("Failed to serialize system config")?;
    store.upsert(tenant, &payload)
}

pub async fn save_config_for_tenant(config: &Config, tenant: &TenantScope) -> Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (config, tenant);
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let home = UserDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .context("Could not find home directory")?;
        #[cfg(test)]
        {
            let store = SystemConfigStore::new(GlobalDatabase::for_home(home)?)?;
            return save_config_to_store(config, tenant, &store);
        }

        #[cfg(not(test))]
        let config = config.clone();
        #[cfg(not(test))]
        let tenant = tenant.clone();
        #[cfg(not(test))]
        tokio::task::spawn_blocking(move || {
            let store = SystemConfigStore::new(GlobalDatabase::for_home(home)?)?;
            save_config_to_store(&config, &tenant, &store)
        })
        .await
        .context("join system config save task")?
    }
}

pub async fn save_config(config: &Config) -> Result<()> {
    save_config_for_tenant(config, &TenantScope::default()).await
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn save_project_config_to_store(
    store: &SystemConfigStore,
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

#[cfg(not(target_arch = "wasm32"))]
async fn open_system_config_store() -> Result<SystemConfigStore> {
    let home = UserDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    let database = GlobalDatabase::for_home(home)?;
    tokio::task::spawn_blocking(move || SystemConfigStore::new(database))
        .await
        .context("join system config store initialization task")?
}

pub(super) fn merge_raw_project_patch(target: &mut Value, patch: Value) {
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

pub(super) fn select_override_shape(shape: &Value, encrypted_effective: Option<&Value>) -> Value {
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
pub(super) fn prepare_project_override(
    base: &Config,
    raw_override: Value,
    patch: Value,
) -> Result<Value> {
    let mut next_override = raw_override;
    merge_raw_project_patch(&mut next_override, patch);
    if let Some(gateway) = next_override.get_mut("gateway").and_then(Value::as_object_mut) {
        gateway.remove("skeys");
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
    Ok(select_override_shape(&next_override, Some(&encrypted_effective)))
}

pub(super) fn merge_patch_between(before: &Value, after: &Value) -> Option<Value> {
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

pub(crate) async fn update_project_config_for_tenant(
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
        if !project_root.is_dir() {
            bail!("project root must be an existing directory");
        }
        if patch_object.is_empty() {
            return load_or_init_config_for_tenant(tenant, Some(project_root)).await;
        }
        let base = load_or_init_config_for_tenant(tenant, None).await?;
        let store = open_system_config_store().await?;
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
        let next_override = prepare_project_override(&base, raw_override, patch)?;
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
        load_or_init_config_for_tenant(tenant, Some(project_root)).await
    }
}

pub(crate) async fn update_project_config(project_root: &Path, patch: Value) -> Result<Config> {
    update_project_config_for_tenant(&TenantScope::default(), project_root, patch).await
}

pub(crate) async fn update_project_config_from_configs(
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
    update_project_config(project_root, patch).await
}
