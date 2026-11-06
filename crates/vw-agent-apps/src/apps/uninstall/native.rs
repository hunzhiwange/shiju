//! Host-only native removal. Plans cannot be supplied by clients.
use std::path::{Path, PathBuf};
use vw_api_types::agent_apps::{AppId, AppInstallation, AppInstallationSource};

#[derive(Debug, PartialEq, Eq)]
pub struct NativeRemoval {
    installation: AppInstallation,
    paths: Vec<PathBuf>,
    fingerprints: Vec<(u64, std::time::SystemTime)>,
}

impl NativeRemoval {
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
    /// Revalidate under the execution host's installation write lock.
    pub fn remove(&self) -> Result<(), String> {
        if preview(&self.installation)? != *self {
            return Err("Installation changed; rediscover before uninstall".into());
        }
        for path in &self.paths {
            remove_file_no_follow(path)?;
        }
        Ok(())
    }
}

pub fn preview(inst: &AppInstallation) -> Result<NativeRemoval, String> {
    if inst.source != AppInstallationSource::System {
        return Err("Manual removal required: unverified native source".into());
    }
    let real = Path::new(inst.real_path.as_deref().ok_or("Missing native target")?);
    let entry = Path::new(&inst.path);
    check_parents(real)?;
    check_parents(entry)?;
    if std::fs::canonicalize(entry).map_err(|e| e.to_string())? != real
        || !known_native_install(inst.app_id, real)
    {
        return Err("Manual removal required: unrecognized or changed native installation".into());
    }
    let mut paths = vec![entry.to_path_buf()];
    if entry != real {
        paths.push(real.to_path_buf());
    }
    if inst.app_id == AppId::GrokBuild {
        let root = grok_native_root(real).ok_or("Unrecognized Grok root")?;
        for kind in ["bin", "downloads"] {
            let dir = root.join(kind);
            if !dir.exists() {
                continue;
            }
            check_parents(&dir.join("placeholder"))?;
            for item in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
                let path = item.map_err(|e| e.to_string())?.path();
                let meta = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
                let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                let launcher =
                    kind == "bin" && matches!(name, "grok" | "grok.exe" | "agent" | "agent.exe");
                if meta.file_type().is_symlink() {
                    if launcher
                        && std::fs::canonicalize(&path)
                            .is_ok_and(|p| grok_native_root(&p).as_ref() == Some(&root))
                    {
                        paths.push(path);
                    }
                } else if is_grok_binary_name(kind, name) && is_native_executable(&path) {
                    paths.push(path);
                }
            }
        }
    }
    paths.sort();
    paths.dedup();
    // Remove launchers before their targets; never recurse or follow a leaf symlink.
    paths.sort_by_key(|p| !std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()));
    let fingerprints = paths
        .iter()
        .map(|p| {
            let m = std::fs::symlink_metadata(p).map_err(|e| e.to_string())?;
            Ok((m.len(), m.modified().map_err(|e| e.to_string())?))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(NativeRemoval { installation: inst.clone(), paths, fingerprints })
}

/// Warnings use host-discovered updater mode and installation metadata.
pub fn warnings(
    inst: &AppInstallation,
    grok_native_present: bool,
    grok_npm_mode: bool,
) -> Vec<&'static str> {
    match inst.app_id {
        AppId::GrokBuild
            if grok_native_present
                && grok_npm_mode
                && inst
                    .real_path
                    .as_deref()
                    .is_some_and(|p| p.replace('\\', "/").contains("/node_modules/")) =>
        {
            vec!["Grok updater in npm mode will reinstall this package copy"]
        }
        AppId::Hermes => vec!["Hermes configuration, sessions and user data are retained"],
        _ => Vec::new(),
    }
}

fn check_parents(path: &Path) -> Result<(), String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir | std::path::Component::CurDir))
    {
        return Err("Refusing non-absolute or traversing native path".into());
    }
    for ancestor in path.ancestors().skip(1) {
        let meta = std::fs::symlink_metadata(ancestor).map_err(|e| e.to_string())?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err("Refusing native path through symlink or non-directory".into());
        }
    }
    Ok(())
}

pub(crate) fn is_native_executable(path: &Path) -> bool {
    use std::io::Read;
    if !std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file()) {
        return false;
    }
    let mut magic = [0; 4];
    std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut magic)).is_ok()
        && (magic == [0x7f, b'E', b'L', b'F']
            || magic[..2] == *b"MZ"
            || matches!(
                magic,
                [0xcf, 0xfa, 0xed, 0xfe]
                    | [0xfe, 0xed, 0xfa, 0xcf]
                    | [0xce, 0xfa, 0xed, 0xfe]
                    | [0xfe, 0xed, 0xfa, 0xce]
                    | [0xca, 0xfe, 0xba, 0xbe]
            ))
}

