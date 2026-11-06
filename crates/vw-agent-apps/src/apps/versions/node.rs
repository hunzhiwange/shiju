//! Node upgrade checks for an installation resolved by the authorized execution host.
use std::time::Duration;
use vw_api_types::agent_apps::{
    AppId, AppInstallation, AppInstallationEnvironment as Environment,
    AppInstallationSource as Source,
};

#[derive(Debug, PartialEq, Eq)]
pub enum NodeRequirement {
    NotApplicable,
    Unknown,
    Satisfied { required: String, current: String },
    Gap { required: String, current: String },
}

/// Select beside this instance, then the host-resolved PATH fallback. The callback
/// must inspect files in the same authorized host/environment, including Windows.
/// A returned path is a probe candidate, never an execution authorization.
pub fn node_candidate(
    installation: &AppInstallation,
    fallback: Option<&str>,
    is_file: impl Fn(&str) -> bool,
) -> Option<String> {
    let windows = matches!(installation.environment, Environment::Windows);
    let path = installation.path.replace('\\', "/");
    let (dir, _) = path.rsplit_once('/')?;
    let sibling = format!("{dir}/{}", if windows { "node.exe" } else { "node" });
    if is_file(&sibling) { Some(sibling) } else { fallback.map(str::to_owned) }
}

fn applicable(installation: &AppInstallation) -> bool {
    !matches!(installation.app_id, AppId::Mcode | AppId::ClaudeDesktop | AppId::Hermes | AppId::Agy)
        && !matches!(installation.environment, Environment::Wsl { .. })
        && !matches!(installation.source, Source::Bun | Source::Homebrew | Source::Native)
        && installation
            .real_path
            .as_deref()
            .unwrap_or(&installation.path)
            .replace('\\', "/")
            .contains("/node_modules/")
}

/// Always probe afresh; callers must pass the selected instance again immediately
/// before mutation, under the lifecycle write lock. Unknown remains distinguishable
/// from satisfied and is allowed by the upstream advisory gate.
pub async fn update_node_gap(
    client: &reqwest::Client,
    installation: &AppInstallation,
    target: &str,
    probe_node: impl FnOnce(&AppInstallation) -> Option<String>,
) -> NodeRequirement {
    update_node_gap_at(client, installation, target, probe_node, "https://registry.npmjs.org/")
        .await
}

/// Internal metadata boundary, also used by isolated HTTP fixtures.
pub(crate) async fn update_node_gap_at(
    client: &reqwest::Client,
    installation: &AppInstallation,
    target: &str,
    probe_node: impl FnOnce(&AppInstallation) -> Option<String>,
    registry: &str,
) -> NodeRequirement {
    if !applicable(installation) {
        return NodeRequirement::NotApplicable;
    }
    let Some(current) = probe_node(installation) else {
        return NodeRequirement::Unknown;
    };
    let package = match installation.app_id {
        AppId::Claude => "@anthropic-ai/claude-code",
        AppId::Codex => "@openai/codex",
        AppId::Gemini => "@google/gemini-cli",
        AppId::GrokBuild => "@xai-official/grok",
        AppId::OpenCode => "opencode-ai",
        AppId::OpenClaw => "openclaw",
        AppId::Pi => "@earendil-works/pi-coding-agent",
        AppId::Kimi => "@moonshot-ai/kimi-code",
        AppId::Dsh => "@deepseek-ai/dsh",
        AppId::Omp => "@oh-my-pi/pi-coding-agent",
        _ => return NodeRequirement::NotApplicable,
    };
    let mut url = reqwest::Url::parse(registry).expect("internal registry URL");
    url.path_segments_mut().expect("base URL").push(package).push(target);
    fetch_requirement(client, url.as_str(), &current).await
}

async fn fetch_requirement(client: &reqwest::Client, url: &str, current: &str) -> NodeRequirement {
    let Ok(response) = client.get(url).timeout(Duration::from_secs(15)).send().await else {
        return NodeRequirement::Unknown;
    };
    let Ok(response) = response.error_for_status() else {
        return NodeRequirement::Unknown;
    };
    let Ok(json) = response.json::<serde_json::Value>().await else {
        return NodeRequirement::Unknown;
    };
    evaluate_requirement(&json, current)
}

fn evaluate_requirement(json: &serde_json::Value, current: &str) -> NodeRequirement {
    let Some(required) = json
        .pointer("/engines/node")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
    else {
        return NodeRequirement::Unknown;
    };
    match node_range_satisfied(required, current) {
        Some(true) => {
            NodeRequirement::Satisfied { required: required.into(), current: current.into() }
        }
        Some(false) => NodeRequirement::Gap { required: required.into(), current: current.into() },
        None => NodeRequirement::Unknown,
    }
}

/// Fresh execution-time gate. No cached preview outcome is accepted.
pub async fn ensure_update_node(
    client: &reqwest::Client,
    installation: &AppInstallation,
    target: &str,
    probe_node: impl FnOnce(&AppInstallation) -> Option<String>,
) -> Result<NodeRequirement, NodeRequirement> {
    let result = update_node_gap(client, installation, target, probe_node).await;
    enforce(result)
}

