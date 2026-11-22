//! Bind the selected view and prepare OpenCode's confined native scanner.
use agent_config::schema::AcpAgentConfig;

const ROOT_ENV: &str = "SHIJU_OPENCODE_SKILL_ROOT";

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&super::MountedSkills>,
) -> Result<(), super::Error> {
    // A reviewed ELF identifies the scanner but grants no confinement for the
    // selected directories. Keep projections out of the native Linux process.
    if cfg!(target_os = "linux") && view.is_some() {
        return Err(super::super::acp_option_error(
            "OpenCode skill mounts require a verified Linux sandbox",
        ));
    }
    if config.env.contains_key(ROOT_ENV) || std::env::var_os(ROOT_ENV).is_some() {
        return Err(super::super::acp_option_error(
            "conflicting OpenCode skill directory environment",
        ));
    }
    let root = if let Some(view) = view {
        view.0.validate().map_err(super::super::to_api_error)?;
        acp_agent_registry::verify_reviewed_agent_executable("opencode", &config.command)
            .map_err(super::super::to_api_error)?;
        view.0.root().to_string_lossy().into_owned()
    } else {
        // The confined scanner keeps its native roots when this view is empty.
        String::new()
    };
    config.env.insert(ROOT_ENV.into(), root);
    config.env.insert("OPENCODE_PURE".into(), "1".into());
    Ok(())
}

#[cfg(test)]
#[path = "opencode_tests.rs"]
mod tests;

pub(in crate::acp) fn sandbox(
    profile: &super::super::model_source::profile::AcpLaunchProfile,
    config: &mut AcpAgentConfig,
    cwd: &std::path::Path,
    view: Option<&impl super::super::host::SkillLease>,
) -> anyhow::Result<acp_client::OpenCodeSandbox> {
    use std::path::Path;
    let workspace = cwd.canonicalize()?;
    let directory = profile.directory().canonicalize()?;
    let state_root = profile
        .state_root
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing OpenCode state root"))?
        .canonicalize()?;
    let disjoint = |path: &Path| !path.starts_with(&workspace) && !workspace.starts_with(path);
    anyhow::ensure!(
        disjoint(&directory) && disjoint(&state_root) && workspace.parent().is_some(),
        "Restricted OpenCode requires a workspace separate from private state"
    );
    let inline: serde_json::Value = serde_json::from_str(
        config
            .env
            .get("OPENCODE_CONFIG_CONTENT")
            .ok_or_else(|| anyhow::anyhow!("Missing OpenCode model configuration"))?,
    )?;
    anyhow::ensure!(
        inline.pointer("/provider/session-provider/npm").and_then(serde_json::Value::as_str)
            == Some("@ai-sdk/openai-compatible"),
        "Restricted OpenCode requires Chat Completions"
    );
    let (root, sources) = view.map_or_else(
        || Ok::<_, anyhow::Error>((None, Vec::new())),
        |view| {
            view.validate().map_err(|error| anyhow::anyhow!(error.to_string()))?;
            let (root, sources) = selection(view.root())?;
            view.validate().map_err(|error| anyhow::anyhow!(error.to_string()))?;
            Ok((Some(root), sources))
        },
    )?;
    anyhow::ensure!(
        sources
            .iter()
            .all(|source| !state_root.starts_with(source) && !directory.starts_with(source)),
        "OpenCode skill source overlaps private runtime state"
    );
    config.env.insert(
        ROOT_ENV.into(),
        root.as_deref()
            .map(Path::to_str)
            .unwrap_or(Some(""))
            .ok_or_else(|| anyhow::anyhow!("Invalid OpenCode skill root"))?
            .into(),
    );
    config.env.insert("SHIJU_OPENCODE_SKILL_SOURCES".into(), serde_json::to_string(&sources)?);
    for (name, contents) in [
        (
            "opencode-launcher.py",
            include_str!("../../../../../../patches/acp-model-source/opencode/launcher.py"),
        ),
        (
            "omp-mcp-bridge.py",
            include_str!("../../../../../../patches/acp-model-source/omp/mcp_bridge.py"),
        ),
        (
            "omp-mcp-stdio.py",
            include_str!("../../../../../../patches/acp-model-source/cortex-code/mcp_bridge.py"),
        ),
    ] {
        profile.write_config(name, contents.as_bytes())?;
    }
    Ok(acp_client::OpenCodeSandbox { directory, workspace })
}

fn selection(
    root: &std::path::Path,
) -> anyhow::Result<(std::path::PathBuf, Vec<std::path::PathBuf>)> {
    let root = root.canonicalize()?;
    let mut sources = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        anyhow::ensure!(entry.file_type()?.is_symlink(), "Invalid OpenCode skill projection");
        let source = entry.path().canonicalize()?;
        anyhow::ensure!(
            source.is_dir() && source.parent().is_some(),
            "Invalid OpenCode skill source"
        );
        sources.push(source);
    }
    sources.sort();
    Ok((root, sources))
}
