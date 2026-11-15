use vw_api_app_identity::{AppId, AppInstallation, AppInstallationSource, AppRuntimeKind};
// Observation-based display: an unreadable host is never labeled uninstalled.
use super::catalog::CatalogRow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presence {
    Installed,
    Missing,
    Unverified,
}

pub fn presence(app: &CatalogRow) -> Presence {
    let Ok(observation) = &app.observation else {
        return Presence::Unverified;
    };
    if !observation.installations.is_empty() {
        return Presence::Installed;
    }
    if app.entry.runtime_kinds.contains(&AppRuntimeKind::DesktopApp) {
        return match &observation.desktop {
            Some(desktop) if desktop.supported => match &desktop.app_bundles {
                Some(paths) if !paths.is_empty() => Presence::Installed,
                Some(_) => Presence::Missing,
                None => Presence::Unverified,
            },
            _ => Presence::Unverified,
        };
    }
    if app.entry.runtime_kinds.contains(&AppRuntimeKind::NativeCli) {
        Presence::Missing
    } else {
        Presence::Unverified
    }
}

pub fn matches_query(app: &CatalogRow, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    app.entry.name.to_lowercase().contains(&query)
        || app.entry.cli_command.as_ref().is_some_and(|c| c.to_lowercase().contains(&query))
}

pub fn preferred(app: &CatalogRow) -> Option<&AppInstallation> {
    let installations = &app.observation.as_ref().ok()?.installations;
    installations.iter().find(|i| i.is_path_default).or_else(|| installations.first())
}

pub fn source_label(source: AppInstallationSource) -> &'static str {
    match source {
        AppInstallationSource::Homebrew => "Homebrew",
        AppInstallationSource::Nvm => "nvm",
        AppInstallationSource::Volta => "Volta",
        AppInstallationSource::Fnm => "fnm",
        AppInstallationSource::Mise => "mise",
        AppInstallationSource::Bun => "Bun",
        AppInstallationSource::Pnpm => "pnpm",
        AppInstallationSource::Scoop => "Scoop",
        AppInstallationSource::Pip => "pip",
        AppInstallationSource::System => "系统安装",
        AppInstallationSource::Native => "官方安装",
        AppInstallationSource::Unknown => "来源待确认",
    }
}

pub fn key(id: AppId) -> String {
    serde_json::to_value(id)
        .expect("application ID serializes")
        .as_str()
        .expect("application ID is a string")
        .to_string()
}
