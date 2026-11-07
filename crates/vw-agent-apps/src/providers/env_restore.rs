//! Restores only host-bound cleanup backups, preserving subsequent external edits.
use std::path::Path;
use vw_api_types::{agent_apps::AppId, app_providers::env_restore::*};
pub fn restore(
    home: &Path,
    private_root: &Path,
    app: AppId,
    scope: &str,
    backup_id: &str,
) -> Result<EnvRestoreResult, &'static str> {
    #[cfg(target_os = "macos")]
    {
        macos::execute(home, private_root, app, scope, backup_id, || {})
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (home, private_root, app, scope, backup_id);
        Err("unsupported environment platform")
    }
}
#[cfg(target_os = "macos")]
pub(super) mod macos {
    use super::*;
    use crate::providers::{
        env_backup::{self, base, fingerprint, identity, safe},
        env_diagnose::macos::{open_root, read_source},
        live::write_guard::{FilePrivacy, lock_directory},
    };
    use std::{
        collections::BTreeSet,
        ffi::OsStr,
        os::{fd::AsRawFd, unix::fs::MetadataExt},
    };
    const CONFLICT: &str = "environment backup changed";
    const UNAVAILABLE: &str = "environment restoration unavailable";
    pub(crate) fn execute(
        home: &Path,
        private_root: &Path,
        app: AppId,
        scope: &str,
        id: &str,
        before_publish: impl FnOnce(),
    ) -> Result<EnvRestoreResult, &'static str> {
        if !matches!(app, AppId::Claude | AppId::Codex | AppId::Gemini | AppId::GrokBuild) {
            return Err("unsupported environment application");
        }
        if !env_backup::valid_id(id) {
            return Err("invalid environment backup identifier");
        }
        let root = open_root(home)?;
        let m = root.metadata().map_err(|_| UNAVAILABLE)?;
        if !safe(&m) {
            return Err(UNAVAILABLE);
        }
        let root_id = identity(&m);
        let mut guard = lock_directory(app, root.try_clone().map_err(|_| UNAVAILABLE)?)
            .map_err(|_| UNAVAILABLE)?;
        // SAFETY: the physical HOME descriptor stays live throughout restoration.
        if unsafe { libc::flock(root.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(CONFLICT);
        }
        let directory = env_backup::directory(private_root, false)?;
        let mut backup = env_backup::load(&directory, id)?;
        if backup.version != 2
            || backup.scope != scope
            || backup.app_id != app
            || backup.home_identity != root_id
            || backup.files.is_empty()
            || backup.files.len() > 5
        {
            return Err(CONFLICT);
        }
        let mut files = BTreeSet::new();
        for f in &backup.files {
            if !files.insert(f.file.filename())
                || f.original.len() > 1024 * 1024
                || f.replacement.len() > 1024 * 1024
            {
                return Err(CONFLICT);
            }
        }
        before_publish();
        let mut items = Vec::new();
        for index in 0..backup.files.len() {
            let outcome = (|| {
                let f = &backup.files[index];
                let current = open_root(home)?.metadata().map_err(|_| UNAVAILABLE)?;
                if identity(&current) != root_id || !safe(&current) {
                    return Ok(EnvRestoreStatus::Conflict);
                }
                let Some((bytes, metadata)) = read_source(&root, f.file.filename())? else {
                    return Ok(EnvRestoreStatus::Conflict);
                };
                if !safe(&metadata) || metadata.nlink() != 1 {
                    return Ok(EnvRestoreStatus::Conflict);
                }
                let digest = base(&metadata, &bytes);
                if let Some(restored) = &f.restored_digest {
                    return Ok(if digest == *restored && bytes == f.original {
                        EnvRestoreStatus::AlreadyRestored
                    } else {
                        EnvRestoreStatus::Conflict
                    });
                }
                let Some(cleaned) = &f.cleaned_digest else {
                    return Ok(if digest == f.base_digest && bytes == f.original {
                        EnvRestoreStatus::NotCleaned
                    } else {
                        EnvRestoreStatus::Conflict
                    });
                };
                if digest != *cleaned || bytes != f.replacement {
                    return Ok(EnvRestoreStatus::Conflict);
                }
                guard
                    .write(
                        OsStr::new(f.file.filename()),
                        f.original.as_bytes(),
                        FilePrivacy::Shared,
                    )
                    .map_err(|_| UNAVAILABLE)?;
                root.sync_all().map_err(|_| UNAVAILABLE)?;
                let restored = fingerprint(&root, f.file.filename(), &f.original)?;
                backup.files[index].restored_digest = Some(restored);
                env_backup::store(&directory, id, &backup, true)?;
                Ok::<_, &'static str>(EnvRestoreStatus::Restored)
            })();
            items.push(EnvRestoreItem {
                file: backup.files[index].file,
                status: outcome.unwrap_or(EnvRestoreStatus::Unavailable),
            });
        }
        Ok(EnvRestoreResult { items })
    }
}
