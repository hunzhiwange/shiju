//! Managed installation receipt creation.
use std::{
    io::Write,
    path::{Path, PathBuf},
};
pub(crate) use app_discovery::local_agy::owns;
use app_discovery::local_agy::{digest, private, private_directory};

pub(super) fn validate(prefix: &Path) -> Result<(), String> {
    let directory = private_directory(prefix)?;
    let bin = prefix.join("bin");
    if bin.canonicalize().map_err(|_| "managed bin unavailable")? != bin
        || !std::fs::symlink_metadata(&bin).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
        || (directory.symlink_metadata().is_ok() && !private(&directory, true))
    {
        return Err("private managed agy paths required".into());
    }
    Ok(())
}
pub(super) fn home(prefix: &Path) -> Result<PathBuf, String> {
    validate(prefix)?;
    let directory = private_directory(prefix)?;
    if !directory.exists() {
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&directory).map_err(|_| "private agy directory unavailable")?;
    }
    if !private(&directory, true) {
        return Err("private agy directory required".into());
    }
    Ok(directory)
}
/// Called only after this executor's official install/update succeeds and passes probing.
pub(super) fn record(prefix: &Path) -> Result<(), String> {
    let directory = home(prefix)?;
    let hash = digest(&prefix.join("bin/agy"))?;
    let mut file =
        tempfile::NamedTempFile::new_in(&directory).map_err(|_| "agy receipt unavailable")?;
    writeln!(file, "{hash}").map_err(|_| "agy receipt write failed")?;
    file.as_file().sync_all().map_err(|_| "agy receipt sync failed")?;
    file.persist(directory.join("receipt")).map_err(|_| "agy receipt commit failed")?;
    Ok(())
}
