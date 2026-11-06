//! Provenance for omp installed by the authorized local lifecycle executor.
//! A manual binary or a changed artifact never acquires ownership from its path alone.
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

fn private_directory(prefix: &Path) -> Result<PathBuf, String> {
    let directory = prefix.join(".shiju-omp");
    if prefix.canonicalize().map_err(|_| "managed prefix unavailable")? != prefix {
        return Err("managed prefix must be canonical".into());
    }
    for ancestor in directory.ancestors().skip(1) {
        if !std::fs::symlink_metadata(ancestor)
            .is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
        {
            return Err("managed prefix must not traverse symlinks".into());
        }
    }
    Ok(directory)
}
#[cfg(unix)]
fn private(path: &Path, directory: bool) -> bool {
    use std::os::unix::fs::MetadataExt;
    std::fs::symlink_metadata(path).is_ok_and(|m| {
        // SAFETY: geteuid has no arguments or side effects.
        m.uid() == unsafe { libc::geteuid() }
            && m.mode() & 0o077 == 0
            && if directory { m.is_dir() } else { m.is_file() }
    })
}
#[cfg(not(unix))]
fn private(_path: &Path, _directory: bool) -> bool {
    false
}

pub(super) fn validate(prefix: &Path) -> Result<(), String> {
    let directory = private_directory(prefix)?;
    let bin = prefix.join("bin");
    if bin.canonicalize().map_err(|_| "managed bin unavailable")? != bin
        || !std::fs::symlink_metadata(&bin).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
        || (directory.symlink_metadata().is_ok() && !private(&directory, true))
    {
        return Err("private managed omp paths required".into());
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
        builder.create(&directory).map_err(|_| "private omp directory unavailable")?;
    }
    if !private(&directory, true) {
        return Err("private omp directory required".into());
    }
    Ok(directory)
}
fn digest(binary: &Path) -> Result<String, String> {
    let prefix = binary.parent().and_then(Path::parent).ok_or("managed binary path required")?;
    private_directory(prefix)?;
    if binary != prefix.join("bin/omp")
        || binary.canonicalize().map_err(|_| "managed binary unavailable")? != binary
        || !crate::apps::uninstall::native::is_native_executable(binary)
    {
        return Err("regular native omp binary required".into());
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options.open(binary).map_err(|_| "managed binary unavailable")?;
    let mut hash = Sha256::new();
    let mut bytes = [0; 65536];
    loop {
        let count = file.read(&mut bytes).map_err(|_| "managed binary read failed")?;
        if count == 0 {
            break;
        }
        hash.update(&bytes[..count]);
    }
    Ok(hash.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}
/// Called only after this executor's official install/update succeeds and passes probing.
pub(super) fn record(prefix: &Path) -> Result<(), String> {
    let directory = home(prefix)?;
    let hash = digest(&prefix.join("bin/omp"))?;
    let mut file =
        tempfile::NamedTempFile::new_in(&directory).map_err(|_| "omp receipt unavailable")?;
    writeln!(file, "{hash}").map_err(|_| "omp receipt write failed")?;
    file.as_file().sync_all().map_err(|_| "omp receipt sync failed")?;
    file.persist(directory.join("receipt")).map_err(|_| "omp receipt commit failed")?;
    Ok(())
}
/// Shared by read discovery and the existing native removal verifier.
pub(crate) fn owns(binary: &Path) -> bool {
    let Some(prefix) = binary.parent().and_then(Path::parent) else {
        return false;
    };
    let Ok(directory) = private_directory(prefix) else {
        return false;
    };
    let receipt = directory.join("receipt");
    if !private(&directory, true) || !private(&receipt, false) {
        return false;
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let Ok(file) = options.open(receipt) else {
        return false;
    };
    let mut expected = String::new();
    if file.take(66).read_to_string(&mut expected).is_err() {
        return false;
    }
    expected.len() == 65 && digest(binary).is_ok_and(|actual| expected == format!("{actual}\n"))
}
