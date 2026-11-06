//! Pure execution-host preference validation; persistence belongs to the caller.
use super::{
    catalog::catalog,
    discovery::wsl::{WslShell, parse_target},
};
use std::io;
use vw_api_types::agent_apps::{AppHostPreferences, AppPreferencesResult, AppUserPreferences};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostPlatform {
    MacOs,
    Windows,
    Linux,
}

pub fn default_user_preferences() -> AppUserPreferences {
    AppUserPreferences {
        app_order: Vec::new(),
        visible_apps: catalog().into_iter().map(|app| app.metadata.id).collect(),
    }
}

/// Filters stale/duplicate order entries and inserts missing IDs after their
/// nearest canonical predecessor. An explicitly empty visibility set is rejected.
/// Returns an advisory for changed host launch settings; never spawns a process.
pub fn normalize_preferences(
    mut user: AppUserPreferences,
    host: AppHostPreferences,
    previous_host: &AppHostPreferences,
    platform: HostPlatform,
) -> io::Result<AppPreferencesResult> {
    let canonical = catalog()
        .into_iter()
        .map(|app| {
            // AppId is a serde string enum; use its wire identity, not CLI names.
            serde_json::from_value::<String>(
                serde_json::to_value(app.metadata.id).expect("AppId serializes"),
            )
            .expect("AppId is a string")
        })
        .collect::<Vec<_>>();
    let mut order = Vec::new();
    for id in user.app_order {
        if canonical.contains(&id) && !order.contains(&id) {
            order.push(id);
        }
    }
    for (index, id) in canonical.iter().enumerate() {
        if order.contains(id) {
            continue;
        }
        let at = canonical[..index]
            .iter()
            .rev()
            .find_map(|previous| order.iter().position(|item| item == previous))
            .map_or(0, |at| at + 1);
        order.insert(at, id.clone());
    }
    user.app_order = order;
    let mut visible = Vec::new();
    for id in user.visible_apps {
        if !visible.contains(&id) {
            visible.push(id);
        }
    }
    if visible.is_empty() {
        return Err(invalid("at least one application must remain visible"));
    }
    user.visible_apps = visible;
    let host = normalize_host(host, platform)?;
    let previous = normalize_host(previous_host.clone(), platform)?;
    let restart_required = host != previous;
    Ok(AppPreferencesResult {
        user,
        host,
        restart_required,
        capabilities: Some(vw_api_types::agent_apps::AppPreferencesCapabilities {
            terminals: terminals(platform).iter().map(|s| (*s).into()).collect(),
            wsl_preferences: platform == HostPlatform::Windows,
        }),
    })
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn normalize_host(
    mut host: AppHostPreferences,
    platform: HostPlatform,
) -> io::Result<AppHostPreferences> {
    let terminals = terminals(platform);
    if host.preferred_terminal.as_deref().is_some_and(|value| !terminals.contains(&value)) {
        return Err(invalid("terminal is not supported on the execution host platform"));
    }
    if platform != HostPlatform::Windows && !host.wsl_shells.is_empty() {
        return Err(invalid("WSL preferences require a Windows execution host"));
    }
    let mut seen = Vec::new();
    for preference in &mut host.wsl_shells {
        if preference.app_id == vw_api_types::agent_apps::AppId::ClaudeDesktop
            || seen.contains(&preference.app_id)
        {
            return Err(invalid("WSL preferences require unique CLI application IDs"));
        }
        seen.push(preference.app_id);
        let shell = preference.shell.as_deref().filter(|value| *value != "auto");
        let flag = preference.flag.as_deref().filter(|value| *value != "auto");
        // Reuse the discovery parser's shell allowlist/default flag rules. This
        // literal is parsing context only, never a target or execution request.
        let target = parse_target(r"\\wsl$\PreferenceValidation", shell, flag)?;
        match target.shell {
            WslShell::Auto { flag } => {
                preference.shell = None;
                preference.flag = flag;
            }
            WslShell::Explicit { shell, flag } => {
                preference.shell = Some(shell);
                preference.flag = Some(flag);
            }
        }
    }
    // Canonical ordering avoids a false advisory when only map entry order changed.
    let ids = default_user_preferences().visible_apps;
    host.wsl_shells.sort_by_key(|entry| ids.iter().position(|id| *id == entry.app_id));
    Ok(host)
}

fn terminals(platform: HostPlatform) -> &'static [&'static str] {
    match platform {
        HostPlatform::MacOs => &[
            "terminal",
            "iterm2",
            "warp",
            "alacritty",
            "kitty",
            "ghostty",
            "otty",
            "wezterm",
            "kaku",
        ],
        HostPlatform::Windows => &["cmd", "powershell", "wt"],
        HostPlatform::Linux => {
            &["gnome-terminal", "konsole", "xfce4-terminal", "alacritty", "kitty", "ghostty"]
        }
    }
}
