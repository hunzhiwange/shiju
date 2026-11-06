//! Confirmed uninstall plans belong to the authorized execution host.
use super::{native, packages};
use crate::apps::lifecycle::coordinator::ToolLifecycleCoordinator;
use vw_api_types::agent_apps::{AppInstallation, AppInstallationEnvironment};

#[derive(Debug, PartialEq, Eq)]
enum Action {
    Package(packages::PackageCommand),
    Native(native::NativeRemoval),
}

#[derive(Debug, PartialEq, Eq)]
pub struct UninstallPlan {
    installation: AppInstallation,
    action: Action,
}

/// Inputs are fresh server-side discovery and ownership metadata, never client commands.
pub fn preview(
    installation: &AppInstallation,
    managers: &[String],
    brew: Option<&packages::BrewOwnership>,
) -> Result<UninstallPlan, String> {
    let action = match packages::preview(installation, managers, brew) {
        Ok(command) => Action::Package(command),
        Err(_) => Action::Native(native::preview(installation)?),
    };
    Ok(UninstallPlan { installation: installation.clone(), action })
}

/// Refresh must rediscover the selected ID on the authorized host, including manager ownership.
pub async fn uninstall<R>(plan: UninstallPlan, confirmed: bool, refresh: R) -> Result<(), String>
where
    R: FnOnce() -> Result<(AppInstallation, Vec<String>, Option<packages::BrewOwnership>), String>
        + Send
        + 'static,
{
    uninstall_inner(plan, confirmed, refresh, None).await
}

/// Explicit child environment from the authorized host, never inherited credentials.
pub async fn uninstall_isolated<R>(
    plan: UninstallPlan,
    confirmed: bool,
    refresh: R,
    environment: crate::apps::lifecycle::process::WriteProcessEnvironment,
) -> Result<(), String>
where
    R: FnOnce() -> Result<(AppInstallation, Vec<String>, Option<packages::BrewOwnership>), String>
        + Send
        + 'static,
{
    uninstall_inner(plan, confirmed, refresh, Some(environment)).await
}

async fn uninstall_inner<R>(
    plan: UninstallPlan,
    confirmed: bool,
    refresh: R,
    environment: Option<crate::apps::lifecycle::process::WriteProcessEnvironment>,
) -> Result<(), String>
where
    R: FnOnce() -> Result<(AppInstallation, Vec<String>, Option<packages::BrewOwnership>), String>
        + Send
        + 'static,
{
    ToolLifecycleCoordinator
        .run(vec![plan.installation.app_id], move || {
            if !confirmed {
                return Err("uninstall confirmation required".into());
            }
            let (fresh, managers, brew) = refresh()?;
            let current = preview(&fresh, &managers, brew.as_ref())?;
            if current != plan {
                return Err("installation changed; rediscover before uninstall".into());
            }
            match &fresh.environment {
                AppInstallationEnvironment::Macos if cfg!(target_os = "macos") => {}
                AppInstallationEnvironment::Linux if cfg!(target_os = "linux") => {}
                AppInstallationEnvironment::Windows if cfg!(windows) => {}
                _ => {
                    return Err(
                        "uninstall requires the selected authorized environment executor".into()
                    );
                }
            }
            match &current.action {
                Action::Native(removal) => removal.remove()?,
                Action::Package(action) => {
                    let mut command = crate::apps::lifecycle::process::command(
                        &action.program,
                        &action.args,
                        &action.env,
                        Some(&action.path_prefix),
                    )?;
                    if let Some(environment) = &environment {
                        environment.apply(&mut command)?;
                        command.envs(action.env.iter().cloned());
                    }
                    crate::apps::lifecycle::process::run_command(command)?;
                }
            }
            match std::fs::symlink_metadata(&fresh.path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                _ => Err(
                    "uninstall postcondition failed: selected entry remains or cannot be verified"
                        .into(),
                ),
            }
        })
        .await
}
