//! Local lifecycle adapter restricted to an administrator-approved installation prefix.
use super::{npm, process, update};
use crate::apps::{
    discovery::{
        read::{self, LocalReadEnvironment},
        report,
    },
    uninstall::execute,
};
use std::path::{Path, PathBuf};
use vw_api_types::agent_apps::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteEnvironment {
    pub prefix: PathBuf,
    pub npm: PathBuf,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub request: AppActionPreviewRequest,
    pub source: AppInstallationSource,
    action: Action,
}
#[derive(Debug, PartialEq, Eq)]
enum Action {
    Install,
    Update(update::UpdatePlan),
    Uninstall(execute::UninstallPlan),
}
fn validate(write: &WriteEnvironment, read: &LocalReadEnvironment) -> Result<(), String> {
    if !cfg!(unix) {
        return Err("local POSIX lifecycle executor required".into());
    }
    for path in [&write.prefix, &write.npm, &read.home] {
        if !path.is_absolute()
            || path.components().any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err("absolute administrator paths required".into());
        }
    }
    if write.prefix.canonicalize().map_err(|_| "write prefix unavailable")? != write.prefix
        || !write.prefix.is_dir()
        || !write.npm.is_file()
        || write.npm.file_name().and_then(|s| s.to_str()) != Some("npm")
        || !write.npm.parent().ok_or("npm parent missing")?.join("node").is_file()
        || read.search_paths != [write.prefix.join("bin")]
    {
        return Err("isolated write environment required".into());
    }
    Ok(())
}
pub fn discover(app: AppId, env: &LocalReadEnvironment) -> Result<AppInstallationReport, String> {
    report::build(read::read(app, env).map_err(|_| "host discovery unavailable")?.installations)
        .map_err(str::to_owned)
}
/// Read using administrator-supplied package ownership anchors, without granting writes.
pub fn scoped_discover(
    app: AppId,
    env: &LocalReadEnvironment,
    write: &WriteEnvironment,
) -> Result<AppInstallationReport, String> {
    let mut report = discover(app, env)?;
    // Source requires the exact reviewed package manifest and resolved ownership.
    if let Ok(args) = npm::install_args(app) {
        let package = args[2].trim_end_matches("@latest");
        let directory = write.prefix.join("lib/node_modules").join(package);
        if let Ok(canonical) = directory.canonicalize() {
            let manifest = std::fs::read(canonical.join("package.json"))
                .ok()
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
            if canonical.starts_with(&write.prefix)
                && manifest.as_ref().and_then(|v| v["name"].as_str()) == Some(package)
            {
                for inst in &mut report.installations {
                    if inst.real_path.as_ref().is_some_and(|p| Path::new(p).starts_with(&canonical))
                    {
                        inst.source = AppInstallationSource::System;
                    }
                }
            }
        }
    }
    Ok(report)
}
fn managers(write: &WriteEnvironment) -> Vec<String> {
    vec![write.npm.to_string_lossy().into_owned()]
}
fn selected(
    report: &AppInstallationReport,
    id: &AppInstallationId,
    write: &WriteEnvironment,
) -> Result<AppInstallation, String> {
    let inst = report.installations.iter().find(|i| &i.id == id).ok_or("installation not found")?;
    if !Path::new(&inst.path).starts_with(&write.prefix)
        || !inst.real_path.as_ref().is_some_and(|p| Path::new(p).starts_with(&write.prefix))
    {
        return Err("installation outside write prefix".into());
    }
    if inst.source == AppInstallationSource::Unknown {
        return Err("unverified installation source".into());
    }
    Ok(inst.clone())
}
pub fn preview(
    request: AppActionPreviewRequest,
    env: &LocalReadEnvironment,
    write: &WriteEnvironment,
) -> Result<Plan, String> {
    preview_with_node(request, env, write, node_preflight::check)
}
fn preview_with_node(
    request: AppActionPreviewRequest,
    env: &LocalReadEnvironment,
    write: &WriteEnvironment,
    node_check: impl FnOnce(&AppInstallation, &WriteEnvironment) -> Result<(), String>,
) -> Result<Plan, String> {
    validate(write, env)?;
    if request.app_id == AppId::Agy {
        super::local_agy::validate(&write.prefix)?;
    } else if request.app_id == AppId::Omp {
        super::local_omp::validate(&write.prefix)?;
    }
    let report = scoped_discover(request.app_id, env, write)?;
    let (source, action) = match (&request.action, &request.target) {
        (AppLifecycleAction::Install, AppActionTarget::Environment { environment }) => {
            let local = if cfg!(target_os = "macos") {
                AppInstallationEnvironment::Macos
            } else {
                AppInstallationEnvironment::Linux
            };
            if environment != &local {
                return Err("local environment executor required".into());
            }
            if !matches!(request.app_id, AppId::Agy | AppId::Omp) {
                npm::install_args(request.app_id).map_err(str::to_owned)?;
            }
            if !report.installations.is_empty() {
                return Err("installation already exists".into());
            }
            // Source is the administrator-approved installer target, not PATH inference.
            (AppInstallationSource::System, Action::Install)
        }
        (AppLifecycleAction::Update, AppActionTarget::Installation { installation_id }) => {
            let inst = selected(&report, installation_id, write)?;
            if !matches!(request.app_id, AppId::Agy | AppId::Omp) {
                node_check(&inst, write)?;
            }
            let plan =
                update::plan(&report, installation_id, &managers(write)).map_err(str::to_owned)?;
            (inst.source, Action::Update(plan))
        }
        (AppLifecycleAction::Uninstall, AppActionTarget::Installation { installation_id }) => {
            let inst = selected(&report, installation_id, write)?;
            (inst.source, Action::Uninstall(execute::preview(&inst, &managers(write), None)?))
        }
        _ => return Err("incompatible lifecycle target".into()),
    };
    Ok(Plan { request, source, action })
}
/// Runs inside Operation::start's existing host lock. Rebuilds the whole plan
/// immediately before execution; HTTP display strings never become commands.
pub fn run(plan: Plan, env: LocalReadEnvironment, write: WriteEnvironment) -> Result<(), String> {
    let fresh = preview(plan.request.clone(), &env, &write)?;
    if fresh != plan {
        return Err("installation changed since preview".into());
    }
    let (program, args, extra) = match &plan.action {
        Action::Install if plan.request.app_id == AppId::Agy => {
            let recipe = super::native_hermes_agy::agy_managed_install(&write.prefix.join("bin"));
            (recipe.program.into(), recipe.args, vec![])
        }
        Action::Install if plan.request.app_id == AppId::Omp => (
            "/bin/bash".into(),
            vec!["-c".into(), "(tmp=$(mktemp) || exit $?; trap 'rm -f \"$tmp\"' EXIT; curl -fsSL https://omp.sh/install -o \"$tmp\" && sh \"$tmp\" --binary)".into()],
            vec![("PI_INSTALL_DIR".into(), write.prefix.join("bin").to_string_lossy().into_owned())],
        ),
        Action::Install => (
            write.npm.to_string_lossy().into_owned(),
            npm::install_args(plan.request.app_id)?.iter().map(|s| (*s).into()).collect(),
            vec![],
        ),
        Action::Update(p) => {
            (p.command.program.clone(), p.command.args.clone(), p.command.env.clone())
        }
        Action::Uninstall(_) => return Err("uninstall must use its coordinated executor".into()),
    };
    let mut command = process::command(&program, &args, &[], None)?;
    let home = if plan.request.app_id == AppId::Agy {
        super::local_agy::home(&write.prefix)?
    } else if plan.request.app_id == AppId::Omp {
        super::local_omp::home(&write.prefix)?
    } else {
        write.prefix.clone()
    };
    let _npm_config = process::WriteProcessEnvironment::new(
        home.clone(),
        write.prefix.clone(),
        write.npm.parent().ok_or("npm parent missing")?.into(),
    )
    .apply(&mut command)?;
    command.envs(extra);
    if matches!(plan.request.app_id, AppId::Agy | AppId::Omp) {
        command.env("TMPDIR", &home);
    }
    process::run_command(command)?;
    if plan.request.app_id == AppId::Agy {
        let observed = discover(AppId::Agy, &env)?;
        if !observed.installations.iter().any(|i| {
            i.path == write.prefix.join("bin/agy").to_string_lossy()
                && i.runtime_status == AppInstallationRuntimeStatus::Runnable
        }) {
            return Err("native agy postcondition failed".into());
        }
        super::local_agy::record(&write.prefix)?;
    }
    if plan.request.app_id == AppId::Omp {
        let observed = discover(AppId::Omp, &env)?;
        if !observed.installations.iter().any(|i| {
            i.path == write.prefix.join("bin/omp").to_string_lossy()
                && i.runtime_status == AppInstallationRuntimeStatus::Runnable
        }) {
            return Err("native omp postcondition failed".into());
        }
        super::local_omp::record(&write.prefix)?;
    }
    let result = scoped_discover(plan.request.app_id, &env, &write)?;
    if matches!(plan.action, Action::Install)
        && !result.installations.iter().any(|i| i.source == AppInstallationSource::System)
    {
        return Err("installed package ownership not verified".into());
    }
    if let Action::Update(preview) = &plan.action
        && !preview.verify_result(&result)
    {
        return Err("selected installation postcondition failed".into());
    }
    if !result
        .installations
        .iter()
        .any(|i| i.runtime_status == AppInstallationRuntimeStatus::Runnable)
    {
        return Err("installation postcondition failed".into());
    }
    Ok(())
}
/// Uninstall owns its coordinator; do not nest it under a second write lock.
pub async fn uninstall<G>(
    plan: Plan,
    env: LocalReadEnvironment,
    write: WriteEnvironment,
    authorize: G,
) -> Result<(), String>
where
    G: FnOnce() -> Result<(), String> + Send + 'static,
{
    let Action::Uninstall(removal) = plan.action else {
        return Err("uninstall plan required".into());
    };
    let AppActionTarget::Installation { installation_id } = plan.request.target else {
        return Err("installation required".into());
    };
    let process_env = process::WriteProcessEnvironment::new(
        write.prefix.clone(),
        write.prefix.clone(),
        write.npm.parent().ok_or("npm parent missing")?.into(),
    );
    execute::uninstall_isolated(
        removal,
        true,
        move || {
            authorize()?;
            validate(&write, &env)?;
            let report = scoped_discover(plan.request.app_id, &env, &write)?;
            Ok((selected(&report, &installation_id, &write)?, managers(&write), None))
        },
        process_env,
    )
    .await
}

#[path = "local_node.rs"]
mod node_preflight;
#[cfg(test)]
#[path = "local_node_tests.rs"]
mod node_tests;

#[cfg(all(test, unix))]
#[path = "local_agy_tests.rs"]
mod agy_tests;

#[cfg(all(test, unix))]
#[path = "local_omp_tests.rs"]
mod omp_tests;
