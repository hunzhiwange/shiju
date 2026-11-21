use super::AcpLaunchProfile;
use serde_json::json;
use std::collections::HashMap;
use model_reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};

pub const PROJECT_SKILLS_ENV: &str = "SHIJU_COPILOT_PROJECT_SKILLS";
const NATIVE_PROJECT_ROOTS: [&str; 3] = [".github/skills", ".claude/skills", ".agents/skills"];

pub fn configure(profile: &mut AcpLaunchProfile, endpoint: &str, workspace: &str) -> anyhow::Result<()> {
    let url = reqwest::Url::parse(endpoint)?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && url.path() == "/",
        "Copilot requires a loopback relay endpoint"
    );
    let config = profile.config();
    let program = std::path::Path::new(config.command.trim()).file_name().and_then(|s| s.to_str());
    let [acp, stdio, tool_filters @ ..] = config.args.as_slice() else {
        anyhow::bail!("Copilot managed launch requires verified ACP arguments");
    };
    // Accept only the native availability filter with literal tool names. It does
    // not grant approvals, and a launch without filters keeps Copilot's defaults.
    anyhow::ensure!(
        matches!(program, Some("copilot" | "copilot.cmd" | "copilot.exe"))
            && acp == "--acp"
            && stdio == "--stdio"
            && tool_filters.iter().all(|arg| {
                arg.strip_prefix("--available-tools=").is_some_and(|name| {
                    !name.is_empty()
                        && !name.starts_with('-')
                        && name.bytes().all(|byte| {
                            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
                        })
                })
            }),
        "Copilot managed launch requires verified ACP arguments"
    );
    anyhow::ensure!(
        !config.env.keys().any(|key| {
            let key = key.to_ascii_uppercase();
            key.starts_with("COPILOT_")
                || matches!(
                    key.as_str(),
                    "GH_TOKEN"
                        | "GITHUB_TOKEN"
                        | "GH_COPILOT_TOKEN"
                        | "NODE_OPTIONS"
                        | "NODE_PATH"
                        | "HTTP_PROXY"
                        | "HTTPS_PROXY"
                        | "ALL_PROXY"
                        | "NO_PROXY"
                )
        }),
        "conflicting Copilot provider or authentication environment"
    );
    if let Some(project) = config.env.get(PROJECT_SKILLS_ENV) {
        let workspace = std::path::Path::new(workspace).canonicalize()?;
        anyhow::ensure!(std::path::Path::new(project) == workspace,
            "Copilot native skills require an explicit grant for the bound project");
        acp_agent_registry::verify_reviewed_agent_executable("copilot", &config.command)
            .map_err(anyhow::Error::msg)?;
        let settings = project_skill_settings(&workspace)?;
        profile.write_config("settings.json", &serde_json::to_vec(&settings)?)?;
    }
    let api = profile
        .model_api
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Copilot requires selected model transport metadata"))?;
    let (provider_type, base_url) = match api.adapter.as_str() {
        "anthropic" => ("anthropic", endpoint.trim_end_matches('/').to_string()),
        "openai" | "openai-compatible" => {
            let upstream = reqwest::Url::parse(&api.url)?;
            anyhow::ensure!(
                !upstream.path().trim_end_matches('/').ends_with("/responses"),
                "Copilot Responses conversion is not verified"
            );
            ("openai", format!("{}/v1", endpoint.trim_end_matches('/')))
        }
        _ => anyhow::bail!("Copilot selected model protocol is unsupported"),
    };
    anyhow::ensure!(
        profile.model_supports_tool_call && profile.model_supports_temperature,
        "Copilot requires a selected model supporting tools and temperature"
    );
    // Both provider routes use their native wire protocol. Reasoning support is
    // a model capability, not a request to convert or enable reasoning; the relay
    // validates the actual request parameters against the selected model.
    let model = &profile.model_alias;
    let mut native_model = json!({"id": model, "provider": "vibewindow", "wireModel": model,
        "maxOutputTokens": profile.model_output_limit});
    let provider = &profile.binding.identity().provider;
    if profile.model_supports_reasoning
        && let Some(declared) = profile.model_reasoning_options.as_deref()
        && model_protocol::endpoint_urls::chat_url(api).is_ok()
        && reasoning_profile(&api.id, &api.adapter, provider, &api.url).format
            == ReasoningFormat::OpenAi
    {
        let mut levels =
            reasoning_levels(&api.id, &api.adapter, provider, &api.url, Some(declared));
        // Copilot 1.0.105 omits the Chat field for "none", leaving the upstream default.
        levels.retain(|level| level != "none");
        if !levels.is_empty() {
            native_model["capabilities"] = json!({"supports": {"reasoning_effort": levels}});
        }
    }
    let content = serde_json::to_vec(&json!({
        "providers": [{"name": "vibewindow", "type": provider_type, "baseUrl": base_url,
            "apiKey": profile.binding.bearer_token(), "wireApi": "completions"}],
        "models": [native_model]
    }))?;
    let directory = profile.directory().to_string_lossy().into_owned();
    profile.overlay_environment(HashMap::from([
        ("COPILOT_HOME".into(), directory.clone()),
        ("COPILOT_PROVIDERS_CONFIG".into(), format!("{directory}/providers.json")),
        ("COPILOT_MODEL".into(), format!("vibewindow/{model}")),
        ("COPILOT_OFFLINE".into(), "true".into()),
        ("COPILOT_AUTO_UPDATE".into(), "false".into()),
    ]))?;
    preserve_history(profile)?;
    profile.append_arguments(&["--no-auto-update".into()])?;
    profile.write_config("providers.json", &content)?;
    Ok(())
}

