//! Bind the fixed OMP release to a private model relay and host-owned file writes.
use super::{super::profile::REFERENCE_ENV, AcpLaunchProfile};
use crate::acp::skill_mounts::omp::{POLICY_FILE, SANDBOX_ENV, SETTINGS_ENV, settings};
use serde_json::json;
use std::{collections::HashMap, path::Path};

const PROVIDER: &str = "shiju-session";

pub fn configure(
    profile: &mut AcpLaunchProfile,
    endpoint: &str,
    directory: &str,
) -> anyhow::Result<()> {
    let launch = profile.config();
    anyhow::ensure!(
        launch.args == ["acp"] && launch.env.keys().all(|key| key == REFERENCE_ENV),
        "OMP requires its reviewed ACP launch"
    );
    acp_agent_registry::verify_reviewed_agent_executable("omp", &launch.command)
        .map_err(anyhow::Error::msg)?;
    let executable = Path::new(&launch.command).canonicalize()?;
    let workspace = Path::new(directory).canonicalize()?;
    let root = profile.directory().canonicalize()?;
    anyhow::ensure!(
        workspace.is_dir()
            && workspace.parent().is_some()
            && !root.starts_with(&workspace)
            && !workspace.starts_with(&root)
            && !executable.starts_with(&workspace),
        "OMP requires a workspace separate from the managed runtime"
    );
    let url = reqwest::Url::parse(endpoint)?;
    let port = url.port().ok_or_else(|| anyhow::anyhow!("Missing OMP relay port"))?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.path() == "/"
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "OMP requires a private loopback relay"
    );
    anyhow::ensure!(
        profile.model_supports_tool_call && !profile.model_alias.is_empty(),
        "OMP requires a selected tool-capable model"
    );

    let home = root.join("home");
    let agent_dir = home.join(".omp/agent");
    std::fs::create_dir_all(&agent_dir)?;
    let model = &profile.model_alias;
    // JSON is a subset of YAML. OMP adds the local provider prefix to the ACP
    // option but sends the model id verbatim to the relay.
    let models = json!({
        "providers": { (PROVIDER): {
            "baseUrl": url.join("v1")?.as_str(),
            "api": "openai-completions",
            "apiKey": "SHIJU_OMP_RELAY_TOKEN",
            "models": [{
                "id": model,
                "name": "ShiJu Session Model",
                "contextWindow": profile.model_context_limit.max(1),
                "maxTokens": profile.model_output_limit.max(1)
            }]
        }}
    });
    let models = serde_json::to_vec(&models)?;
    std::fs::rename(
        profile.write_config("models.yml", &models)?,
        agent_dir.join("models.yml"),
    )?;
    profile.write_config("omp-models-source.json", &models)?;
    for (name, contents) in [
        ("omp-launcher.py", include_str!("../../../../../../../patches/acp-model-source/omp/launcher.py")),
        ("omp-mcp-bridge.py", include_str!("../../../../../../../patches/acp-model-source/omp/mcp_bridge.py")),
        ("omp-mcp-stdio.py", include_str!("../../../../../../../patches/acp-model-source/cortex-code/mcp_bridge.py")),
    ] {
        profile.write_config(name, contents.as_bytes())?;
    }
    let quote = |path: &Path| -> anyhow::Result<String> {
        Ok(serde_json::to_string(
            path.to_str().ok_or_else(|| anyhow::anyhow!("Non-UTF-8 OMP path"))?,
        )?)
    };
    let skill_settings = profile.write_config("omp-native-skills.yml", &settings(None)?)?;
    let skill_settings = skill_settings.canonicalize()?;
    let policy = format!(
        "(version 1)\n(allow default)\n\
         (deny file-read-data (subpath \"/Users\") (subpath \"/Volumes\") (subpath \"/private/tmp\") (subpath \"/private/var/folders\"))\n\
         (allow file-read-data (subpath {workspace}) (subpath {home}) (literal {executable}) (literal {skill_settings}))\n\
         (deny file-write*)\n(allow file-write* (subpath {home}) (literal \"/dev/null\"))\n\
         (deny process-fork process-exec)\n(allow process-exec (literal {executable}))\n\
         (deny network*)\n(allow network-outbound (remote ip \"localhost:{port}\"))\n",
        workspace = quote(&workspace)?,
        home = quote(&home)?,
        executable = quote(&executable)?,
        skill_settings = quote(&skill_settings)?,
    );
    let policy = profile.write_config(POLICY_FILE, policy.as_bytes())?;
    profile.overlay_environment(HashMap::from([
        (SANDBOX_ENV.into(), policy.to_string_lossy().into_owned()),
        (SETTINGS_ENV.into(), skill_settings.to_string_lossy().into_owned()),
        ("SHIJU_OMP_HOME".into(), home.to_string_lossy().into_owned()),
        ("SHIJU_OMP_WORKSPACE".into(), workspace.to_string_lossy().into_owned()),
        ("PI_CODING_AGENT_DIR".into(), agent_dir.to_string_lossy().into_owned()),
        ("SHIJU_OMP_RELAY_TOKEN".into(), profile.binding.bearer_token().into()),
        ("NO_PROXY".into(), "127.0.0.1,localhost".into()),
    ]))?;
    profile.session_model_id = Some(format!("{PROVIDER}/{}", profile.model_alias));
    Ok(())
}
