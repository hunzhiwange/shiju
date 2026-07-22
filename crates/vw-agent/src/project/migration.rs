//! 旧项目 JSON 注册表到共享 SQLite 的迁移。

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vw_api_types::id::TenantScope;
use vw_config_types::paths::home_config_dir;
use vw_shared::project::Info;

use super::store::ProjectStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationOutcome {
    pub source_count: usize,
    pub imported_count: usize,
    pub backed_up: bool,
}

pub fn migrate_project_registry(home: &Path, store: &ProjectStore) -> Result<MigrationOutcome> {
    let config_dir = home_config_dir(home);
    let source_dir = config_dir.join("storage").join("project");
    if !source_dir.exists() {
        return Ok(MigrationOutcome { source_count: 0, imported_count: 0, backed_up: false });
    }
    if !source_dir.is_dir() {
        bail!("project registry source is not a directory: {}", source_dir.display());
    }

    let backup_dir = config_dir.join("database").join("migration-backups").join("projects");
    if backup_dir.exists() {
        bail!("project registry backup already exists: {}", backup_dir.display());
    }

    let sources = parse_all_sources(&source_dir)?;
    let projects = sources.iter().map(|source| source.info.clone()).collect::<Vec<_>>();
    let imported_count = store.import_exact_registry(&TenantScope::default(), &projects)?;

    let backup_parent =
        backup_dir.parent().context("project registry backup path has no parent")?;
    fs::create_dir_all(backup_parent).with_context(|| {
        format!("create project registry backup directory: {}", backup_parent.display())
    })?;
    fs::rename(&source_dir, &backup_dir).with_context(|| {
        format!(
            "move project registry into backup: {} -> {}",
            source_dir.display(),
            backup_dir.display()
        )
    })?;

    Ok(MigrationOutcome { source_count: sources.len(), imported_count, backed_up: true })
}

fn parse_all_sources(source_dir: &Path) -> Result<Vec<ProjectSource>> {
    let mut paths = fs::read_dir(source_dir)
        .with_context(|| format!("read project registry directory: {}", source_dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.retain(|path| {
        path.is_file() && path.extension().is_some_and(|extension| extension == "json")
    });
    paths.sort();

    let mut ids = HashSet::new();
    let mut sources = Vec::with_capacity(paths.len());
    for path in paths {
        let file_stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .filter(|value| !value.trim().is_empty())
            .with_context(|| format!("project registry filename is invalid: {}", path.display()))?;
        let bytes = fs::read(&path)
            .with_context(|| format!("read project registry source: {}", path.display()))?;
        let info: Info = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse project registry source: {}", path.display()))?;
        if info.id != file_stem {
            bail!(
                "project registry filename id does not match payload: {} != {}",
                file_stem,
                info.id
            );
        }
        ensure_unique_project_id(&mut ids, &info)?;
        sources.push(ProjectSource { _path: path, info });
    }
    Ok(sources)
}

fn ensure_unique_project_id(ids: &mut HashSet<String>, info: &Info) -> Result<()> {
    if !ids.insert(info.id.clone()) {
        bail!("duplicate project id in registry sources: {}", info.id);
    }
    Ok(())
}

struct ProjectSource {
    _path: PathBuf,
    info: Info,
}

#[cfg(test)]
#[path = "migration_tests.rs"]
mod migration_tests;
