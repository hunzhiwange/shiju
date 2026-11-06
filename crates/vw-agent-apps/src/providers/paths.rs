//! Read-only path resolution on the authorized execution host. No ambient client HOME.
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::paths::{ConfigPathState, ConfigPathStatus, ProviderPaths},
};

/// Capture only the execution process's relevant environment, never request-supplied values.
pub fn execution_environment() -> BTreeMap<String, String> {
    [
        "OMP_PROFILE",
        "PI_PROFILE",
        "PI_CONFIG_DIR",
        "PI_CODING_AGENT_DIR",
        "XDG_DATA_HOME",
        "HERMES_HOME",
        "MINIMAX_DATA_DIR",
        "MAVIS_DATA_DIR",
        "KIMI_CODE_HOME",
        "DSH_HOME",
    ]
    .into_iter()
    .filter_map(|key| std::env::var(key).ok().map(|value| (key.into(), value)))
    .collect()
}
fn profile(raw: Option<&str>) -> Result<Option<&str>, &'static str> {
    let name = raw.unwrap_or_default().trim();
    if name.is_empty() || name == "default" {
        return Ok(None);
    }
    let reserved = name.split('.').next().unwrap_or_default().to_ascii_uppercase();
    if name.len() > 64
        || !name.as_bytes()[0].is_ascii_lowercase() && !name.as_bytes()[0].is_ascii_digit()
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
        || name.ends_with('.')
        || matches!(reserved.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || reserved.len() == 4
            && (reserved.starts_with("COM") || reserved.starts_with("LPT"))
            && reserved.as_bytes()[3].is_ascii_digit()
    {
        return Err("invalid configuration profile");
    }
    Ok(Some(name))
}
fn bounded(home: &Path, raw: &str) -> Result<PathBuf, &'static str> {
    if raw.is_empty() || raw.len() > 4096 || raw.chars().any(char::is_control) {
        return Err("invalid configuration directory");
    }
    // Match the source settings' tilde expansion; never expand another user's HOME.
    let path = if raw == "~" {
        home.to_path_buf()
    } else if raw.starts_with("~/") || raw.starts_with("~\\") {
        home.join(raw[2..].replace('\\', "/"))
    } else {
        PathBuf::from(raw)
    };
    if !home.is_absolute()
        || !path.is_absolute()
        || !path.starts_with(home)
        || path.components().any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err("unauthorized configuration directory");
    }
    Ok(path)
}
/// Metadata only: descriptor-relative traversal refuses symlinks, devices and unreadable paths.
/// The trusted HOME is opened once, so replacing a descendant cannot redirect traversal.
#[cfg(unix)]
fn probe(home: &Path, path: &Path, directory: bool) -> ConfigPathState {
    use ConfigPathState::*;
    use std::{
        fs::OpenOptions,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::{ffi::OsStrExt, fs::OpenOptionsExt},
        },
    };
    let Ok(relative) = path.strip_prefix(home) else {
        return Inaccessible;
    };
    let Ok(mut file) = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(home)
    else {
        return Inaccessible;
    };
    let parts: Vec<_> = relative.components().collect();
    for (index, part) in parts.iter().enumerate() {
        let Component::Normal(name) = part else {
            return Inaccessible;
        };
        let Ok(name) = std::ffi::CString::new(name.as_bytes()) else {
            return Inaccessible;
        };
        let last = index + 1 == parts.len();
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | libc::O_NONBLOCK
            | if !last || directory { libc::O_DIRECTORY } else { 0 };
        // SAFETY: file owns a live directory descriptor; name is one NUL-terminated component.
        let fd = unsafe { libc::openat(file.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
                Absent
            } else {
                Inaccessible
            };
        }
        // SAFETY: openat returned a fresh descriptor, consumed exactly once.
        file = unsafe { std::fs::File::from_raw_fd(fd) };
    }
    match file.metadata() {
        Ok(meta) if if directory { meta.is_dir() } else { meta.is_file() } => Present,
        _ => Inaccessible,
    }
}
#[cfg(not(unix))]
fn probe(_home: &Path, _path: &Path, _directory: bool) -> ConfigPathState {
    ConfigPathState::Inaccessible
}

