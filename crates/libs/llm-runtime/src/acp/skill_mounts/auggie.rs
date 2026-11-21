//! Configure the restricted Auggie process from the request-owned skill lease.
use super::super::{host::SkillLease, model_source::profile::AcpLaunchProfile};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(in crate::acp) fn sandbox(
    profile: &mut AcpLaunchProfile,
    cwd: &Path,
    view: Option<&impl SkillLease>,
) -> anyhow::Result<acp_client::AuggieSandbox> {
    let workspace = cwd.canonicalize()?;
    let launch = profile.directory().canonicalize()?;
    let home = launch.join("home");
    let entry = Path::new(&profile.config().command).canonicalize()?;
    let history =
        super::super::model_source::drivers::auggie::history_directory(profile)?.canonicalize()?;
    anyhow::ensure!(
        workspace.is_dir()
            && workspace.parent().is_some()
            && !launch.starts_with(&workspace)
            && !workspace.starts_with(&launch)
            && !history.starts_with(&workspace)
            && !workspace.starts_with(&history)
            && !entry.starts_with(&workspace),
        "Auggie workspace overlaps the private runtime"
    );
    for key in ["HOME", "VIBEWINDOW_CONFIG_DIR"] {
        if let Some(directory) = std::env::var_os(key) {
            let private = PathBuf::from(directory).canonicalize()?;
            anyhow::ensure!(
                !private.starts_with(&workspace),
                "Auggie workspace contains a private host directory"
            );
        }
    }
    let port =
        profile.relay_address().ok_or_else(|| anyhow::anyhow!("Missing Auggie relay"))?.port();

    let (root, skill_roots) = selection(view)?;
    fs::create_dir(&home)?;
    preserve_disabled_skills(&home)?;
    profile.overlay_environment(std::collections::HashMap::from([(
        "SHIJU_AUGGIE_SKILL_VIEW".into(),
        root,
    )]))?;
    Ok(acp_client::AuggieSandbox {
        workspace,
        home,
        launch,
        history,
        skill_roots,
        relay_port: port,
    })
}

fn selection(view: Option<&impl SkillLease>) -> anyhow::Result<(String, Vec<PathBuf>)> {
    let Some(view) = view else {
        return Ok((String::new(), Vec::new()));
    };
    view.validate().map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let root = view.root().canonicalize()?;
    let mut paths = vec![root.clone()];
    for entry in fs::read_dir(&root)? {
        let target = entry?.path().canonicalize()?;
        anyhow::ensure!(target.is_dir(), "Invalid Auggie selected skill directory");
        paths.push(target);
    }
    view.validate().map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let root = root.to_str().ok_or_else(|| anyhow::anyhow!("Non-UTF-8 Auggie skill path"))?;
    Ok((root.into(), paths))
}

fn preserve_disabled_skills(home: &Path) -> anyhow::Result<()> {
    let Some(user_home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Ok(());
    };
    let mut modes = serde_json::Map::new();
    for file in ["disabled-skills.json", "feature-config.json"] {
        match fs::read(user_home.join(".augment").join(file)) {
            Ok(data) => {
                if let Ok(config) = serde_json::from_slice::<serde_json::Value>(&data)
                    && let Some(configured) =
                        config.get("skillModes").and_then(|value| value.as_object())
                {
                    for (skill, mode) in configured {
                        if mode == "disabled" {
                            modes.insert(skill.clone(), mode.clone());
                        } else {
                            modes.remove(skill);
                        }
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    if !modes.is_empty() {
        let directory = home.join(".augment");
        fs::create_dir(&directory)?;
        fs::write(
            directory.join("feature-config.json"),
            serde_json::to_vec(&serde_json::json!({"skillModes": modes}))?,
        )?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "auggie_tests.rs"]
mod tests;
