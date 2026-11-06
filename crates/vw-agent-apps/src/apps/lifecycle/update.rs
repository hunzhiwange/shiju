//! Host-side upgrade planning. Plans are never deserialized from client input.
use vw_api_types::agent_apps::{AppInstallation, AppInstallationId, AppInstallationReport};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCommand {
    pub program: String,
    pub args: Vec<String>,
    /// Process-scoped settings; never modify the host global environment.
    pub env: Vec<(String, String)>,
    /// Prepend this directory to the host environment's PATH for env shebangs.
    pub path_prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePlan {
    installation: AppInstallation,
    pub command: UpdateCommand,
    pub needs_confirmation: bool,
}

/// `report` and `companions` are observations from the authorized execution
/// host, not DTOs accepted from clients. The caller owns host/scope authorization.
pub fn plan(
    report: &AppInstallationReport,
    selected: &AppInstallationId,
    companions: &[String],
) -> Result<UpdatePlan, &'static str> {
    let mut matches = report.installations.iter().filter(|i| &i.id == selected);
    let installation = matches.next().ok_or("selected installation missing")?;
    if matches.next().is_some() {
        return Err("ambiguous installation identity");
    }
    if report.installations.iter().any(|i| {
        i.app_id != installation.app_id
            || i.target_host_id != installation.target_host_id
            || i.environment != installation.environment
    }) {
        return Err("installation report requires one application, host and environment");
    }
    Ok(UpdatePlan {
        installation: installation.clone(),
        command: recipe(installation, companions)?,
        needs_confirmation: report.installations.len() > 1,
    })
}

impl UpdatePlan {
    pub fn app_id(&self) -> vw_api_types::agent_apps::AppId {
        self.installation.app_id
    }

    pub(super) fn verify_result(&self, report: &AppInstallationReport) -> bool {
        report.installations.iter().any(|i| {
            i.id == self.installation.id
                && i.app_id == self.installation.app_id
                && i.target_host_id == self.installation.target_host_id
                && i.environment == self.installation.environment
                && i.runtime_status
                    == vw_api_types::agent_apps::AppInstallationRuntimeStatus::Runnable
        })
    }

    /// Invoke with fresh host discoveries while holding the application write lock.
    /// PATH-default changes alone do not change the selected installation.
    pub fn revalidate(
        &self,
        report: &AppInstallationReport,
        companions: &[String],
        confirmed: bool,
    ) -> Result<(), &'static str> {
        let current = plan(report, &self.installation.id, companions)?;
        let mut expected = self.installation.clone();
        let mut observed = current.installation.clone();
        expected.is_path_default = false;
        observed.is_path_default = false;
        if expected != observed || self.command != current.command {
            return Err("installation changed since preview");
        }
        if (self.needs_confirmation || current.needs_confirmation) && !confirmed {
            return Err("multiple installations require confirmation");
        }
        Ok(())
    }
}

fn parent(path: &str) -> Result<&str, &'static str> {
    path.rsplit_once('/')
        .map(|(dir, _)| dir)
        .filter(|dir| !dir.is_empty())
        .ok_or("absolute installation path required")
}

fn checked_path(path: &str, windows: bool) -> Result<String, &'static str> {
    let path = if windows { path.replace('\\', "/") } else { path.to_owned() };
    let bytes = path.as_bytes();
    let absolute = if windows {
        bytes.len() > 3 && bytes[0].is_ascii_alphabetic() && bytes[1..3] == *b":/"
            || path.starts_with("//")
    } else {
        path.starts_with('/')
    };
    if !absolute
        || path.contains(['\0', '\r', '\n'])
        || path.split('/').any(|part| part == "." || part == "..")
    {
        return Err("absolute installation path required");
    }
    Ok(path)
}

fn command(program: String, args: Vec<String>) -> Result<UpdateCommand, &'static str> {
    Ok(UpdateCommand { path_prefix: parent(&program)?.into(), program, args, env: Vec::new() })
}

