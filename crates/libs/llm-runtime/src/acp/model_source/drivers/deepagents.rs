//! Reviewed DeepAgents 0.1.42 uses an explicit OpenAI Chat model and private relay.
use super::AcpLaunchProfile;
use std::collections::HashMap;

pub fn configure(
    profile: &mut AcpLaunchProfile,
    endpoint: &str,
    directory: &str,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args.is_empty(),
        "DeepAgents custom arguments are not verified"
    );
    anyhow::ensure!(
        profile.config().env.keys().all(|key| key == super::super::profile::REFERENCE_ENV),
        "DeepAgents custom environment is not verified"
    );
    let url = reqwest::Url::parse(endpoint)?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some()
            && url.path() == "/"
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "DeepAgents requires a private loopback relay"
    );
    anyhow::ensure!(
        profile.model_supports_tool_call
            && !profile.model_alias.is_empty()
            && !profile.model_alias.chars().any(char::is_whitespace)
            && !profile.model_alias.contains('='),
        "DeepAgents requires a selected tool-capable model with a literal model ID"
    );
    profile.overlay_environment(HashMap::from([
        ("OPENAI_BASE_URL".into(), format!("{}/v1", endpoint.trim_end_matches('/'))),
        ("OPENAI_API_KEY".into(), profile.binding.bearer_token().into()),
        ("LANGCHAIN_TRACING_V2".into(), "false".into()),
        ("LANGSMITH_TRACING".into(), "false".into()),
    ]))?;
    profile.append_arguments(&[
        "--model".into(),
        format!("openai:{}", profile.model_alias),
        "--workspace".into(),
        directory.into(),
    ])
}
