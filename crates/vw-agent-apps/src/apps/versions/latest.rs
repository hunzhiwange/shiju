#[derive(Debug, PartialEq, Eq)]
pub enum UnknownReason {
    Network,
    InvalidMetadata,
    LocalAhead,
    IncomparableLocal,
    Unsupported,
}
#[derive(Debug, PartialEq, Eq)]
pub struct LatestVersion {
    pub local_version: Option<String>,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub unknown_reason: Option<UnknownReason>,
}
/// Keep installed state independent from metadata availability.
pub fn select_latest(local: Option<&str>, remote: Result<String, UnknownReason>) -> LatestVersion {
    let mut result = LatestVersion {
        local_version: local.map(str::to_owned),
        latest_version: None,
        update_available: false,
        unknown_reason: None,
    };
    let candidate =
        remote.and_then(|v| valid_version(&v).map(|_| v).ok_or(UnknownReason::InvalidMetadata));
    match candidate {
        Err(reason) => result.unknown_reason = Some(reason),
        Ok(candidate) => {
            let remote = valid_version(&candidate).expect("validated remote");
            match local.map(valid_version) {
                Some(None) => result.unknown_reason = Some(UnknownReason::IncomparableLocal),
                Some(Some(local)) if local > remote => {
                    result.unknown_reason = Some(UnknownReason::LocalAhead)
                }
                parsed => {
                    result.update_available = parsed.flatten().is_some_and(|local| local < remote);
                    result.latest_version = Some(candidate);
                }
            }
        }
    }
    result
}

fn valid_version(raw: &str) -> Option<semver::Version> {
    let mut version = semver::Version::parse(raw.trim().trim_start_matches('v')).ok()?;
    let build = version.build.as_str().to_ascii_lowercase();
    let parts: Vec<_> = build.split('.').collect();
    let git = parts.len() > 1
        && parts[0].parse::<u64>().is_ok()
        && parts[1]
            .strip_prefix('g')
            .is_some_and(|s| s.len() >= 4 && s.bytes().all(|b| b.is_ascii_hexdigit()));
    if version.major >= 1000
        || git
        || parts.iter().any(|p| matches!(*p, "canary" | "nightly" | "dev" | "dirty" | "snapshot"))
    {
        return None;
    }
    // Build metadata is not part of precedence.
    version.build = semver::BuildMetadata::EMPTY;
    Some(version)
}

const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

#[derive(Clone, Copy)]
enum Source {
    Npm(bool),
    Github,
    Pypi,
    Agy,
}

fn metadata_version(
    source: Source,
    json: &serde_json::Value,
    local: Option<&str>,
) -> Result<String, UnknownReason> {
    let candidate = match source {
        Source::Npm(previews) => {
            let latest =
                json.get("latest").and_then(|v| v.as_str()).filter(|v| valid_version(v).is_some());
            let mut best = latest;
            if previews
                && local
                    .and_then(valid_version)
                    .zip(latest.and_then(valid_version))
                    .is_some_and(|(a, b)| a > b)
                && let Some(next) = json.get("next").and_then(|v| v.as_str())
                && valid_version(next).zip(best.and_then(valid_version)).is_some_and(|(a, b)| a > b)
            {
                best = Some(next);
            }
            best.map(str::to_owned)
        }
        Source::Github => {
            if json.get("draft").and_then(|v| v.as_bool()) == Some(true)
                || json.get("prerelease").and_then(|v| v.as_bool()) == Some(true)
            {
                return Err(UnknownReason::InvalidMetadata);
            }
            let name = json.get("name").and_then(|v| v.as_str()).and_then(|name| {
                name.split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | ','))
                    .find(|part| valid_version(part).is_some())
                    .map(str::to_owned)
            });
            name.or_else(|| {
                json.get("tag_name")
                    .and_then(|v| v.as_str())
                    .filter(|v| valid_version(v).is_some())
                    .map(|v| v.trim_start_matches('v').to_owned())
            })
        }
        Source::Pypi => json.pointer("/info/version").and_then(|v| v.as_str()).map(str::to_owned),
        Source::Agy => json.get("version").and_then(|v| v.as_str()).map(str::to_owned),
    };
    candidate.filter(|v| valid_version(v).is_some()).ok_or(UnknownReason::InvalidMetadata)
}

async fn fetch(
    client: &reqwest::Client,
    url: &str,
    source: Source,
    local: Option<&str>,
    timeout: std::time::Duration,
) -> Result<String, UnknownReason> {
    let response = client
        .get(url)
        .header("User-Agent", "shiju-agent-apps")
        .timeout(timeout)
        .send()
        .await
        .map_err(|_| UnknownReason::Network)?
        .error_for_status()
        .map_err(|_| UnknownReason::Network)?;
    let json = response.json().await.map_err(|_| UnknownReason::InvalidMetadata)?;
    metadata_version(source, &json, local)
}

/// Query fixed upstream metadata endpoints. No command execution or installation mutation.
/// The caller supplies the already-authorized host's local version; failure never erases it.
pub async fn query_latest(
    client: &reqwest::Client,
    app: vw_api_types::agent_apps::AppId,
    local: Option<&str>,
) -> LatestVersion {
    use vw_api_types::agent_apps::AppId::*;
    let package = match app {
        Claude => Some("@anthropic-ai/claude-code"),
        Codex => Some("@openai/codex"),
        Gemini => Some("@google/gemini-cli"),
        GrokBuild => Some("@xai-official/grok"),
        OpenCode => Some("opencode-ai"),
        OpenClaw => Some("openclaw"),
        Pi => Some("@earendil-works/pi-coding-agent"),
        Mcode => Some("@minimax-ai/code"),
        Kimi => Some("@moonshot-ai/kimi-code"),
        Dsh => Some("@deepseek-ai/dsh"),
        Omp => Some("@oh-my-pi/pi-coding-agent"),
        _ => None,
    };
    let remote = if let Some(package) = package {
        let url = format!(
            "https://registry.npmjs.org/-/package/{}/dist-tags",
            package.replace('/', "%2f")
        );
        let npm = fetch(client, &url, Source::Npm(app == Claude), local, TIMEOUT).await;
        if npm.is_err() && app == OpenCode {
            fetch(
                client,
                "https://api.github.com/repos/anomalyco/opencode/releases/latest",
                Source::Github,
                local,
                TIMEOUT,
            )
            .await
        } else {
            npm
        }
    } else {
        match app {
            Hermes => {
                let github = fetch(client, "https://api.github.com/repos/NousResearch/hermes-agent/releases/latest", Source::Github, local, TIMEOUT).await;
                if github.is_ok() { github } else {
                    fetch(client, "https://pypi.org/pypi/hermes-agent/json", Source::Pypi, local, TIMEOUT).await
                }
            }
            Agy => {
                match agy_platform(std::env::consts::OS, std::env::consts::ARCH) {
                    Some(platform) => fetch(client, &format!("https://antigravity-cli-auto-updater-974169037036.us-central1.run.app/manifests/{platform}.json"), Source::Agy, local, TIMEOUT).await,
                    None => Err(UnknownReason::Unsupported),
                }
            }
            _ => Err(UnknownReason::Unsupported),
        }
    };
    select_latest(local, remote)
}

fn agy_platform(os: &str, arch: &str) -> Option<String> {
    let os = match os {
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "windows",
        _ => return None,
    };
    let arch = match arch {
        "aarch64" => "arm64",
        "x86_64" => "amd64",
        _ => return None,
    };
    Some(format!("{os}_{arch}"))
}

#[cfg(test)]
#[path = "latest_tests.rs"]
mod http_tests;
