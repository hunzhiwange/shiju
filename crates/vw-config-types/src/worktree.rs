//! Tenant-global settings for host-managed Git worktrees.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct WorktreeSettings {
    /// Empty or whitespace uses the existing task_worktrees_dir; otherwise absolute.
    pub root: String,
    pub fetch_before_create: bool,
    /// Enables cleanup suggestions. Deletion still requires confirmation when
    /// ask_before_delete is true; enabling this alone never authorizes deletion.
    pub auto_cleanup_enabled: bool,
    #[schemars(range(min = 1))]
    pub keep_count: u32,
    pub ask_before_delete: bool,
}

impl Default for WorktreeSettings {
    fn default() -> Self {
        Self {
            root: String::new(),
            fetch_before_create: false,
            auto_cleanup_enabled: true,
            keep_count: 15,
            ask_before_delete: true,
        }
    }
}

impl WorktreeSettings {
    /// Validate before persisting or using host settings.
    pub fn validate(&self) -> Result<(), &'static str> {
        let root = self.root.trim();
        if !root.is_empty() && !Path::new(root).is_absolute() {
            return Err("worktree_settings.root must be empty or an absolute path");
        }
        if self.keep_count == 0 {
            return Err("worktree_settings.keep_count must be greater than 0");
        }
        Ok(())
    }

    /// Resolve without creating directories or expanding shell expressions.
    pub fn resolve_root(&self, home: impl AsRef<Path>) -> Result<PathBuf, &'static str> {
        self.validate()?;
        let root = self.root.trim();
        if root.is_empty() {
            Ok(crate::paths::task_worktrees_dir(home))
        } else {
            Ok(PathBuf::from(root))
        }
    }
}
