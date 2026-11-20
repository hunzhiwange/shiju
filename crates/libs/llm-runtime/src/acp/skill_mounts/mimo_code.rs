//! Add the request-owned selection to MiMo's native scanner.
use super::super::{
    Error, host::SkillLease, model_source::profile::AcpLaunchProfile, to_api_error,
};
use agent_config::schema::AcpAgentConfig;
use serde_json::json;
use std::{fs, path::Path};

const CONFIG_CONTENT: &str = "MIMOCODE_CONFIG_CONTENT";
const SANDBOX: &str = "SHIJU_MIMO_SANDBOX";
pub(in crate::acp) const POLICY_FILE: &str = "mimo-code.sb";

pub(in crate::acp) fn configure(
    profile: &AcpLaunchProfile,
    config: &mut AcpAgentConfig,
    view: Option<&impl SkillLease>,
) -> Result<(), Error> {
    let Some(view) = view else {
        return Ok(());
    };
    view.validate()?;
    let result = (|| -> anyhow::Result<()> {
        let root = view.root().canonicalize()?;
        let policy = fs::read_to_string(profile.directory().join(POLICY_FILE))?;
        let (contents, policy) = selection(&root, policy)?;
        let policy = profile.write_config("mimo-code-selected.sb", policy.as_bytes())?;
        config.env.insert(SANDBOX.into(), policy.to_string_lossy().into_owned());
        config.env.insert(CONFIG_CONTENT.into(), contents);
        Ok(())
    })();
    result.map_err(to_api_error)
}

fn selection(root: &Path, mut policy: String) -> anyhow::Result<(String, String)> {
    let mut paths = vec![root.to_owned()];
    for entry in fs::read_dir(root)? {
        paths.push(fs::read_link(entry?.path())?);
    }
    for path in paths {
        let path = path.to_str().ok_or_else(|| anyhow::anyhow!("Non-UTF-8 MiMo skill path"))?;
        anyhow::ensure!(!path.contains(['{', '}', '\n', '\r']), "Invalid MiMo skill path");
        let quoted = serde_json::to_string(path)?;
        policy.push_str(&format!("\n(allow file-read-data (subpath {quoted}))\n"));
    }
    Ok((json!({"skills": {"paths": [root]}}).to_string(), policy))
}

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "mimo_code_tests.rs"]
mod tests;
