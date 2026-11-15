//! Local skill disable marker shared by loading and replay.
use std::path::{Path, PathBuf};

pub fn local_skill_disabled_marker_path(skill_dir: &Path) -> PathBuf {
    skill_dir.join("SKILL.disabled")
}

pub fn is_local_skill_disabled(skill_dir: &Path) -> bool {
    local_skill_disabled_marker_path(skill_dir).is_file()
}
