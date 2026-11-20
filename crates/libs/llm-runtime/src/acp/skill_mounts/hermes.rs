//! Bind the reviewed Hermes scanner and restrict its native skill read roots.
use agent_config::schema::AcpAgentConfig;

const ROOT_ENV: &str = "SHIJU_HERMES_SKILL_ROOT";

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&super::MountedSkills>,
) -> Result<(), super::Error> {
    if config.env.contains_key(ROOT_ENV) {
        return Err(super::super::acp_option_error(
            "conflicting Hermes skill directory environment",
        ));
    }
    let root = if let Some(view) = view {
        view.0.validate().map_err(super::super::to_api_error)?;
        acp_agent_registry::verify_reviewed_agent_executable("hermes", &config.command)
            .map_err(super::super::to_api_error)?;
        view.0.root().to_string_lossy().into_owned()
    } else {
        // Keep workspace-native discovery when no ShiJu view is selected.
        String::new()
    };
    config.env.insert(ROOT_ENV.into(), root);
    Ok(())
}

#[cfg(test)]
#[path = "hermes_tests.rs"]
mod tests;

pub(in crate::acp) fn sandbox(
    profile: &super::super::model_source::profile::AcpLaunchProfile,
    cwd: &std::path::Path,
    view: Option<&impl super::super::host::SkillLease>,
) -> anyhow::Result<acp_client::HermesSandbox> {
    #[cfg(not(unix))]
    anyhow::bail!("Restricted Hermes requires Darwin arm64");
    #[cfg(unix)]
    {
        use std::{
            fs,
            os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt},
        };
        let workspace = cwd.canonicalize()?;
        let home = profile.directory().canonicalize()?;
        let env = &profile.config().env;
        anyhow::ensure!(
            env.keys().all(|name| matches!(
                name.as_str(),
                "VW_ACP_LAUNCH_REFERENCE" | "HERMES_HOME" | "HERMES_HOME_MODE" | "CUSTOM_BASE_URL"
            )),
            "Restricted Hermes requires its generated launch environment"
        );
        let skill_roots = publish_skills(&home, &workspace, view)?;
        let reference = env["VW_ACP_LAUNCH_REFERENCE"]
            .strip_prefix("vw-session-model:")
            .filter(|reference| {
                reference.len() == 64 && reference.bytes().all(|b| b.is_ascii_hexdigit())
            })
            .ok_or_else(|| anyhow::anyhow!("Missing private Hermes history reference"))?;
        // Match the vendor's existing state location so managed history remains resumable.
        let mut history = std::env::temp_dir().canonicalize()?;
        let uid = unsafe { libc::geteuid() };
        for component in [format!("shiju-hermes-state-{uid}").as_str(), reference] {
            history.push(component);
            match fs::DirBuilder::new().mode(0o700).create(&history) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.into()),
            }
            let metadata = history.symlink_metadata()?;
            anyhow::ensure!(
                metadata.is_dir()
                    && metadata.uid() == uid
                    && metadata.permissions().mode() & 0o777 == 0o700,
                "Insecure Hermes history directory"
            );
        }
        let relay = reqwest::Url::parse(&env["CUSTOM_BASE_URL"])?;
        anyhow::ensure!(
            relay.host_str() == Some("127.0.0.1"),
            "Hermes sandbox requires local relay"
        );
        let relay_port =
            relay.port().ok_or_else(|| anyhow::anyhow!("Missing Hermes relay port"))?;
        Ok(acp_client::HermesSandbox { workspace, home, history, skill_roots, relay_port })
    }
}

#[cfg(unix)]
fn publish_skills(
    home: &std::path::Path,
    workspace: &std::path::Path,
    view: Option<&impl super::super::host::SkillLease>,
) -> anyhow::Result<Vec<std::path::PathBuf>> {
    use std::{fs, os::unix::fs::symlink};
    let mut skill_roots = Vec::new();
    let native = workspace.join(".hermes/skills");
    match native.symlink_metadata() {
        Ok(metadata) => {
            anyhow::ensure!(
                metadata.is_dir() && native.canonicalize()? == native,
                "Hermes native skill root must be a workspace directory"
            );
            symlink(&native, home.join("skills"))?;
            skill_roots.push(native);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    if let Some(view) = view {
        view.validate().map_err(|error| anyhow::anyhow!(error.to_string()))?;
        let root = view.root().canonicalize()?;
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            let target = entry.path().canonicalize()?;
            anyhow::ensure!(target.is_dir(), "Invalid selected Hermes skill directory");
            skill_roots.push(target);
        }
        view.validate().map_err(|error| anyhow::anyhow!(error.to_string()))?;
        skill_roots.push(root);
    }
    Ok(skill_roots)
}
