//! Selective shell cleanup owns receipts and private backups on the execution host.
use std::path::Path;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{env_clean::*, env_diagnose::EnvDiagnosis},
};

/// Bind receipts to the authorized tenant/host identity, independently of file coordinates.
pub fn diagnose(home: &Path, app: AppId, scope: &str) -> Result<EnvDiagnosis, &'static str> {
    let report = super::env_diagnose::diagnose(home, app)?;
    #[cfg(target_os = "macos")]
    {
        macos::issue(home, app, scope, report)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = scope;
        Ok(report)
    }
}
pub fn clean(
    home: &Path,
    private_root: &Path,
    app: AppId,
    scope: &str,
    selections: &[EnvCleanSelection],
) -> Result<EnvCleanResult, &'static str> {
    #[cfg(target_os = "macos")]
    {
        macos::clean(home, private_root, app, scope, selections)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (home, private_root, app, scope, selections);
        Err("unsupported environment platform")
    }
}

#[cfg(target_os = "macos")]
pub(super) mod macos {
    use super::*;
    use crate::providers::env_backup::{self, Backup, BackupFile, base, identity, random_id, safe};
    use crate::providers::env_diagnose::macos::{open_root, read_source, scan};
    use std::{
        collections::BTreeMap,
        os::unix::fs::MetadataExt,
        sync::{Mutex, OnceLock},
        time::{Duration, Instant},
    };
    use vw_api_types::app_providers::env_diagnose::*;
    const UNAVAILABLE: &str = "environment cleanup unavailable";
    const CONFLICT: &str = "environment diagnosis changed";
    #[derive(Clone)]
    struct Receipt {
        scope: String,
        app: AppId,
        root: (u64, u64),
        file: UserShellFile,
        line: usize,
        base: String,
        issued: Instant,
    }
    fn receipts() -> &'static Mutex<BTreeMap<String, Receipt>> {
        static RECEIPTS: OnceLock<Mutex<BTreeMap<String, Receipt>>> = OnceLock::new();
        RECEIPTS.get_or_init(Default::default)
    }
    pub(crate) fn issue(
        home: &Path,
        app: AppId,
        scope: &str,
        mut report: EnvDiagnosis,
    ) -> Result<EnvDiagnosis, &'static str> {
        let root = open_root(home)?;
        let m = root.metadata().map_err(|_| UNAVAILABLE)?;
        let mut cache = receipts().lock().map_err(|_| UNAVAILABLE)?;
        cache.retain(|_, r| r.issued.elapsed() < Duration::from_secs(900));
        for file in [
            UserShellFile::Bashrc,
            UserShellFile::BashProfile,
            UserShellFile::Zshrc,
            UserShellFile::Zprofile,
            UserShellFile::Profile,
        ] {
            let entries: Vec<_> = report
                .entries
                .iter_mut()
                .filter(|e| matches!(e.source, EnvSource::UserShell { file: f, .. } if f == file))
                .collect();
            if !entries.iter().any(|e| e.cleanup_available) {
                continue;
            }
            let (content, metadata) = read_source(&root, file.filename())?.ok_or(CONFLICT)?;
            let mut current = Vec::new();
            scan(&content, app, |line| EnvSource::UserShell { file, line }, true, &mut current)?;
            if current != entries.iter().map(|e| (**e).clone()).collect::<Vec<_>>() {
                return Err(CONFLICT);
            }
            let digest = base(&metadata, &content);
            for entry in entries {
                if !safe(&m) || !safe(&metadata) || metadata.nlink() != 1 {
                    entry.cleanup_available = false;
                    continue;
                }
                if cache.len() >= 16384 {
                    return Err(UNAVAILABLE);
                }
                let id = random_id()?;
                let EnvSource::UserShell { line, .. } = entry.source else {
                    return Err(CONFLICT);
                };
                cache.insert(
                    id.clone(),
                    Receipt {
                        scope: scope.into(),
                        app,
                        root: identity(&m),
                        file,
                        line,
                        base: digest.clone(),
                        issued: Instant::now(),
                    },
                );
                entry.cleanup =
                    Some(EnvCleanSelection { diagnostic_id: id, base_digest: digest.clone() });
            }
        }
        Ok(report)
    }
    pub(super) fn clean(
        home: &Path,
        private_root: &Path,
        app: AppId,
        scope: &str,
        selections: &[EnvCleanSelection],
    ) -> Result<EnvCleanResult, &'static str> {
        execute(home, private_root, app, scope, selections, |_| Ok(()))
    }
    /// The observer is an internal failure boundary used to prove backup-before-write ordering.
    pub(crate) fn execute(
        home: &Path,
        private_root: &Path,
        app: AppId,
        scope: &str,
        selections: &[EnvCleanSelection],
        after_backup: impl FnOnce(&str) -> Result<(), &'static str>,
    ) -> Result<EnvCleanResult, &'static str> {
        use crate::providers::live::write_guard::{FilePrivacy, lock_directory};
        use std::{collections::BTreeSet, ffi::OsStr, os::fd::AsRawFd};
        if !matches!(app, AppId::Claude | AppId::Codex | AppId::Gemini | AppId::GrokBuild) {
            return Err("unsupported environment application");
        }
        if selections.is_empty() || selections.len() > 256 {
            return Err("invalid environment selection");
        }
        let root = open_root(home)?;
        let root_metadata = root.metadata().map_err(|_| UNAVAILABLE)?;
        if !safe(&root_metadata) {
            return Err(UNAVAILABLE);
        }
        let root_id = identity(&root_metadata);
        let mut guard = lock_directory(app, root.try_clone().map_err(|_| UNAVAILABLE)?)
            .map_err(|_| UNAVAILABLE)?;
        // Cross-process cleanup cooperates on the physical HOME directory, without a user file lock.
        // SAFETY: root is retained through all writes; close releases flock on every exit path.
        if unsafe { libc::flock(root.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(CONFLICT);
        }
        let mut cache = receipts().lock().map_err(|_| UNAVAILABLE)?;
        let mut selected = BTreeSet::new();
        let mut groups: BTreeMap<&str, Vec<Receipt>> = BTreeMap::new();
        for selection in selections {
            if !selected.insert(&selection.diagnostic_id) {
                return Err("invalid environment selection");
            }
            let r = cache.get(&selection.diagnostic_id).ok_or(CONFLICT)?;
            if r.scope != scope
                || r.app != app
                || r.root != root_id
                || r.base != selection.base_digest
                || r.issued.elapsed() >= Duration::from_secs(900)
            {
                return Err(CONFLICT);
            }
            groups.entry(r.file.filename()).or_default().push(r.clone());
        }
        let mut files = Vec::new();
        for (name, definitions) in &groups {
            let (original, metadata) = read_source(&root, name)?.ok_or(CONFLICT)?;
            if !safe(&metadata) || metadata.nlink() != 1 {
                return Err(CONFLICT);
            }
            let digest = base(&metadata, &original);
            if definitions.iter().any(|r| r.base != digest) {
                return Err(CONFLICT);
            }
            let lines: BTreeSet<_> = definitions.iter().map(|r| r.line).collect();
            if lines.len() != definitions.len() {
                return Err("invalid environment selection");
            }
            let replacement: String = original
                .split_inclusive('\n')
                .enumerate()
                .map(
                    |(index, line)| {
                        if lines.contains(&(index + 1)) { retained_comment(line) } else { line }
                    },
                )
                .collect();
            files.push(BackupFile {
                file: definitions[0].file,
                base_digest: digest,
                original,
                replacement,
                cleaned_digest: None,
                restored_digest: None,
            });
        }
        let current_home = || -> Result<(), &'static str> {
            let metadata = open_root(home)?.metadata().map_err(|_| UNAVAILABLE)?;
            if identity(&metadata) != root_id || !safe(&metadata) {
                return Err(CONFLICT);
            }
            Ok(())
        };
        let validate = |f: &BackupFile| -> Result<(), &'static str> {
            current_home()?;
            let (bytes, m) = read_source(&root, f.file.filename())?.ok_or(CONFLICT)?;
            if base(&m, &bytes) != f.base_digest {
                return Err(CONFLICT);
            }
            Ok(())
        };
        for file in &files {
            validate(file)?;
        }
        let id = random_id()?;
        let directory = env_backup::directory(private_root, true)?;
        let mut backup =
            Backup { version: 2, scope: scope.into(), app_id: app, home_identity: root_id, files };
        env_backup::store(&directory, &id, &backup, false)?;
        after_backup(&id)?;
        // Check the complete selection again after the durable backup, before any publication.
        for file in &backup.files {
            validate(file)?;
        }
        for index in 0..backup.files.len() {
            let file = &backup.files[index];
            validate(file)?;
            guard
                .write(
                    OsStr::new(file.file.filename()),
                    file.replacement.as_bytes(),
                    FilePrivacy::Shared,
                )
                .map_err(|_| UNAVAILABLE)?;
            root.sync_all().map_err(|_| UNAVAILABLE)?;
            let digest = env_backup::fingerprint(&root, file.file.filename(), &file.replacement)?;
            backup.files[index].cleaned_digest = Some(digest);
            env_backup::store(&directory, &id, &backup, true)?;
        }
        for selection in selections {
            cache.remove(&selection.diagnostic_id);
        }
        Ok(EnvCleanResult { backup_id: id, removed_definitions: selections.len() })
    }
    // Diagnosed files contain only literal assignments. Preserve trailing comments verbatim.
    fn retained_comment(line: &str) -> &str {
        let mut quote = None;
        let mut previous_space = false;
        for (index, ch) in line.char_indices() {
            if quote == Some(ch) {
                quote = None;
            } else if quote.is_none() {
                if ch == '#' && previous_space {
                    return &line[index..];
                }
                if ch == '\'' || ch == '"' {
                    quote = Some(ch);
                }
            }
            previous_space = ch == ' ' || ch == '\t';
        }
        ""
    }
}
