//! Read-only discovery on the authorized execution host; never imports or enables skills.
use std::{collections::BTreeMap, fs, io, path::{Path, PathBuf}};
use vw_api_types::{agent_apps::AppId, app_providers::paths::ConfigPathState};
use super::{
    app_paths::{AppSkillsPath, SkillDistribution, distribution},
    archive::MAX_ARCHIVE_ENTRIES,
    inventory::get_all_installed,
    local_install::{LocalSkill, validate_directory},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillLocation {
    pub app_id: AppId,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct UnmanagedSkill {
    pub directory: String,
    pub name: String,
    pub description: Option<String>,
    pub content_hash: Option<String>,
    /// Identical copies retain every application source.
    pub found_in: Vec<SkillLocation>,
    pub managed: bool,
    pub conflict: bool,
    /// Links are reported without opening their targets, including dangling links.
    pub external_link: bool,
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid or unsafe skill scan source")
}

/// `home` and resolved application paths must come from the selected host contract.
/// Saved records alone establish ownership; a directory name never implies ownership.
/// Includes managed copies for callers to distinguish them from import candidates.
pub fn scan_unmanaged(
    home: &Path,
    ssot: &Path,
    saved: &[LocalSkill],
    targets: &[AppSkillsPath],
) -> io::Result<Vec<UnmanagedSkill>> {
    let installed = get_all_installed(ssot, saved)?;
    let mut rows: Vec<UnmanagedSkill> = Vec::new();
    let mut count = 0;
    for target in targets {
        if target.distribution != distribution(target.app_id) { return Err(invalid()); }
        if target.distribution == SkillDistribution::NoDirectorySync { continue; }
        let root = &target.directory;
        // Revalidate instead of trusting an earlier path observation or its state.
        if !root.is_absolute() || !root.starts_with(home)
            || root.starts_with(ssot) || ssot.starts_with(root)
        { return Err(invalid()); }
        match crate::providers::paths::open_beneath(home, root, true) {
            Ok(_) => {}
            Err(ConfigPathState::Absent) => continue,
            Err(_) => return Err(invalid()),
        }
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            count += 1;
            if count > MAX_ARCHIVE_ENTRIES { return Err(invalid()); }
            let directory = entry.file_name().into_string().map_err(|_| invalid())?;
            if directory.starts_with('.') { continue; }
            validate_directory(&directory).map_err(|_| invalid())?;
            let kind = entry.file_type()?;
            let location = SkillLocation { app_id: target.app_id, path: entry.path() };
            if kind.is_symlink() {
                rows.push(UnmanagedSkill {
                    name: directory.clone(), directory, description: None,
                    content_hash: None, found_in: vec![location], managed: false,
                    conflict: false, external_link: true,
                });
                continue;
            }
            if !kind.is_dir() { continue; }
            let has_document = ["SKILL.md", "SKILL.toml"].iter().try_fold(false, |found, name| {
                match fs::symlink_metadata(entry.path().join(name)) {
                    Ok(_) => Ok(true),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(found),
                    Err(error) => Err(error),
                }
            })?;
            if !has_document { continue; }
            let record = LocalSkill {
                directory: directory.clone(), name: directory.clone(),
                description: None, source: entry.path(),
            };
            let detail = get_all_installed(root, &[record])?.remove(0);
            if !detail.present { return Err(invalid()); }
            let managed = installed.iter().any(|saved| {
                saved.present && saved.skill.directory.eq_ignore_ascii_case(&directory)
                    && saved.content_hash == detail.content_hash
            });
            if let Some(row) = rows.iter_mut().find(|row| {
                !row.external_link && row.directory.eq_ignore_ascii_case(&directory)
                    && row.content_hash == detail.content_hash
            }) {
                if !row.found_in.contains(&location) { row.found_in.push(location); }
            } else {
                rows.push(UnmanagedSkill {
                    directory, name: detail.skill.name, description: detail.skill.description,
                    content_hash: detail.content_hash, found_in: vec![location],
                    managed, conflict: false, external_link: false,
                });
            }
        }
    }
    let mut variants = BTreeMap::<String, usize>::new();
    for row in &rows { *variants.entry(row.directory.to_ascii_lowercase()).or_default() += 1; }
    for row in &mut rows {
        row.conflict = variants[&row.directory.to_ascii_lowercase()] > 1
            || installed.iter().any(|saved| {
                saved.skill.directory.eq_ignore_ascii_case(&row.directory)
                    && (!saved.present || row.external_link || saved.content_hash != row.content_hash)
            });
        row.found_in.sort_by(|a, b| a.path.cmp(&b.path));
    }
    rows.sort_by(|a, b| a.directory.cmp(&b.directory).then(a.content_hash.cmp(&b.content_hash)));
    Ok(rows)
}
