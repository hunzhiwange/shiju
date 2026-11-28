//! Skills embedded from the pinned create-task-plan-skills submodule.

#[cfg(not(target_arch = "wasm32"))]
use anyhow::{Context, Result};
use include_dir::{Dir, include_dir};
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

pub static BUNDLED_SKILLS: &[(&str, Dir<'static>)] = &[
    (
        "create-task-plans",
        include_dir!(
            "$CARGO_MANIFEST_DIR/../../../skills/create-task-plan-skills/create-task-plans"
        ),
    ),
    (
        "do-task-plans",
        include_dir!("$CARGO_MANIFEST_DIR/../../../skills/create-task-plan-skills/do-task-plans"),
    ),
    (
        "do-task-plans-speed",
        include_dir!(
            "$CARGO_MANIFEST_DIR/../../../skills/create-task-plan-skills/do-task-plans-speed"
        ),
    ),
    (
        "git-commit",
        include_dir!("$CARGO_MANIFEST_DIR/../../../skills/create-task-plan-skills/git-commit"),
    ),
    (
        "rustcodegraph",
        include_dir!("$CARGO_MANIFEST_DIR/../../../skills/create-task-plan-skills/rustcodegraph"),
    ),
    (
        "shiju-cli-workbench",
        include_dir!(
            "$CARGO_MANIFEST_DIR/../../../skills/create-task-plan-skills/shiju-cli-workbench"
        ),
    ),
    (
        "run-task-plan-sessions",
        include_dir!(
            "$CARGO_MANIFEST_DIR/../../../skills/create-task-plan-skills/run-task-plan-sessions"
        ),
    ),
    (
        "skill-creator",
        include_dir!("$CARGO_MANIFEST_DIR/../../../skills/create-task-plan-skills/skill-creator"),
    ),
    (
        "officecli",
        include_dir!("$CARGO_MANIFEST_DIR/../../../skills/create-task-plan-skills/officecli"),
    ),
    (
        "officecli-docx",
        include_dir!("$CARGO_MANIFEST_DIR/../../../skills/create-task-plan-skills/officecli-docx"),
    ),
    (
        "officecli-xlsx",
        include_dir!("$CARGO_MANIFEST_DIR/../../../skills/create-task-plan-skills/officecli-xlsx"),
    ),
    (
        "officecli-pptx",
        include_dir!("$CARGO_MANIFEST_DIR/../../../skills/create-task-plan-skills/officecli-pptx"),
    ),
    (
        "pdf",
        include_dir!("$CARGO_MANIFEST_DIR/../../../skills/create-task-plan-skills/pdf"),
    ),
    (
        "mermaid",
        include_dir!("$CARGO_MANIFEST_DIR/../../../skills/create-task-plan-skills/mermaid"),
    ),
];

pub fn bundled_skill_dir(skill_id: &str) -> Option<&'static Dir<'static>> {
    BUNDLED_SKILLS.iter().find_map(|(id, dir)| (*id == skill_id).then_some(dir))
}

/// Discover packaged files in a per-content cache shared by the gateway and CLI.
#[cfg(not(target_arch = "wasm32"))]
pub fn bundled_skills_root() -> Result<PathBuf> {
    let root = bundled_cache_path();
    if root.is_dir() {
        return Ok(root.to_path_buf());
    }

    let parent = root.parent().context("invalid bundled skills cache path")?;
    std::fs::create_dir_all(parent)?;
    let staging = tempfile::tempdir_in(parent)?;
    for (id, directory) in BUNDLED_SKILLS {
        let target = staging.path().join(id);
        std::fs::create_dir(&target)?;
        copy_bundled_files(directory, &target)?;
    }

    match std::fs::rename(staging.path(), root) {
        Ok(()) => Ok(root.to_path_buf()),
        Err(_) if root.is_dir() => Ok(root.to_path_buf()),
        Err(error) => Err(error).context("could not publish bundled skills cache"),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn bundled_cache_path() -> &'static Path {
    use std::hash::{DefaultHasher, Hash, Hasher};
    use std::sync::LazyLock;

    fn hash_directory(directory: &Dir<'_>, fingerprint: &mut DefaultHasher) {
        for file in directory.files() {
            file.path().hash(fingerprint);
            file.contents().hash(fingerprint);
        }
        for child in directory.dirs() {
            hash_directory(child, fingerprint);
        }
    }

    static ROOT: LazyLock<PathBuf> = LazyLock::new(|| {
        let mut fingerprint = DefaultHasher::new();
        for (id, directory) in BUNDLED_SKILLS {
            id.hash(&mut fingerprint);
            hash_directory(directory, &mut fingerprint);
        }
        agent_paths::paths()
            .cache
            .join("bundled-skills")
            .join(format!("{:016x}", fingerprint.finish()))
    });
    &ROOT
}

/// Verify an exact packaged asset before letting the read tool use the system path.
#[cfg(not(target_arch = "wasm32"))]
pub fn is_bundled_skill_file(path: &Path) -> bool {
    let root = bundled_cache_path();
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    let mut parts = relative.components();
    let Some(std::path::Component::Normal(id)) = parts.next() else {
        return false;
    };
    let relative_file = parts.as_path();
    if !parts.all(|part| matches!(part, std::path::Component::Normal(_))) {
        return false;
    }
    let Some(dir) = id.to_str().and_then(bundled_skill_dir) else {
        return false;
    };
    let Some(file) = dir.get_file(relative_file) else {
        return false;
    };
    let Ok(canonical_root) = root.canonicalize() else {
        return false;
    };
    path.canonicalize().is_ok_and(|resolved| resolved == canonical_root.join(relative))
        && std::fs::symlink_metadata(path).is_ok_and(|metadata| {
            metadata.is_file() && metadata.len() == file.contents().len() as u64
        })
        && std::fs::read(path).is_ok_and(|contents| contents == file.contents())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn install_bundled_skill(
    skills_root: &Path,
    skill_id: &str,
    preloaded_source: Option<&str>,
) -> Result<PathBuf> {
    let dir = bundled_skill_dir(skill_id)
        .with_context(|| format!("unknown bundled skill: {skill_id}"))?;
    std::fs::create_dir_all(skills_root)
        .with_context(|| format!("failed to create {}", skills_root.display()))?;

    let target = skills_root.join(skill_id);
    if target.exists() {
        return Ok(target);
    }

    let staging = tempfile::tempdir_in(skills_root)
        .with_context(|| format!("failed to stage skill in {}", skills_root.display()))?;
    let staged_skill = staging.path().join(skill_id);
    std::fs::create_dir(&staged_skill)?;
    copy_bundled_files(dir, &staged_skill)?;

    if let Some(source) = preloaded_source {
        let metadata = serde_json::json!({
            "slug": skill_id,
            "version": "preloaded",
            "source": source,
        });
        let path = staged_skill.join("_meta.json");
        std::fs::write(&path, serde_json::to_vec_pretty(&metadata)?)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    match std::fs::rename(&staged_skill, &target) {
        Ok(()) => Ok(target),
        Err(_) if target.exists() => Ok(target),
        Err(err) => Err(err).with_context(|| format!("failed to install {}", target.display())),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn copy_bundled_files(source: &Dir<'_>, dest: &Path) -> Result<()> {
    for file in source.files() {
        let name = file.path().file_name().context("invalid bundled file path")?;
        let path = dest.join(name);
        std::fs::write(&path, file.contents())
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    for dir in source.dirs() {
        let name = dir.path().file_name().context("invalid bundled directory path")?;
        let path = dest.join(name);
        std::fs::create_dir(&path)
            .with_context(|| format!("failed to create {}", path.display()))?;
        copy_bundled_files(dir, &path)?;
    }

    Ok(())
}
