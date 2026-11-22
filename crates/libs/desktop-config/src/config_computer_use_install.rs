//! Install the signed controller outside the enclosing desktop bundle for TCC attribution.
use config_runtime_paths::{BUNDLED_COMPUTER_USE_APP_NAME, BUNDLED_COMPUTER_USE_EXECUTABLE};
use std::{
    ffi::CString,
    fs, io,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::Command,
};

const RECEIPT: &str = "Contents/Resources/.shiju-bundle-inputs.plist";

pub(super) fn install(source: &Path, config_dir: &Path) -> io::Result<PathBuf> {
    let helpers = config_dir.join("Helpers");
    fs::create_dir_all(&helpers)?;
    if !fs::symlink_metadata(&helpers)?.is_dir() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, "Helpers is not a directory"));
    }
    let target = helpers.join(BUNDLED_COMPUTER_USE_APP_NAME);
    let rename_flags = match fs::symlink_metadata(&target) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => libc::RENAME_EXCL,
        Err(error) => return Err(error),
        Ok(metadata) if metadata.is_dir() && target.join(RECEIPT).is_file() => {
            if ["Contents/Info.plist", BUNDLED_COMPUTER_USE_EXECUTABLE].iter().all(|relative| {
                super::same_file_contents(&source.join(relative), &target.join(relative))
                    .unwrap_or(false)
            }) && verify(&target).is_ok()
            {
                return Ok(target);
            }
            libc::RENAME_SWAP
        }
        Ok(metadata) if metadata.is_symlink() && legacy_link(&target)? => libc::RENAME_SWAP,
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "unrecognized computer use app at install location",
            ));
        }
    };

    let staging = tempfile::Builder::new().prefix(".computer-use-").tempdir_in(&helpers)?;
    let staged = staging.path().join(BUNDLED_COMPUTER_USE_APP_NAME);
    let status = Command::new("/usr/bin/ditto").arg(source).arg(&staged).status()?;
    if !status.success() {
        return Err(io::Error::other("copying the signed computer use app failed"));
    }
    verify(&staged)?;
    let from = CString::new(staged.as_os_str().as_bytes())?;
    let to = CString::new(target.as_os_str().as_bytes())?;
    // Both paths are in Helpers, so the upgrade never leaves the app missing.
    if unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), rename_flags) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(target)
}

fn verify(app: &Path) -> io::Result<()> {
    let output = Command::new("/usr/bin/codesign")
        .args(["--verify", "--strict", "-R=identifier \"com.shiju.computer-use\""])
        .arg(app)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "computer use app has an invalid signature",
        ));
    }
    Ok(())
}

fn legacy_link(target: &Path) -> io::Result<bool> {
    let previous = fs::read_link(target)?;
    Ok(previous.is_absolute()
        && previous.ends_with(Path::new("Contents/Helpers").join(BUNDLED_COMPUTER_USE_APP_NAME)))
}