fn recipe(inst: &AppInstallation, companions: &[String]) -> Result<UpdateCommand, &'static str> {
    use vw_api_types::agent_apps::{
        AppId::*, AppInstallationEnvironment, AppInstallationSource::*,
    };
    let windows = matches!(inst.environment, AppInstallationEnvironment::Windows);
    let path = checked_path(&inst.path, windows)?;
    let real = checked_path(
        inst.real_path.as_deref().ok_or("resolved installation target required")?,
        windows,
    )?;
    let lower = real.to_ascii_lowercase();
    let dir = parent(&path)?;
    // Companions must be freshly resolved by the authorized host, including .cmd
    // versus .exe on Windows. Never search PATH or synthesize a missing program.
    let sibling = |name: &str| -> Result<String, &'static str> {
        for observed in companions {
            let candidate = checked_path(observed, windows)?;
            let basename = candidate.rsplit('/').next().unwrap_or_default();
            let matches = if windows {
                [format!("{name}.cmd"), format!("{name}.exe")]
                    .iter()
                    .any(|n| basename.eq_ignore_ascii_case(n))
            } else {
                basename == name
            };
            let same_dir = if windows {
                parent(&candidate)?.eq_ignore_ascii_case(dir)
            } else {
                parent(&candidate)? == dir
            };
            if matches && same_dir {
                return Ok(candidate);
            }
        }
        Err("installation has no verified sibling package manager")
    };
    let self_update =
        |args: &[&str]| command(path.clone(), args.iter().map(|v| (*v).into()).collect());
    if matches!(inst.app_id, Hermes | Agy) {
        return self_update(&["update"]);
    }
    for (marker, cask) in [("/caskroom/", true), ("/cellar/", false)] {
        if let Some((_, suffix)) = lower.split_once(marker) {
            let token = suffix.split('/').next().unwrap_or_default();
            if token.is_empty()
                || !token.bytes().all(|b| b.is_ascii_alphanumeric() || b"@+._-".contains(&b))
            {
                return Err("invalid Homebrew package token");
            }
            let mut args = vec!["upgrade".into()];
            if cask {
                args.push("--cask".into());
            }
            args.push(token.into());
            return command(sibling("brew")?, args);
        }
    }
    if inst.app_id == Omp && lower.starts_with("/nix/store/") {
        return Err("unmanaged installation");
    }
    let npm_managed = lower.contains("/node_modules/")
        || matches!(inst.source, Nvm | Fnm | Mise | Volta | Bun | Pnpm);
    if npm_managed {
        let (manager, args) = match inst.source {
            Bun => ("bun", vec!["add".into(), "-g".into(), package(inst.app_id)?]),
            Pnpm if inst.app_id != Omp => {
                ("pnpm", vec!["add".into(), "-g".into(), package(inst.app_id)?])
            }
            Volta if inst.app_id != Omp => ("volta", vec!["install".into(), package(inst.app_id)?]),
            _ if inst.app_id != Omp => (
                "npm",
                super::npm::install_args(inst.app_id)?.iter().map(|s| (*s).into()).collect(),
            ),
            _ => return Err("unmanaged installation"),
        };
        return command(sibling(manager)?, args);
    }
    if inst.app_id == Codex
        && let Some(index) = lower.find("/packages/standalone/")
    {
        let home = &real[..index];
        let quote = |s: &str| s.replace('\'', "'\\''");
        let script = if windows {
            format!(
                "$ErrorActionPreference = 'Stop'; $env:CODEX_NON_INTERACTIVE = '1'; $env:CODEX_INSTALL_DIR = '{}'; $env:CODEX_HOME = '{}'; Invoke-RestMethod https://chatgpt.com/codex/install.ps1 | Invoke-Expression; exit $LASTEXITCODE",
                dir.replace('\'', "''"),
                home.replace('\'', "''")
            )
        } else {
            format!(
                "export CODEX_NON_INTERACTIVE=1 CODEX_INSTALL_DIR='{}' CODEX_HOME='{}'; tmp=$(mktemp) || exit $?; trap 'rm -f \"$tmp\"' EXIT; curl -fsSL https://chatgpt.com/codex/install.sh -o \"$tmp\" && sh \"$tmp\"",
                quote(dir),
                quote(home)
            )
        };
        let recipe = super::native_primary::InstallCommand {
            program: if windows { "powershell" } else { "bash" },
            args: if windows {
                vec!["-NoProfile".into(), "-NonInteractive".into(), "-Command".into(), script]
            } else {
                vec!["-c".into(), script]
            },
        };
        return installer(recipe, windows, dir, companions);
    }
    match inst.app_id {
        Claude
            if lower.contains("/.local/share/claude/") || lower.contains("/claude/versions/") =>
        {
            self_update(&["update"])
        }
        GrokBuild
            if lower.contains("/.grok/downloads/grok-")
                || path.replace('\\', "/").contains("/.grok/bin/") =>
        {
            let mut update = self_update(&["update"])?;
            // The native updater may delegate to npm according to Grok's own
            // receipt. Preserve npm 12 postinstall behavior within this tree.
            update.env = vec![
                ("npm_config_allow_scripts".into(), "@xai-official/grok".into()),
                ("npm_config_ignore_scripts".into(), "false".into()),
                ("npm_config_include".into(), "optional".into()),
            ];
            Ok(update)
        }
        OpenCode
            if inst.source == Native
                || lower.contains("/.opencode/")
                || lower.contains("/go/bin/") =>
        {
            self_update(&["upgrade"])
        }
        Kimi if inst.source == Native => {
            // Old Kimi silently ignores --yes. Require a known modern version;
            // legacy native installations rerun the installer in the same root.
            let modern = inst
                .version
                .as_deref()
                .and_then(|s| s.trim_start_matches('v').parse::<semver::Version>().ok())
                .is_some_and(|v| v >= semver::Version::new(0, 43, 0));
            if modern {
                return self_update(&["upgrade", "--yes"]);
            }
            let bin_dir = parent(&real)?;
            if !bin_dir.ends_with("/bin") {
                return Err("unmanaged installation");
            }
            let root = parent(bin_dir)?;
            installer(
                super::native_kimi_omp::kimi_installer_update_command(environment(inst), root)?,
                windows,
                dir,
                companions,
            )
        }
        Omp if inst.source == Native => self_update(&["update"]),
        Mcode if lower.contains("/.minimax-code/") => {
            let index = lower.find("/.minimax-code/").ok_or("unmanaged installation")?;
            let root = &real[..index + "/.minimax-code".len()];
            installer(
                super::native_grok_mcode::mcode_update_command(environment(inst), root)?,
                windows,
                dir,
                companions,
            )
        }
        _ => Err("unmanaged installation"),
    }
}

