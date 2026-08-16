//! Trust records for reviewed plugin hook declarations.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::hooks::PluginHook;

const TRUST_FILE_NAME: &str = "plugin-hook-trust.json";
const TRUST_VERSION: u32 = 1;

/// Review status for a plugin hook declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginHookTrustStatus {
    Trusted,
    Pending,
}

/// Persistent trust store for plugin hook hashes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHookTrustStore {
    #[serde(default = "default_trust_version")]
    version: u32,
    #[serde(default)]
    trusted_hashes: BTreeSet<String>,
    #[serde(skip)]
    path: PathBuf,
}

impl PluginHookTrustStore {
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self {
                version: TRUST_VERSION,
                trusted_hashes: BTreeSet::new(),
                path: path.to_path_buf(),
            });
        }

        let bytes = fs::read(path).with_context(|| {
            format!("failed to read plugin hook trust store: {}", path.display())
        })?;
        let mut store: Self = serde_json::from_slice(&bytes).with_context(|| {
            format!("failed to parse plugin hook trust store: {}", path.display())
        })?;
        if store.version == 0 {
            store.version = TRUST_VERSION;
        }
        store.path = path.to_path_buf();
        Ok(store)
    }

    pub fn review_status(&self, plugin_name: &str, hook: &PluginHook) -> PluginHookTrustStatus {
        let hash = plugin_hook_trust_hash(plugin_name, hook);
        if self.trusted_hashes.contains(&hash) {
            PluginHookTrustStatus::Trusted
        } else {
            PluginHookTrustStatus::Pending
        }
    }

    pub fn trust_hook(&mut self, plugin_name: &str, hook: &PluginHook) -> Result<()> {
        self.trusted_hashes.insert(plugin_hook_trust_hash(plugin_name, hook));
        self.save()
    }

    /// Remove one previously reviewed declaration by its exact stable hash.
    pub(crate) fn revoke_hash(&mut self, hook_hash: &str) -> Result<bool> {
        let removed = self.trusted_hashes.remove(hook_hash);
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create plugin hook trust dir: {}", parent.display())
            })?;
        }
        let bytes = serde_json::to_vec_pretty(self)
            .context("failed to serialize plugin hook trust store")?;
        fs::write(&self.path, bytes).with_context(|| {
            format!("failed to write plugin hook trust store: {}", self.path.display())
        })
    }
}

/// Stable trust file location under the agent config directory.
pub fn plugin_hook_trust_file(config_dir: &Path) -> PathBuf {
    config_dir.join(TRUST_FILE_NAME)
}

/// Compute a review hash for the security-relevant hook identity.
pub fn plugin_hook_trust_hash(plugin_name: &str, hook: &PluginHook) -> String {
    let working_dir = hook.working_dir.to_string_lossy();
    let manifest_path = hook.manifest_path.to_string_lossy();
    let payload = serde_json::json!({
        "plugin_name": plugin_name,
        "event": hook.event.as_str(),
        "command": hook.command.as_str(),
        "args": &hook.args,
        "working_dir": working_dir.as_ref(),
        "manifest_path": manifest_path.as_ref(),
    });
    let bytes = serde_json::to_vec(&payload).expect("trust hash payload serializes");
    hex::encode(Sha256::digest(bytes))
}

fn default_trust_version() -> u32 {
    TRUST_VERSION
}
