//! 技能根归一化和不跟随链接的格式相关目录发现。

use anyhow::{Context, Result, bail};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::super::PluginManifest;
use super::super::audit::resolve_component_path;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillPackageFormat {
    Native,
    Legacy,
    Portable,
}

pub(crate) fn package_format(manifest: &PluginManifest) -> SkillPackageFormat {
    if manifest.manifest_path == manifest.root.join("plugin.json") {
        SkillPackageFormat::Portable
    } else if manifest.manifest_path.parent()
        == Some(manifest.root.join(".vibewindow-plugin").as_path())
    {
        SkillPackageFormat::Native
    } else {
        SkillPackageFormat::Legacy
    }
}

/// 加载时再次核对已审计路径；默认根缺失时不创建或改写原包。
pub(crate) fn skill_roots(manifest: &PluginManifest) -> Result<Vec<PathBuf>> {
    let paths = if manifest.skills.is_empty() {
        match fs::symlink_metadata(manifest.root.join("skills")) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error).context("failed to inspect default plugin skill root"),
            Ok(_) => vec!["./skills"],
        }
    } else {
        manifest.skills.iter().map(|path| path.raw.as_str()).collect()
    };
    let mut roots = BTreeSet::new();
    for raw in paths {
        if package_format(manifest) == SkillPackageFormat::Portable
            && fs::symlink_metadata(manifest.root.join("skills"))
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
        {
            continue;
        }
        let path = resolve_component_path(&manifest.root, "skills", raw)?.canonical_path;
        if !path.is_dir() {
            bail!("plugin skills root must be a directory: {}", path.display());
        }
        roots.insert(path);
    }
    Ok(roots.into_iter().collect())
}

pub(crate) fn skill_files(
    root: &Path,
    format: SkillPackageFormat,
) -> (Vec<PathBuf>, Vec<(PathBuf, String)>) {
    let mut files = BTreeSet::new();
    let mut errors = Vec::new();
    let walker = WalkDir::new(root).follow_links(false).sort_by_file_name();
    let walker = if format == SkillPackageFormat::Portable { walker.max_depth(2) } else { walker };
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                errors.push((error.path().unwrap_or(root).to_path_buf(), error.to_string()));
                continue;
            }
        };
        if entry.file_type().is_symlink() {
            errors.push((
                entry.path().to_path_buf(),
                "plugin skill path must not be a symlink".into(),
            ));
            continue;
        }
        if !entry.file_type().is_file()
            || format == SkillPackageFormat::Portable && entry.depth() != 2
        {
            continue;
        }
        let name = entry.file_name().to_str();
        if name == Some("SKILL.md")
            || format == SkillPackageFormat::Native && name == Some("SKILL.toml")
        {
            files.insert(entry.path().to_path_buf());
        }
    }
    (files.into_iter().collect(), errors)
}
