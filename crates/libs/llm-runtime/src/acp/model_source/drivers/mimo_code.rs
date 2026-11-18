//! Bind the reviewed MiMo Code process to the request-owned relay and workspace.
use super::{super::profile::REFERENCE_ENV, AcpLaunchProfile};
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
        "MiMo Code requires its reviewed ACP launch"
    );
    acp_agent_registry::verify_reviewed_agent_executable("mimo-code", &launch.command)
        .map_err(anyhow::Error::msg)?;
    let executable = Path::new(&launch.command).canonicalize()?;
    let workspace = Path::new(directory).canonicalize()?;
    anyhow::ensure!(
        workspace.is_dir() && workspace.parent().is_some(),
        "Invalid MiMo Code workspace"
    );
    let url = reqwest::Url::parse(endpoint)?;
    let port = url.port().ok_or_else(|| anyhow::anyhow!("Missing MiMo Code relay port"))?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.path() == "/"
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "MiMo Code requires a private loopback relay"
    );
    anyhow::ensure!(
        profile.model_supports_tool_call
            && !profile.model_alias.is_empty()
            && !profile.model_alias.contains(['{', '}', '\0']),
        "MiMo Code requires a literal selected tool-capable model"
    );

    super::opencode::reject_managed_configuration("mimocode", "MIMOCODE_TEST_MANAGED_CONFIG_DIR")?;
    let root = profile.directory().canonicalize()?;
    anyhow::ensure!(
        !root.starts_with(&workspace)
            && !workspace.starts_with(&root)
            && !executable.starts_with(&workspace),
        "MiMo Code workspace overlaps private runtime files"
    );
    let home = root.join("home");
    std::fs::create_dir(&home)?;
    let model = format!("{PROVIDER}/{}", profile.model_alias);
    let config = json!({
        "model": model, "small_model": model,
        "enabled_providers": [PROVIDER],
        "provider": { (PROVIDER): {
            "name": "ShiJu Session Model",
            "npm": "@ai-sdk/openai-compatible",
            "only_configured_models": true,
            "options": {
                "baseURL": url.join("v1")?.as_str(), "apiKey": "{env:SHIJU_MIMO_RELAY_TOKEN}"
            },
            "models": { (profile.model_alias.clone()): {
                "name": "Session model", "tool_call": true,
                "limit": {
                    "context": profile.model_context_limit.max(1),
                    "output": profile.model_output_limit.max(1)
                }
            }}
        }},
        "permission": {"*": "deny", "read": "allow", "list": "allow",
            "glob": "allow", "grep": "allow", "edit": "ask"},
        "autoupdate": false, "share": "disabled"
    });
    let config = profile.write_config("mimocode.json", &serde_json::to_vec(&config)?)?;
    let quote = |path: &Path| -> anyhow::Result<String> {
        Ok(serde_json::to_string(
            path.to_str().ok_or_else(|| anyhow::anyhow!("Non-UTF-8 MiMo Code path"))?,
        )?)
    };
    let policy = format!(
        "(version 1)\n(allow default)\n(deny file-read-data (subpath \"/Users\") (subpath \"/Volumes\") (subpath \"/private/tmp\") (subpath \"/private/var/folders\"))\n\
         (allow file-read-data (subpath \"/System\") (subpath \"/usr/lib\") (subpath \"/usr/share\")\n\
         (subpath \"/dev\") (subpath {workspace}) (subpath {home}) (literal {executable}) (literal {config}))\n\
         (deny file-write*)\n(allow file-write* (subpath {home}) (literal \"/dev/null\"))\n\
         (deny process-fork process-exec)\n(allow process-exec (literal {executable}))\n\
         (deny network*)\n\
         (allow network-outbound (remote ip \"localhost:{port}\"))\n",
        workspace = quote(&workspace)?,
        home = quote(&home)?,
        executable = quote(&executable)?,
        config = quote(&config.canonicalize()?)?,
    );
    let policy = profile.write_config("mimo-code.sb", policy.as_bytes())?;
    profile.overlay_environment(HashMap::from([
        ("SHIJU_MIMO_SANDBOX".into(), policy.to_string_lossy().into_owned()),
        ("SHIJU_MIMO_HOME".into(), home.to_string_lossy().into_owned()),
        ("SHIJU_MIMO_WORKSPACE".into(), workspace.to_string_lossy().into_owned()),
        ("SHIJU_MIMO_RELAY_TOKEN".into(), profile.binding.bearer_token().into()),
        ("MIMOCODE_CONFIG".into(), config.to_string_lossy().into_owned()),
        ("MIMOCODE_DISABLE_PROJECT_CONFIG".into(), "1".into()),
        ("MIMOCODE_DISABLE_CLAUDE_CODE".into(), "1".into()),
        ("MIMOCODE_DISABLE_CLAUDE_CODE_COMMANDS".into(), "1".into()),
        ("MIMOCODE_DISABLE_GIT".into(), "1".into()),
        ("MIMOCODE_DISABLE_AUTOUPDATE".into(), "1".into()),
        ("MIMOCODE_DISABLE_MODELS_FETCH".into(), "1".into()),
        ("NO_PROXY".into(), "127.0.0.1,localhost".into()),
    ]))?;
    profile.session_model_id = Some(model);
    Ok(())
}
