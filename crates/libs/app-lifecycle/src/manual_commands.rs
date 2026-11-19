//! Copy-only recipes. This module performs no discovery, IO or execution.
//! Sources: adjacent reviewed installers/npm.rs and cc-switch's
//! src/components/apps/toolUninstallCommands.ts. Package-manager support is
//! intentionally an allowlist, not inferred from an npm package's existence.
use super::native_primary::{InstallCommand, InstallEnvironment};
use vw_api_app_identity::AppId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualPlatform {
    Windows,
    MacOs,
    Linux,
    /// Paste into Bash inside the intended distro, never the Windows host shell.
    Wsl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualMethod {
    Npm,
    Pnpm,
    Bun,
    BrewFormula,
    BrewCask,
    Native,
    Hermes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualAction {
    Install,
    Uninstall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualShell {
    Bash,
    PowerShell,
}

#[derive(Debug, Clone)]
pub struct ManualCommand {
    pub app_id: AppId,
    pub platform: ManualPlatform,
    pub method: ManualMethod,
    pub action: ManualAction,
    pub shell: ManualShell,
    /// Distinguishes e.g. Homebrew core and the upstream tap.
    pub variant: Option<&'static str>,
    pub text: String,
}

/// Unsupported combinations return no rows, including all desktop app recipes.
/// Inputs select fixed data only; no path, version, shell or user text is accepted.
/// These strings are clipboard content, never input to the authorized executor.
pub fn commands_for(
    app: AppId,
    platform: ManualPlatform,
    method: ManualMethod,
    action: ManualAction,
) -> Vec<ManualCommand> {
    let shell = if platform == ManualPlatform::Windows {
        ManualShell::PowerShell
    } else {
        ManualShell::Bash
    };
    let row = |text, variant| ManualCommand {
        app_id: app, platform, method, action, shell, variant, text,
    };
    if action == ManualAction::Install {
        let recipe = match method {
            ManualMethod::Npm => super::npm::install_args(app).map(|args| InstallCommand {
                program: "npm",
                args: args.iter().map(|arg| (*arg).to_owned()).collect(),
            }),
            ManualMethod::Native => native_install(app, platform),
            _ => return Vec::new(),
        };
        return recipe.ok().map(|command| row(render_install(command, shell), None))
            .into_iter().collect();
    }
    UNINSTALL.iter().filter(|entry| {
        entry.app == app && entry.method == method && match entry.platforms {
            Platforms::All => true,
            Platforms::Posix => platform != ManualPlatform::Windows,
            Platforms::MacOs => platform == ManualPlatform::MacOs,
        }
    }).map(|entry| {
        let program = match (entry.method, shell) {
            (ManualMethod::Npm, ManualShell::PowerShell) => "npm.cmd",
            (ManualMethod::Pnpm, ManualShell::PowerShell) => "pnpm.cmd",
            _ => entry.program,
        };
        row(render_argv(program, entry.args, shell), entry.variant)
    }).collect()
}

fn native_install(app: AppId, platform: ManualPlatform) -> Result<InstallCommand, &'static str> {
    let environment = match platform {
        ManualPlatform::Windows => InstallEnvironment::Windows,
        ManualPlatform::Wsl => InstallEnvironment::Wsl,
        _ => InstallEnvironment::Posix,
    };
    match app {
        AppId::Claude | AppId::OpenCode if platform != ManualPlatform::Windows =>
            super::native_primary::install_command(app, environment),
        AppId::GrokBuild | AppId::Mcode =>
            super::native_grok_mcode::install_command(app, environment),
        AppId::Hermes | AppId::Agy =>
            super::native_hermes_agy::install_command(app, environment),
        AppId::Kimi | AppId::Omp =>
            super::native_kimi_omp::install_command(app, environment),
        _ => Err("no reviewed native installation recipe for this combination"),
    }
}

fn quote(value: &str, shell: ManualShell) -> String {
    let escaped = match shell {
        ManualShell::Bash => value.replace('\'', "'\\''"),
        ManualShell::PowerShell => value.replace('\'', "''"),
    };
    format!("'{escaped}'")
}

fn render_argv(program: &str, args: &[&str], shell: ManualShell) -> String {
    let prefix = if shell == ManualShell::PowerShell { "& " } else { "" };
    let mut text = format!("{prefix}{}", quote(program, shell));
    for arg in args {
        text.push(' ');
        text.push_str(&quote(arg, shell));
    }
    text
}

fn render_install(command: InstallCommand, shell: ManualShell) -> String {
    // PowerShell recipes already contain a complete script, including isolated
    // child installers and exit-code handling. Preserve that script verbatim;
    // joining the executor's -Command argv would add another quoting layer.
    if command.program == "powershell" {
        return command.args.last().cloned().unwrap_or_default();
    }
    let program = if shell == ManualShell::PowerShell && command.program == "npm" {
        "npm.cmd"
    } else {
        command.program
    };
    let mut args: Vec<&str> = command.args.iter().map(String::as_str).collect();
    if program == "bash" {
        // The executor enables pipefail; the copyable form must preserve it too.
        args.splice(0..0, ["-o", "pipefail"]);
    }
    render_argv(program, &args, shell)
}

#[derive(Clone, Copy)]
enum Platforms { All, Posix, MacOs }

struct UninstallRecipe {
    app: AppId,
    method: ManualMethod,
    program: &'static str,
    args: &'static [&'static str],
    platforms: Platforms,
    variant: Option<&'static str>,
}

const UNINSTALL: &[UninstallRecipe] = &[
    UninstallRecipe {
        app: AppId::Dsh, method: ManualMethod::Npm, program: "npm",
        args: &["uninstall", "-g", "@deepseek-ai/dsh"], platforms: Platforms::All,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::Claude, method: ManualMethod::Npm, program: "npm",
        args: &["uninstall", "-g", "@anthropic-ai/claude-code"], platforms: Platforms::All,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::Codex, method: ManualMethod::Npm, program: "npm",
        args: &["uninstall", "-g", "@openai/codex"], platforms: Platforms::All,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::Gemini, method: ManualMethod::Npm, program: "npm",
        args: &["uninstall", "-g", "@google/gemini-cli"], platforms: Platforms::All,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::GrokBuild, method: ManualMethod::Npm, program: "npm",
        args: &["uninstall", "-g", "@xai-official/grok"], platforms: Platforms::All,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::OpenCode, method: ManualMethod::Npm, program: "npm",
        args: &["uninstall", "-g", "opencode-ai"], platforms: Platforms::All,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::OpenClaw, method: ManualMethod::Npm, program: "npm",
        args: &["uninstall", "-g", "openclaw"], platforms: Platforms::All,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::Pi, method: ManualMethod::Npm, program: "npm",
        args: &["uninstall", "-g", "@earendil-works/pi-coding-agent"], platforms: Platforms::All,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::Mcode, method: ManualMethod::Npm, program: "npm",
        args: &["uninstall", "-g", "@minimax-ai/code"], platforms: Platforms::All,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::Kimi, method: ManualMethod::Npm, program: "npm",
        args: &["uninstall", "-g", "@moonshot-ai/kimi-code"], platforms: Platforms::All,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::OpenCode, method: ManualMethod::Pnpm, program: "pnpm",
        args: &["remove", "-g", "opencode-ai"], platforms: Platforms::All,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::OpenClaw, method: ManualMethod::Pnpm, program: "pnpm",
        args: &["remove", "-g", "openclaw"], platforms: Platforms::All,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::OpenCode, method: ManualMethod::Bun, program: "bun",
        args: &["remove", "-g", "opencode-ai"], platforms: Platforms::Posix,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::OpenClaw, method: ManualMethod::Bun, program: "bun",
        args: &["remove", "-g", "openclaw"], platforms: Platforms::All,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::Omp, method: ManualMethod::Bun, program: "bun",
        args: &["remove", "-g", "@oh-my-pi/pi-coding-agent"], platforms: Platforms::All,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::Gemini, method: ManualMethod::BrewFormula, program: "brew",
        args: &["uninstall", "--formula", "gemini-cli"], platforms: Platforms::Posix,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::OpenCode, method: ManualMethod::BrewFormula, program: "brew",
        args: &["uninstall", "--formula", "anomalyco/tap/opencode"], platforms: Platforms::Posix,
        variant: Some("anomalyco/tap"),
    },
    UninstallRecipe {
        app: AppId::OpenCode, method: ManualMethod::BrewFormula, program: "brew",
        args: &["uninstall", "--formula", "opencode"], platforms: Platforms::Posix,
        variant: Some("homebrew/core"),
    },
    UninstallRecipe {
        app: AppId::Claude, method: ManualMethod::BrewCask, program: "brew",
        args: &["uninstall", "--cask", "claude-code"], platforms: Platforms::MacOs,
        variant: Some("claude-code"),
    },
    UninstallRecipe {
        app: AppId::Claude, method: ManualMethod::BrewCask, program: "brew",
        args: &["uninstall", "--cask", "claude-code@latest"], platforms: Platforms::MacOs,
        variant: Some("claude-code@latest"),
    },
    UninstallRecipe {
        app: AppId::Codex, method: ManualMethod::BrewCask, program: "brew",
        args: &["uninstall", "--cask", "codex"], platforms: Platforms::MacOs,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::Agy, method: ManualMethod::BrewCask, program: "brew",
        args: &["uninstall", "--cask", "antigravity-cli"], platforms: Platforms::MacOs,
        variant: None,
    },
    UninstallRecipe {
        app: AppId::Hermes, method: ManualMethod::Hermes, program: "hermes",
        args: &["uninstall", "--yes"], platforms: Platforms::All,
        variant: None,
    },
];