// Project scope comes from the host launch, never from the settings document.
fn project_skill_settings(workspace: &std::path::Path) -> anyhow::Result<ProjectSkillSettings> {
    use std::io::Read;
    let workspace = workspace.canonicalize()?;
    anyhow::ensure!(workspace.is_dir() && workspace.parent().is_some(), "invalid Copilot project");
    let mut source = open_project_skill_settings(&workspace)?.take(65537);
    let mut bytes = Vec::new();
    source.read_to_end(&mut bytes)?;
    anyhow::ensure!(bytes.len() <= 65536, "Copilot project skill settings exceed 64 KiB");
    let mut settings: ProjectSkillSettings = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow::anyhow!("invalid Copilot project skill settings"))?;
    for root in &mut settings.skill_directories {
        anyhow::ensure!(
            NATIVE_PROJECT_ROOTS.contains(&root.as_str()),
            "Copilot project skill root must be a native project directory"
        );
        let path = project_skill_path(&workspace, root)?;
        check_skill_tree(&path)?;
        *root = path.to_str().ok_or_else(|| anyhow::anyhow!("non-UTF-8 Copilot skill directory"))?.into();
    }
    for location in &mut settings.ignored_skills_locations {
        anyhow::ensure!(
            NATIVE_PROJECT_ROOTS.contains(&location.as_str()),
            "Copilot ignored locations must be native project roots; use disabledSkills for names"
        );
        let path = workspace.join(&*location);
        *location = path.to_str().ok_or_else(|| anyhow::anyhow!("non-UTF-8 Copilot ignored location"))?.into();
    }
    Ok(settings)
}

#[cfg(unix)]
fn open_project_skill_settings(workspace: &std::path::Path) -> anyhow::Result<std::fs::File> {
    use std::{fs::File, os::{fd::{AsRawFd, FromRawFd}, unix::fs::MetadataExt}};
    let mut file = File::open(workspace)?;
    for (name, flags) in [
        (c".github", libc::O_DIRECTORY),
        (c"copilot", libc::O_DIRECTORY),
        (c"settings.json", libc::O_NONBLOCK),
    ] {
        // Keep each parent open until its child is resolved. No component can
        // be redirected through a symlink while the settings are opened.
        let fd = unsafe {
            libc::openat(file.as_raw_fd(), name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | flags)
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // SAFETY: openat returned a new owned descriptor.
        file = unsafe { File::from_raw_fd(fd) };
    }
    let metadata = file.metadata()?;
    anyhow::ensure!(
        metadata.is_file() && metadata.nlink() == 1,
        "Copilot project skill settings must be a regular file without hard links"
    );
    Ok(file)
}

#[cfg(not(unix))]
fn open_project_skill_settings(_: &std::path::Path) -> anyhow::Result<std::fs::File> {
    anyhow::bail!("isolated Copilot project settings are unavailable on this platform")
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct ProjectSkillSettings {
    skill_directories: Vec<String>,
    disabled_skills: Vec<String>,
    ignored_skills_locations: Vec<String>,
}

fn project_skill_path(workspace: &std::path::Path, source: &str) -> anyhow::Result<std::path::PathBuf> {
    use std::path::{Component, Path};
    let mut result = workspace.to_owned();
    anyhow::ensure!(!source.is_empty(), "empty Copilot project skill path");
    for component in Path::new(source).components() {
        let Component::Normal(component) = component else {
            anyhow::bail!("Copilot project skill paths must be relative to the bound project");
        };
        result.push(component);
        anyhow::ensure!(
            !result.symlink_metadata()?.file_type().is_symlink(),
            "Copilot project skill paths cannot use symbolic links"
        );
    }
    Ok(result)
}

fn check_skill_tree(root: &std::path::Path) -> anyhow::Result<()> {
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                pending.push(entry.path());
            } else {
                anyhow::ensure!(kind.is_file(), "Copilot project skills require regular files");
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    anyhow::ensure!(entry.metadata()?.nlink() == 1, "Copilot project skill file is hard linked");
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "copilot_skill_tests.rs"]
mod skill_tests;

// Persist only native history. Provider credentials remain in the leased profile.
pub fn preserve_history(profile: &AcpLaunchProfile) -> anyhow::Result<()> {
    #[cfg(not(unix))]
    anyhow::bail!("Copilot history ACL verification unavailable on this platform");
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
        let reference = profile
            .config()
            .env
            .get(super::super::profile::REFERENCE_ENV)
            .and_then(|value| value.strip_prefix("vw-session-model:"))
            .filter(|value| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| anyhow::anyhow!("Copilot requires a session-owned history reference"))?;
        let root = profile
            .directory()
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Copilot runtime parent missing"))?
            .join("shiju-acp-copilot-history");
        let history = root.join(reference);
        let owner = profile.directory().symlink_metadata()?.uid();
        for path in [&root, &history] {
            match std::fs::DirBuilder::new().mode(0o700).create(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => anyhow::bail!("cannot create private Copilot history directory"),
            }
            let metadata = path.symlink_metadata()?;
            anyhow::ensure!(
                metadata.is_dir()
                    && metadata.uid() == owner
                    && metadata.permissions().mode() & 0o777 == 0o700,
                "insecure Copilot history directory"
            );
        }
        std::os::unix::fs::symlink(history, profile.directory().join("session-state"))?;
        Ok(())
    }
}