fn known_native_install(tool: AppId, real: &Path) -> bool {
    let real_text = real.to_string_lossy();
    let normalized = real_text.replace('\\', "/").to_ascii_lowercase();
    if normalized.contains("/node_modules/") || !is_native_executable(real) {
        return false;
    }
    match tool {
        AppId::Claude => {
            normalized
                .split_once("/.local/share/claude/versions/")
                .is_some_and(|(_, version)| !version.is_empty() && !version.contains('/'))
                || normalized.ends_with("/.local/bin/claude")
                || normalized.ends_with("/.local/bin/claude.exe")
        }
        AppId::Codex => normalized
            .split_once("/packages/standalone/releases/")
            .and_then(|(_, release)| release.split_once('/'))
            .is_some_and(|(version, binary)| {
                !version.is_empty()
                    && matches!(binary, "bin/codex" | "bin/codex.exe" | "codex" | "codex.exe")
            }),
        AppId::GrokBuild => grok_native_root(real).is_some(),
        // The current curl installer (including Git Bash on Windows) uses this
        // directory. Do not invent OPENCODE_INSTALL_DIR / XDG_BIN_DIR layouts.
        AppId::OpenCode => {
            normalized.ends_with("/.opencode/bin/opencode")
                || normalized.ends_with("/.opencode/bin/opencode.exe")
        }
        // Official install.sh / install.ps1 targets. A cask binary the self-updater
        // replaced in place sits in Homebrew's bin and stays a manual uninstall.
        AppId::Agy => {
            normalized.ends_with("/.local/bin/agy")
                || normalized.ends_with("/agy/bin/agy.exe")
                || crate::apps::lifecycle::local_agy::owns(real)
        }
        // Official install.sh (`~/.local/bin`) / install.ps1 (`%LOCALAPPDATA%\omp`) targets.
        AppId::Omp => {
            normalized.ends_with("/.local/bin/omp") || normalized.ends_with("/omp/omp.exe")
        }
        _ => false,
    }
}

const GROK_PLATFORMS: [&str; 6] = [
    "macos-aarch64",
    "macos-x86_64",
    "linux-aarch64",
    "linux-x86_64",
    "windows-aarch64.exe",
    "windows-x86_64.exe",
];

/// Names the official Grok installer writes: `bin/grok`, `bin/grok-<version>` (0.2.112+),
/// `downloads/grok-<platform>` and `downloads/grok-<version>-<platform>`.
fn is_grok_binary_name(dir: &str, name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let Some(rest) = name.strip_prefix("grok-") else {
        return dir == "bin" && matches!(name.as_str(), "grok" | "grok.exe");
    };
    match dir {
        "bin" => is_grok_version(rest.strip_suffix(".exe").unwrap_or(rest)),
        "downloads" => GROK_PLATFORMS.iter().any(|platform| {
            rest == *platform
                || rest
                    .strip_suffix(platform)
                    .and_then(|version| version.strip_suffix('-'))
                    .is_some_and(is_grok_version)
        }),
        _ => false,
    }
}

fn is_grok_version(version: &str) -> bool {
    version.starts_with(|c: char| c.is_ascii_digit())
        && version.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
}

/// The `.grok` directory owning an installer binary, or None for any other file.
fn grok_native_root(real: &Path) -> Option<PathBuf> {
    let name = real.file_name()?.to_str()?;
    let dir = real.parent()?;
    let kind = dir.file_name()?.to_str()?.to_ascii_lowercase();
    let root = dir.parent()?;
    (root.file_name()?.eq_ignore_ascii_case(".grok") && is_grok_binary_name(&kind, name))
        .then(|| root.to_path_buf())
}

// Walk from the filesystem root using O_NOFOLLOW directory descriptors. A parent
// replacement after validation cannot redirect unlinkat to a symlink target.
#[cfg(unix)]
fn remove_file_no_follow(path: &Path) -> Result<(), String> {
    use std::os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::ffi::OsStrExt,
    };
    let mut dir = std::fs::File::open("/").map_err(|e| e.to_string())?;
    let parent = path.parent().ok_or("Missing parent")?;
    for component in parent.components() {
        let std::path::Component::Normal(name) = component else { continue };
        let name = std::ffi::CString::new(name.as_bytes()).map_err(|e| e.to_string())?;
        // SAFETY: dir is live, name is NUL terminated; ownership transfers only on success.
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        // SAFETY: openat returned a fresh descriptor.
        dir = std::fs::File::from(unsafe { OwnedFd::from_raw_fd(fd) });
    }
    let name = std::ffi::CString::new(path.file_name().ok_or("Missing filename")?.as_bytes())
        .map_err(|e| e.to_string())?;
    // SAFETY: live directory descriptor and valid C string; flags=0 never removes directories.
    if unsafe { libc::unlinkat(dir.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

#[cfg(not(unix))]
fn remove_file_no_follow(_path: &Path) -> Result<(), String> {
    Err("Native file removal requires a host no-follow implementation on this platform".into())
}
