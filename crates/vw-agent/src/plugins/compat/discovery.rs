//! Identify supported package entries without traversing unrelated source directories.

use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

use super::super::audit::canonical_plugin_root;
use super::portable_manifest::SCHEMA_PREFIX;

/// Ordinary root JSON does not make a directory a plugin; unsafe selected entries still fail.
pub(crate) fn is_plugin_candidate(root: &Path) -> Result<bool> {
    canonical_plugin_root(root)?;
    let portable = root.join("plugin.json");
    match fs::symlink_metadata(&portable) {
        Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
            return Ok(true);
        }
        Ok(_) => {
            let bytes = fs::read(&portable).with_context(|| {
                format!("failed to read plugin manifest: {}", portable.display())
            })?;
            if serde_json::from_slice::<Value>(&bytes).ok().is_some_and(|value| {
                value
                    .get("$schema")
                    .and_then(Value::as_str)
                    .is_some_and(|schema| schema.starts_with(SCHEMA_PREFIX))
            }) {
                return Ok(true);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| {
                format!("failed to inspect plugin manifest: {}", portable.display())
            });
        }
    }

    for directory in [".vibewindow-plugin", ".codex-plugin", ".claude-plugin", ".cursor-plugin"] {
        let parent = root.join(directory);
        match fs::symlink_metadata(&parent) {
            Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
                return Ok(true);
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("failed to inspect plugin manifest directory: {}", parent.display())
                });
            }
        }
        let manifest = parent.join("plugin.json");
        match fs::symlink_metadata(&manifest) {
            Ok(_) => return Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("failed to inspect plugin manifest: {}", manifest.display())
                });
            }
        }
    }
    Ok(false)
}
