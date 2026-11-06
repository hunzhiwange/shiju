//! WSL lifecycle plans are derived on the authorized Windows host.
use crate::apps::discovery::wsl::{WslShell, WslTarget};
use vw_api_types::agent_apps::{
    AppId, AppInstallation, AppInstallationEnvironment, AppInstallationReport,
};

#[derive(Debug, PartialEq, Eq)]
pub struct WslPlan {
    app: AppId,
    target: WslTarget,
    installation: Option<AppInstallation>,
    script: String,
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn path(value: &str) -> Result<(), String> {
    if !value.starts_with('/')
        || value.starts_with("//")
        || value.contains(['\\', '\0', '\r', '\n'])
        || value.split('/').any(|p| p == "." || p == "..")
    {
        return Err("absolute Linux target path required".into());
    }
    Ok(())
}

fn validate(target: &WslTarget, installation: &AppInstallation) -> Result<(), String> {
    if installation.environment
        != (AppInstallationEnvironment::Wsl { distribution: target.distro.clone() })
    {
        return Err("selected WSL distribution mismatch".into());
    }
    path(&installation.path)?;
    path(installation.real_path.as_deref().ok_or("resolved WSL target required")?)
}

pub fn install(target: WslTarget, app: AppId) -> Result<WslPlan, String> {
    let recipe =
        super::process::install_recipe(app, super::native_primary::InstallEnvironment::Wsl)?;
    let script = argv(recipe.program, &recipe.args);
    Ok(WslPlan { app, target, installation: None, script })
}

pub fn upgrade(
    target: WslTarget,
    installation: &AppInstallation,
    companions: &[String],
) -> Result<WslPlan, String> {
    validate(&target, installation)?;
    for companion in companions {
        path(companion)?;
    }
    let report = AppInstallationReport { installations: vec![installation.clone()] };
    let plan = super::update::plan(&report, &installation.id, companions)?;
    let c = plan.command;
    let mut script = format!("export PATH={}:\"$PATH\"; ", quote(&c.path_prefix));
    for (key, value) in c.env {
        script.push_str(&format!("export {key}={}; ", quote(&value)));
    }
    script.push_str(&argv(&c.program, &c.args));
    Ok(WslPlan {
        app: installation.app_id,
        target,
        installation: Some(installation.clone()),
        script,
    })
}

fn argv(program: &str, args: &[String]) -> String {
    let mut words = vec![quote(program)];
    if matches!(program, "bash" | "/bin/bash") {
        words.extend(["-o".into(), "pipefail".into()]);
    }
    words.extend(args.iter().map(|a| quote(a)));
    words.join(" ")
}

/// No local filesystem inspection: ownership and entry checks execute inside WSL.
/// Match upstream's supported npm-prefix and Hermes uninstall scope.
pub fn uninstall(target: WslTarget, installation: &AppInstallation) -> Result<WslPlan, String> {
    validate(&target, installation)?;
    let real = installation.real_path.as_deref().ok_or("resolved WSL target required")?;
    if real.contains("/Cellar/") || real.contains("/Caskroom/") {
        return Err("use Homebrew to uninstall this WSL application".into());
    }
    let entry = quote(&installation.path);
    let removal = if installation.app_id == AppId::Hermes {
        format!("{entry} uninstall --yes")
    } else {
        let package = if installation.app_id == AppId::Omp {
            "@oh-my-pi/pi-coding-agent"
        } else {
            super::npm::install_args(installation.app_id)?[2]
                .strip_suffix("@latest")
                .ok_or("unmanaged WSL package")?
        };
        let binary = crate::apps::catalog::catalog()
            .into_iter()
            .find(|a| a.metadata.id == installation.app_id)
            .and_then(|a| a.metadata.cli_command)
            .ok_or("CLI required")?;
        let (prefix, _) = real
            .split_once(&format!("/lib/node_modules/{package}/"))
            .filter(|(prefix, _)| {
                !prefix.is_empty() && installation.path == format!("{prefix}/bin/{binary}")
            })
            .ok_or("use original installer to uninstall this WSL application")?;
        let manager = format!("{prefix}/bin/npm");
        format!("{} uninstall -g --prefix {} {}", quote(&manager), quote(prefix), quote(package))
    };
    let removal = if installation.app_id == AppId::OpenClaw {
        format!("{entry} uninstall --service --yes --non-interactive && {removal}")
    } else {
        removal
    };
    let script = format!(
        "entry={entry}; [ \"$(readlink -f -- \"$entry\")\" = {} ] && {removal} && [ ! -e \"$entry\" ] && [ ! -L \"$entry\" ]",
        quote(real)
    );
    Ok(WslPlan {
        app: installation.app_id,
        target,
        installation: Some(installation.clone()),
        script,
    })
}

impl WslPlan {
    pub(super) fn command(
        &self,
        executable: &std::path::Path,
    ) -> Result<std::process::Command, String> {
        // Re-parse even manually constructed targets; Auto must have been resolved
        // by host shell discovery before a lifecycle preview can be executed.
        let WslShell::Explicit { shell, flag } = &self.target.shell else {
            return Err("resolve WSL shell before lifecycle execution".into());
        };
        let checked = crate::apps::discovery::wsl::parse_target(
            &format!(r"\\wsl$\{}", self.target.distro),
            Some(shell),
            Some(flag),
        )
        .map_err(|_| "invalid WSL execution target")?;
        if checked != self.target {
            return Err("invalid WSL execution target".into());
        }
        let mut command = std::process::Command::new(executable);
        // POSIX syntax is isolated from fish/zsh in an explicit sh child.
        command.args([
            "-d",
            &self.target.distro,
            "--",
            shell,
            flag,
            &format!("sh -c {}", quote(&self.script)),
        ]);
        Ok(command)
    }
}

/// Host callbacks must use authenticated scope and the bound WSL distribution.
/// Fresh plan equality includes shell, distro, source, identity and command.
/// Node gates and postconditions are supplied by host discovery under this lock.
pub async fn execute<R, G, P>(
    preview: WslPlan,
    confirmed: bool,
    refresh: R,
    gate: G,
    postcondition: P,
) -> Result<(), String>
where
    R: FnOnce() -> Result<WslPlan, String> + Send + 'static,
    G: FnOnce() -> Result<crate::apps::versions::node::NodeRequirement, String> + Send + 'static,
    P: FnOnce() -> Result<(), String> + Send + 'static,
{
    if !cfg!(windows) {
        return Err("WSL requires a Windows execution host".into());
    }
    execute_on(
        preview,
        confirmed,
        refresh,
        gate,
        postcondition,
        std::path::PathBuf::from("wsl.exe"),
    )
    .await
}

pub(super) async fn execute_on<R, G, P>(
    preview: WslPlan,
    confirmed: bool,
    refresh: R,
    gate: G,
    postcondition: P,
    executable: std::path::PathBuf,
) -> Result<(), String>
where
    R: FnOnce() -> Result<WslPlan, String> + Send + 'static,
    G: FnOnce() -> Result<crate::apps::versions::node::NodeRequirement, String> + Send + 'static,
    P: FnOnce() -> Result<(), String> + Send + 'static,
{
    super::coordinator::ToolLifecycleCoordinator
        .run(vec![preview.app], move || {
            if !confirmed {
                return Err("WSL lifecycle confirmation required".into());
            }
            let current = refresh()?;
            if current != preview {
                return Err("WSL installation or shell changed since preview".into());
            }
            if matches!(gate()?, crate::apps::versions::node::NodeRequirement::Gap { .. }) {
                return Err("Node upgrade required".into());
            }
            super::process::run_command(current.command(&executable)?)?;
            postcondition()
        })
        .await
}