fn enforce(result: NodeRequirement) -> Result<NodeRequirement, NodeRequirement> {
    if matches!(result, NodeRequirement::Gap { .. }) { Err(result) } else { Ok(result) }
}

type Core = [u64; 3];

#[derive(Debug, Clone, Copy)]
enum Bound {
    Ge(Core),
    Gt(Core),
    Le(Core),
    Lt(Core),
}

impl Bound {
    fn admits(self, v: Core) -> bool {
        match self {
            Bound::Ge(b) => v >= b,
            Bound::Gt(b) => v > b,
            Bound::Le(b) => v <= b,
            Bound::Lt(b) => v < b,
        }
    }
}

/// `version` 是否满足 npm 版本范围 `range`。支持比较符、`^`、`~`、x 通配、`a - b` 与 `||`；
/// 认不出的写法返回 None，调用方按「不确定」放行。
pub fn node_range_satisfied(range: &str, version: &str) -> Option<bool> {
    let parsed = semver::Version::parse(version.trim().trim_start_matches('v')).ok()?;
    let version = [parsed.major, parsed.minor, parsed.patch];
    let mut unknown = false;
    for set in range.split("||") {
        match comparator_set_admits(set, version) {
            Some(true) => return Some(true),
            Some(false) => {}
            None => unknown = true,
        }
    }
    (!unknown).then_some(false)
}

fn comparator_set_admits(set: &str, version: Core) -> Option<bool> {
    let tokens: Vec<&str> = set.split_whitespace().collect();
    let mut bounds = Vec::new();
    if let [low, "-", high] = tokens.as_slice() {
        bounds.extend(comparator(">=", &parse_partial(low)?)?);
        bounds.extend(comparator("<=", &parse_partial(high)?)?);
    } else {
        // `>= 24` 这种比较符与版本之间带空格的写法也合法。
        let mut pending: Option<&str> = None;
        for token in tokens {
            let (op, rest) = split_operator(token);
            let op = match pending.take() {
                Some(_) if !op.is_empty() => return None,
                Some(pending) => pending,
                None => op,
            };
            if rest.is_empty() {
                pending = Some(op);
                continue;
            }
            bounds.extend(comparator(op, &parse_partial(rest)?)?);
        }
        if pending.is_some() {
            return None;
        }
    }
    Some(bounds.into_iter().all(|bound| bound.admits(version)))
}

fn split_operator(token: &str) -> (&str, &str) {
    for op in [">=", "<=", ">", "<", "=", "^", "~"] {
        if let Some(rest) = token.strip_prefix(op) {
            return (op, rest);
        }
    }
    ("", token)
}

/// `24` / `24.16` / `24.16.0` / `24.x` / `*` → 写出来的那几段；`v` 前缀与预发布段忽略。
fn parse_partial(raw: &str) -> Option<Vec<u64>> {
    let core = raw.trim_start_matches('v').split(['-', '+']).next()?;
    let mut parts = Vec::new();
    for segment in core.split('.') {
        if matches!(segment, "x" | "X" | "*") {
            break;
        }
        parts.push(segment.parse().ok()?);
    }
    (parts.len() <= 3).then_some(parts)
}

fn comparator(op: &str, parts: &[u64]) -> Option<Vec<Bound>> {
    let at = |i: usize| parts.get(i).copied().unwrap_or(0);
    let floor = [at(0), at(1), at(2)];
    // 最后写出的那段 +1、其后归零：`24` → 25.0.0，`24.16` → 24.17.0。
    let next = || {
        let mut core = floor;
        core[parts.len() - 1] = core[parts.len() - 1].checked_add(1)?;
        for slot in core.iter_mut().skip(parts.len()) {
            *slot = 0;
        }
        Some(core)
    };
    let exact = parts.len() == 3;
    if parts.is_empty() {
        return matches!(op, "" | "=" | ">=" | "<=" | "^" | "~").then(Vec::new);
    }
    Some(match op {
        "" | "=" if exact => vec![Bound::Ge(floor), Bound::Le(floor)],
        "" | "=" => vec![Bound::Ge(floor), Bound::Lt(next()?)],
        ">=" => vec![Bound::Ge(floor)],
        ">" if exact => vec![Bound::Gt(floor)],
        ">" => vec![Bound::Ge(next()?)],
        "<" => vec![Bound::Lt(floor)],
        "<=" if exact => vec![Bound::Le(floor)],
        "<=" => vec![Bound::Lt(next()?)],
        "~" if parts.len() == 1 => vec![Bound::Ge(floor), Bound::Lt([at(0).checked_add(1)?, 0, 0])],
        "~" => vec![Bound::Ge(floor), Bound::Lt([at(0), at(1).checked_add(1)?, 0])],
        "^" => {
            let upper = if at(0) > 0 || parts.len() == 1 {
                [at(0).checked_add(1)?, 0, 0]
            } else if at(1) > 0 || parts.len() == 2 {
                [0, at(1).checked_add(1)?, 0]
            } else {
                [0, 0, at(2).checked_add(1)?]
            };
            vec![Bound::Ge(floor), Bound::Lt(upper)]
        }
        _ => return None,
    })
}

#[cfg(test)]
#[path = "node_tests.rs"]
mod tests;