/// Returned observations do not authorize writes or guarantee future path identity.
pub fn read(
    home: &Path,
    app: AppId,
    override_dir: Option<&str>,
    environment: &BTreeMap<String, String>,
) -> Result<ProviderPaths, &'static str> {
    use AppId::*;
    use ConfigPathState::*;
    if !cfg!(target_os = "macos") {
        return Err("unsupported configuration platform");
    }
    let env =
        |name: &str| environment.get(name).map(String::as_str).filter(|value| !value.is_empty());
    if override_dir.is_some()
        && !matches!(
            app,
            Claude | Codex | Gemini | GrokBuild | OpenCode | OpenClaw | Hermes | Pi | Omp
        )
    {
        return Err("unsupported configuration override");
    }
    let selected = if app == Omp {
        profile(
            environment
                .get("OMP_PROFILE")
                .or_else(|| environment.get("PI_PROFILE"))
                .map(String::as_str),
        )?
    } else {
        None
    };
    let omp_root = home.join(env("PI_CONFIG_DIR").unwrap_or(".omp").trim_start_matches('/'));
    let omp_default = match selected {
        Some(name) => omp_root.join("profiles").join(name).join("agent"),
        None => omp_root.join("agent"),
    };
    let raw_directory = match app {
        Claude => home.join(".claude"),
        Codex => home.join(".codex"),
        Gemini => home.join(".gemini"),
        GrokBuild => home.join(".grok"),
        OpenCode => home.join(".config/opencode"),
        OpenClaw => home.join(".openclaw"),
        Hermes => env("HERMES_HOME")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".hermes")),
        Pi => {
            env("PI_CODING_AGENT_DIR").map(PathBuf::from).unwrap_or_else(|| home.join(".pi/agent"))
        }
        Omp => {
            let inherited = profile(env("PI_PROFILE")).ok().flatten();
            let agent = env("PI_CODING_AGENT_DIR").map(PathBuf::from).filter(|path| {
                inherited
                    .is_none_or(|name| *path != omp_root.join("profiles").join(name).join("agent"))
            });
            if selected.is_some() {
                omp_default.clone()
            } else {
                agent.unwrap_or_else(|| omp_default.clone())
            }
        }
        Mcode => env("MINIMAX_DATA_DIR")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .or_else(|| env("MAVIS_DATA_DIR").map(str::trim).filter(|s| !s.is_empty()))
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".minimax")),
        Kimi => env("KIMI_CODE_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".kimi-code")),
        Dsh => env("DSH_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".dsh")),
        Agy => home.join(".gemini/antigravity-cli"),
        ClaudeDesktop => home.join("Library/Application Support"),
    };
    if override_dir.is_none() && app != Pi && !raw_directory.is_absolute() {
        return Err("relative execution directory is unsupported");
    }
    let raw =
        override_dir.unwrap_or(raw_directory.to_str().ok_or("invalid configuration directory")?);
    let directory = bounded(home, raw)?;
    // Inaccessible default paths are returned as states. An explicit override must not
    // turn a symlink or an untraversable directory into an authorized target.
    if override_dir.is_some() && probe(home, &directory, true) == Inaccessible {
        return Err("unauthorized configuration directory");
    }
    let prefer = |first: &str, second: &str| {
        if probe(home, &directory.join(first), false) == Absent
            && probe(home, &directory.join(second), false) != Absent
        {
            second.to_string()
        } else {
            first.to_string()
        }
    };
    let names: Vec<String> = match app {
        Claude => vec![prefer("settings.json", "claude.json")],
        Codex => vec!["config.toml".into(), "auth.json".into()],
        Gemini => vec![".env".into(), "settings.json".into()],
        GrokBuild | Kimi => vec!["config.toml".into()],
        OpenCode => vec![if probe(home, &directory.join("opencode.jsonc"), false) != Absent {
            "opencode.jsonc".into()
        } else {
            "opencode.json".into()
        }],
        OpenClaw => vec!["openclaw.json".into()],
        Hermes | Mcode => vec!["config.yaml".into()],
        Pi => vec!["models.json".into(), "settings.json".into()],
        Omp => vec![
            prefer("models.yml", "models.yaml"),
            prefer("config.yml", "config.yaml"),
            "mcp.json".into(),
        ],
        Dsh => ["web", "desktop", "headless", "acp"]
            .map(|name| format!("profiles/{name}/cordis.patch.yml"))
            .to_vec(),
        Agy => vec!["settings.json".into()],
        ClaudeDesktop => vec![
            "Claude/claude_desktop_config.json".into(),
            "Claude-3p/claude_desktop_config.json".into(),
        ],
    };
    let mut data_directory = directory.clone();
    if app == Omp
        && directory == omp_default
        && let Some(xdg) = env("XDG_DATA_HOME")
    {
        let mut data = bounded(home, xdg)?.join("omp");
        if let Some(name) = selected {
            data = data.join("profiles").join(name);
        }
        if probe(home, &data, true) == Present {
            data_directory = data;
        }
    }
    let files = names
        .into_iter()
        .map(|name| {
            let path = directory.join(name);
            ConfigPathStatus {
                state: probe(home, &path, false),
                path: path.to_string_lossy().into_owned(),
            }
        })
        .collect();
    Ok(ProviderPaths {
        app_id: app,
        directory: directory.to_string_lossy().into_owned(),
        data_directory: data_directory.to_string_lossy().into_owned(),
        files,
    })
}
