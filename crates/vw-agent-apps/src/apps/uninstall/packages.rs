//! Package previews from authorized host discoveries, never client commands.
use vw_api_types::agent_apps::{
    AppId, AppInstallation, AppInstallationEnvironment, AppInstallationSource,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageCommand {
    pub program: String,
    pub args: Vec<String>,
    pub path_prefix: String,
    pub env: Vec<(String, String)>,
}

/// Fresh brew info metadata read by the authorized execution host.
#[derive(Debug, Clone)]
pub struct BrewOwnership {
    pub prefix: String,
    pub token: String,
    pub tap: String,
    pub cask: bool,
}

const MANUAL: &str = "manual removal required: package ownership is not verified";

/// Preview only. Executor must rediscover and compare under its write lock.
/// Managers and ownership metadata must come from the authorized host.
pub fn preview(
    inst: &AppInstallation,
    managers: &[String],
    brew: Option<&BrewOwnership>,
) -> Result<PackageCommand, &'static str> {
    if inst.source == AppInstallationSource::Unknown {
        return Err(MANUAL);
    }
    let windows = inst.environment == AppInstallationEnvironment::Windows;
    let path = checked(&inst.path, windows)?;
    let real = checked(inst.real_path.as_deref().ok_or(MANUAL)?, windows)?;
    let dir = path.rsplit_once('/').ok_or(MANUAL)?.0;
    if dir.ends_with("/.bin") {
        return Err(MANUAL);
    }
    let manager = |name: &str, bin: &str| -> Result<String, &'static str> {
        managers
            .iter()
            .filter_map(|p| checked(p, windows).ok())
            .find(|p| {
                let expected = if windows {
                    vec![format!("{bin}/{name}.cmd"), format!("{bin}/{name}.exe")]
                } else {
                    vec![format!("{bin}/{name}")]
                };
                expected.iter().any(|e| if windows { e.eq_ignore_ascii_case(p) } else { e == p })
            })
            .ok_or("manual removal required: verified package manager missing")
    };
    let command = |program: String, args: Vec<String>| PackageCommand {
        path_prefix: program.rsplit_once('/').map_or("", |(p, _)| p).into(),
        program,
        args,
        env: Vec::new(),
    };
    // Formulae can contain node_modules; verify Homebrew ownership first.
    for (marker, cask) in [("/Cellar/", false), ("/Caskroom/", true)] {
        if let Some((prefix, rest)) = real.split_once(marker) {
            let token = rest.split('/').next().ok_or(MANUAL)?;
            let allowed = match inst.app_id {
                AppId::Claude => cask && matches!(token, "claude-code" | "claude-code@latest"),
                AppId::Codex => cask && token == "codex",
                AppId::Gemini => !cask && token == "gemini-cli",
                AppId::OpenCode => !cask && token == "opencode",
                AppId::Agy => cask && token == "antigravity-cli",
                _ => false,
            };
            let owner = brew.ok_or(MANUAL)?;
            let tap = if cask { "homebrew/cask" } else { "homebrew/core" };
            if !allowed
                || owner.prefix != prefix
                || owner.token != token
                || owner.cask != cask
                || owner.tap != tap
            {
                return Err(MANUAL);
            }
            return Ok(command(
                manager("brew", &format!("{prefix}/bin"))?,
                vec![
                    "uninstall".into(),
                    if cask { "--cask" } else { "--formula" }.into(),
                    format!("{tap}/{token}"),
                ],
            ));
        }
    }
    if inst.app_id == AppId::Hermes {
        // Delegate uv environment removal to selected CLI, retaining data.
        return Ok(command(path, vec!["uninstall".into(), "--yes".into()]));
    }
    let package = if inst.app_id == AppId::Omp {
        "@oh-my-pi/pi-coding-agent"
    } else {
        super::super::lifecycle::npm::install_args(inst.app_id).map_err(|_| MANUAL)?[2]
            .strip_suffix("@latest")
            .ok_or(MANUAL)?
    };
    let (tool, root, args, env) = match inst.source {
        AppInstallationSource::Bun => {
            let prefix = dir.strip_suffix("/bin").ok_or(MANUAL)?;
            let root = format!("{prefix}/install/global");
            (
                "bun",
                root.clone(),
                vec!["remove".into(), "-g".into(), package.into()],
                vec![
                    ("BUN_INSTALL_GLOBAL_DIR".into(), root),
                    ("BUN_INSTALL_BIN".into(), dir.into()),
                ],
            )
        }
        AppInstallationSource::Pnpm => {
            let (root, _) = real.split_once("/node_modules/").ok_or(MANUAL)?;
            let suffix = root.strip_prefix(&format!("{dir}/global/")).ok_or(MANUAL)?;
            if suffix.is_empty() || !suffix.bytes().all(|b| b.is_ascii_digit()) {
                return Err(MANUAL);
            }
            (
                "pnpm",
                root.into(),
                vec![
                    "remove".into(),
                    "-g".into(),
                    "--global-dir".into(),
                    root.into(),
                    "--global-bin-dir".into(),
                    dir.into(),
                    package.into(),
                ],
                Vec::new(),
            )
        }
        AppInstallationSource::System
        | AppInstallationSource::Nvm
        | AppInstallationSource::Fnm
        | AppInstallationSource::Mise
        | AppInstallationSource::Homebrew => {
            let prefix = if windows { dir } else { dir.strip_suffix("/bin").ok_or(MANUAL)? };
            let root = if windows { prefix.to_owned() } else { format!("{prefix}/lib") };
            (
                "npm",
                root,
                vec![
                    "uninstall".into(),
                    "-g".into(),
                    "--prefix".into(),
                    prefix.into(),
                    package.into(),
                ],
                Vec::new(),
            )
        }
        _ => return Err(MANUAL),
    };
    let package_dir =
        std::fs::canonicalize(format!("{root}/node_modules/{package}")).map_err(|_| MANUAL)?;
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(package_dir.join("package.json")).map_err(|_| MANUAL)?,
    )
    .map_err(|_| MANUAL)?;
    if manifest.get("name").and_then(|n| n.as_str()) != Some(package) {
        return Err(MANUAL);
    }
    let target = std::fs::canonicalize(&real).map_err(|_| MANUAL)?;
    // Windows global launchers are batch/PowerShell shims rather than symlinks.
    let owns_shim = windows
        && (path.ends_with(".cmd") || path.ends_with(".ps1"))
        && std::fs::read_to_string(&path).is_ok_and(|script| {
            script.replace('\\', "/").contains(&format!("node_modules/{package}/"))
        });
    if !target.starts_with(&package_dir) && !owns_shim {
        return Err(MANUAL);
    }
    let mut result = command(manager(tool, dir)?, args);
    result.env = env;
    Ok(result)
}

fn checked(path: &str, windows: bool) -> Result<String, &'static str> {
    let normalized = if windows { path.replace('\\', "/") } else { path.into() };
    let bytes = normalized.as_bytes();
    let absolute = if windows {
        bytes.len() > 3 && bytes[0].is_ascii_alphabetic() && &bytes[1..3] == b":/"
            || normalized.starts_with("//")
    } else {
        normalized.starts_with('/')
    };
    if !absolute
        || normalized.contains(['\0', '\r', '\n'])
        || normalized.split('/').any(|p| p == "." || p == "..")
    {
        return Err(MANUAL);
    }
    Ok(normalized)
}
