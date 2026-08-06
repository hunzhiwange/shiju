use anyhow::{Context, Result, bail};
#[cfg(not(target_arch = "wasm32"))]
use directories::UserDirs;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
#[cfg(not(target_arch = "wasm32"))]
use std::fs::{File, OpenOptions};
#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicU64, Ordering};
use vw_api_types::id::TenantScope;
use vw_config_types::config::Config;

#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::config::SystemConfigStore;
use crate::app::agent::config::schema::CONFIG_OVERRIDE_FILENAME;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::config::schema::config_io::{
    merge_config_payload, read_project_config_override,
};
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
static PROJECT_OVERRIDE_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[cfg(not(target_arch = "wasm32"))]
struct TemporaryOverrideFile {
    path: PathBuf,
    keep: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for TemporaryOverrideFile {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

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
        #[cfg(test)]
        if config.config_path.file_name().is_some_and(|name| name == ".vibewindow.json")
            && config.config_path.parent().is_some_and(Path::is_dir)
        {
            validate_config(config)?;
            let payload = serde_json::to_vec_pretty(config)
                .context("Failed to serialize test runtime config")?;
            tokio::fs::write(&config.config_path, payload)
                .await
                .with_context(|| format!("Failed to write {}", config.config_path.display()))?;
            return Ok(());
        }

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
fn open_temporary_override(project_root: &Path) -> Result<(File, TemporaryOverrideFile)> {
    for _ in 0..32 {
        let sequence = PROJECT_OVERRIDE_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = project_root
            .join(format!(".{CONFIG_OVERRIDE_FILENAME}.tmp-{}-{sequence}", std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok((file, TemporaryOverrideFile { path, keep: false })),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).context("Failed to create project config override temp file");
            }
        }
    }
    bail!("Failed to allocate project config override temp file");
}

/// 原子写入显式项目根目录的 raw 配置覆盖。
pub(super) fn save_project_config_override(project_root: &Path, payload: &Value) -> Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (project_root, payload);
        bail!("Project config overrides are not supported in WASM");
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if !payload.is_object() {
            bail!("project config override root must be an object");
        }
        if !project_root.is_dir() {
            bail!("project root must be an existing directory");
        }

        let destination = project_root.join(CONFIG_OVERRIDE_FILENAME);
        let mut contents = serde_json::to_vec_pretty(payload)
            .context("Failed to serialize project config override")?;
        contents.push(b'\n');
        let (mut file, mut temporary) = open_temporary_override(project_root)?;
        file.write_all(&contents).context("Failed to write project config override temp file")?;
        file.sync_all().context("Failed to fsync project config override temp file")?;
        drop(file);

        std::fs::rename(&temporary.path, &destination)
            .context("Failed to atomically replace project config override")?;
        temporary.keep = true;

        #[cfg(unix)]
        File::open(project_root)
            .and_then(|directory| directory.sync_all())
            .context("Failed to fsync project config override directory")?;
        Ok(())
    }
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

pub(crate) async fn update_project_config(project_root: &Path, patch: Value) -> Result<Config> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (project_root, patch);
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
            return load_or_init_config_for_tenant(&TenantScope::default(), Some(project_root))
                .await;
        }
        let tenant = TenantScope::default();
        let base = load_or_init_config_for_tenant(&tenant, None).await?;
        let override_path = project_root.join(CONFIG_OVERRIDE_FILENAME);
        let raw_override = read_project_config_override(&override_path)
            .await?
            .unwrap_or_else(|| Value::Object(Map::new()));
        let next_override = prepare_project_override(&base, raw_override, patch)?;
        save_project_config_override(project_root, &next_override)?;
        load_or_init_config_for_tenant(&tenant, Some(project_root)).await
    }
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