fn package(app: vw_api_types::agent_apps::AppId) -> Result<String, &'static str> {
    if app == vw_api_types::agent_apps::AppId::Omp {
        return Ok("@oh-my-pi/pi-coding-agent@latest".into());
    }
    Ok(super::npm::install_args(app)?[2].into())
}

fn environment(inst: &AppInstallation) -> super::native_primary::InstallEnvironment {
    use super::native_primary::InstallEnvironment;
    match inst.environment {
        vw_api_types::agent_apps::AppInstallationEnvironment::Windows => {
            InstallEnvironment::Windows
        }
        vw_api_types::agent_apps::AppInstallationEnvironment::Wsl { .. } => InstallEnvironment::Wsl,
        _ => InstallEnvironment::Posix,
    }
}

fn installer(
    recipe: super::native_primary::InstallCommand,
    windows: bool,
    dir: &str,
    companions: &[String],
) -> Result<UpdateCommand, &'static str> {
    // Absolute host interpreter resolution is deliberately required from the
    // executor; installer plans carry a fixed interpreter, never a PATH tool.
    if windows {
        let program = companions
            .iter()
            .find(|p| p.replace('\\', "/").to_ascii_lowercase().ends_with("/powershell.exe"))
            .ok_or("verified PowerShell interpreter required")?;
        let program = checked_path(program, true)?;
        let mut args = recipe.args;
        // Existing recipes isolate installer exit in a child. Pin that child to
        // the same verified interpreter as well.
        if let Some(script) = args.last_mut() {
            *script =
                script.replace("& powershell ", &format!("& '{}' ", program.replace('\'', "''")));
        }
        return Ok(UpdateCommand { program, args, path_prefix: dir.into(), env: Vec::new() });
    }
    Ok(UpdateCommand {
        program: "/bin/bash".into(),
        args: recipe.args,
        path_prefix: dir.into(),
        env: Vec::new(),
    })
}
