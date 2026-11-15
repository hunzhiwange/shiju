//! Read-only Claude Desktop presence on the authorized execution host.
use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopPlatform {
    Macos,
    Windows,
    Linux,
    Unsupported,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DesktopPresence {
    pub supported: bool,
    /// None means application detection is unavailable, never “not installed”.
    pub app_bundles: Option<Vec<PathBuf>>,
    /// Existing configuration directories are observations, not installation proof.
    pub config_directories: Vec<PathBuf>,
}

/// `base` is home on macOS, LocalAppData on Windows, XDG config root on Linux.
/// `applications` is the host's system Applications directory on macOS.
/// Inputs must be an authorized host snapshot, never client-supplied authority.
pub fn probe(
    platform: DesktopPlatform,
    base: &Path,
    applications: &Path,
) -> io::Result<DesktopPresence> {
    if platform == DesktopPlatform::Unsupported {
        return Ok(DesktopPresence {
            supported: false,
            app_bundles: None,
            config_directories: Vec::new(),
        });
    }
    if !base.is_absolute() || (platform == DesktopPlatform::Macos && !applications.is_absolute()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "absolute execution-host roots required",
        ));
    }
    let config_root = if platform == DesktopPlatform::Macos {
        base.join("Library/Application Support")
    } else {
        base.to_path_buf()
    };
    let mut config_directories = Vec::new();
    for name in ["Claude", "Claude-3p"] {
        let exact = config_root.join(name);
        if directory(&exact)? {
            config_directories.push(exact);
        } else if platform == DesktopPlatform::Windows {
            // Preserve the upstream versioned Windows config-directory fallback.
            let entries = match std::fs::read_dir(&config_root) {
                Ok(entries) => Some(entries),
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => return Err(error),
            };
            let mut candidates = Vec::new();
            for entry in entries.into_iter().flatten() {
                let path = entry?.path();
                if path.file_name().and_then(|name| name.to_str()).is_some_and(|candidate| {
                    candidate.starts_with("Claude")
                        && candidate.contains("-3p") == (name == "Claude-3p")
                }) && directory(&path)?
                {
                    candidates.push(path);
                }
            }
            candidates.sort();
            if let Some(path) = candidates.into_iter().next() {
                config_directories.push(path);
            }
        }
    }
    let app_bundles = if platform == DesktopPlatform::Macos {
        let mut bundles = Vec::new();
        for path in [applications.join("Claude.app"), base.join("Applications/Claude.app")] {
            if directory(&path)? {
                bundles.push(path);
            }
        }
        Some(bundles)
    } else {
        None
    };
    Ok(DesktopPresence { supported: true, app_bundles, config_directories })
}

fn directory(path: &Path) -> io::Result<bool> {
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(metadata.is_dir()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}
