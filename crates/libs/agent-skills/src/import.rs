//! Import explicitly selected local sources into the native skills catalog.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use config_skills::SkillSecurityConfig;

use crate::installer::{copy_dir_recursive_secure, enforce_skill_security_audit};
use crate::loader::{load_skill_toml, read_markdown_skill_metadata};
use crate::marketplace::install::{ensure_skills_root, ensure_target_absent};
use crate::types::SkillLoadMode;

pub fn import_skill_folder(
    source: &Path,
    skills_root: &Path,
    security: &SkillSecurityConfig,
) -> Result<()> {
    let name = source.file_name().and_then(|name| name.to_str()).context("无效的技能目录名")?;
    import_directory(source, name, skills_root, security)
}

/// Report per-skill failures so that successful imports remain visible after a partial import.
pub fn import_skill_zip(
    source: &Path,
    skills_root: &Path,
    security: &SkillSecurityConfig,
) -> Result<(usize, Vec<String>)> {
    ensure!(fs::symlink_metadata(source)?.is_file(), "请选择 ZIP 文件");
    let extracted =
        app_skill_catalog::archive::extract_local_zip(source).context("无法解包 ZIP")?;
    let mut candidates = Vec::new();
    let mut entries = walkdir::WalkDir::new(extracted.path()).sort_by_file_name().into_iter();
    while let Some(entry) = entries.next() {
        let entry = entry?;
        if !entry.file_type().is_dir() {
            continue;
        }
        let name = if entry.depth() == 0 { source.file_stem() } else { Some(entry.file_name()) };
        let name = name.and_then(|name| name.to_str()).unwrap_or_default();
        if entry.depth() > 0 && (name.starts_with('.') || name == "__MACOSX") {
            entries.skip_current_dir();
        } else if skill_manifest(entry.path()).is_some() {
            candidates.push((entry.path().to_owned(), name.to_owned()));
            entries.skip_current_dir();
        }
    }
    ensure!(!candidates.is_empty(), "ZIP 中没有包含 SKILL.md 或 SKILL.toml 的技能目录");

    let mut installed = 0;
    let mut errors = Vec::new();
    for (directory, name) in candidates {
        match import_directory(&directory, &name, skills_root, security) {
            Ok(()) => installed += 1,
            Err(error) => errors.push(format!("{name}：{error:#}")),
        }
    }
    Ok((installed, errors))
}

fn import_directory(
    source: &Path,
    name: &str,
    skills_root: &Path,
    security: &SkillSecurityConfig,
) -> Result<()> {
    ensure!(
        !name.is_empty()
            && !name.starts_with('.')
            && !name.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':')),
        "无效的技能目录名"
    );
    ensure!(fs::symlink_metadata(source)?.is_dir(), "请选择技能文件夹");
    let manifest = skill_manifest(source).context("技能文件夹根目录缺少 SKILL.md 或 SKILL.toml")?;
    ensure_skills_root(skills_root)?;
    let skills_root = skills_root.canonicalize()?;
    let source = source.canonicalize()?;
    ensure!(!skills_root.starts_with(&source), "来源文件夹包含目标技能目录");
    let target = skills_root.join(name);
    ensure_target_absent(&target)?;

    // Discovery walks the skills root recursively, so stage beside it.
    let parent = skills_root.parent().context("无效的目标技能目录")?;
    let staging = tempfile::Builder::new().prefix(".skill-import-").tempdir_in(parent)?;
    let skill = staging.path().join(name);
    copy_dir_recursive_secure(&source, &skill)?;
    if manifest == "SKILL.toml" {
        load_skill_toml(&skill.join(manifest), SkillLoadMode::Full)?;
    } else {
        read_markdown_skill_metadata(&skill.join(manifest))?;
    }
    enforce_skill_security_audit(&skill, security)
        .map_err(|_| anyhow::anyhow!("技能安全审计未通过"))?;

    ensure_target_absent(&target)?;
    fs::rename(&skill, &target).context("无法写入目标技能目录")
}

fn skill_manifest(directory: &Path) -> Option<&'static str> {
    ["SKILL.toml", "SKILL.md"].into_iter().find(|name| {
        fs::symlink_metadata(directory.join(name)).is_ok_and(|metadata| metadata.is_file())
    })
}
