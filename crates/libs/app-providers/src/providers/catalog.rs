//! Read-only observations of tenant records and explicitly authorized host configuration.
use std::{collections::BTreeMap, path::Path};
use vw_api_app_provider_base::AppProviderSummary;
use vw_api_app_provider_edit::catalog::*;
use vw_api_app_identity::{AppId};

/// `home` is the administrator-authorized execution home, never a client path.
/// Native reads cover Kimi TOML, OpenCode/OpenClaw JSON/JSONC, Hermes YAML, Pi models JSONC, OMP models YAML and Mcode custom-provider YAML.
/// Hermes also honors an execution-host HERMES_HOME beneath this authorized HOME.
/// No CC database, ambient client home, separate credential files or processes are read.
/// Configurations can contain secrets; only the typed metadata projection leaves this owner.
pub fn read(home: &Path, app: AppId, rows: Vec<AppProviderSummary>) -> ProviderCatalog {
    let unavailable = |reason| ProviderSelection::Unavailable { reason };
    // Kimi is exceptional: its native default_model provides a selected member.
    let additive = matches!(
        app,
        AppId::OpenCode
            | AppId::OpenClaw
            | AppId::Hermes
            | AppId::Pi
            | AppId::Mcode
            | AppId::Dsh
            | AppId::Omp
            | AppId::Agy
    );
    let mut result = ProviderCatalog {
        app_id: app,
        rows: rows
            .into_iter()
            .filter(|p| p.app_id == app)
            .map(|provider| ProviderCatalogRow {
                provider,
                native_member: ProviderMembership::Unknown,
            })
            .collect(),
        direct: if additive {
            ProviderSelection::NotApplicable
        } else {
            unavailable(ProviderReadUnavailable::NotImplemented)
        },
        in_use: if additive {
            ProviderSelection::NotApplicable
        } else {
            unavailable(ProviderReadUnavailable::NotImplemented)
        },
        native_read: None,
    };
    if app == AppId::Agy {
        return result;
    }
    match observe(home, app) {
        Ok((members, selection)) => {
            let mut rows: BTreeMap<_, _> = result
                .rows
                .into_iter()
                .map(|mut row| {
                    row.native_member = if members.contains_key(&row.provider.id) {
                        ProviderMembership::Present
                    } else {
                        ProviderMembership::Absent
                    };
                    (row.provider.id.clone(), row)
                })
                .collect();
            for id in members.into_keys() {
                rows.entry(id.clone()).or_insert_with(|| ProviderCatalogRow {
                    provider: AppProviderSummary {
                        id: id.clone(),
                        app_id: app,
                        name: id,
                        configured: true,
                    },
                    native_member: ProviderMembership::Present,
                });
            }
            result.rows = rows.into_values().collect();
            result.in_use = selection;
        }
        Err(reason) => {
            result.native_read = Some(reason);
            if !additive {
                result.in_use = unavailable(reason);
            }
        }
    }
    result
}

type Members = BTreeMap<String, ()>;
pub(super) fn observe(
    home: &Path,
    app: AppId,
) -> Result<(Members, ProviderSelection), ProviderReadUnavailable> {
    use ProviderReadUnavailable::*;
    if !cfg!(target_os = "macos") {
        return Err(UnsupportedPlatform);
    }
    #[cfg(target_os = "macos")]
    if app == AppId::OpenCode {
        let mut bytes = None;
        for name in ["opencode.jsonc", "opencode.json"] {
            bytes = read_file(home, &[".config", "opencode", name])?;
            if bytes.is_some() {
                break;
            }
        }
        let rows =
            super::opencode::declarations(bytes.as_deref()).map_err(|_| InvalidConfiguration)?;
        return Ok((
            rows.into_keys().map(|id| (id, ())).collect(),
            ProviderSelection::NotApplicable,
        ));
    }
    #[cfg(target_os = "macos")]
    if app == AppId::OpenClaw {
        let bytes = super::openclaw::read(home).map_err(|_| InvalidConfiguration)?;
        let rows =
            super::openclaw::declarations(bytes.as_deref()).map_err(|_| InvalidConfiguration)?;
        return Ok((
            rows.into_keys().map(|id| (id, ())).collect(),
            ProviderSelection::NotApplicable,
        ));
    }
    #[cfg(target_os = "macos")]
    if app == AppId::Hermes {
        let bytes = super::hermes::read(home).map_err(|_| InvalidConfiguration)?;
        let rows =
            super::hermes::declarations(bytes.as_deref()).map_err(|_| InvalidConfiguration)?;
        return Ok((
            rows.into_keys().map(|id| (id, ())).collect(),
            ProviderSelection::NotApplicable,
        ));
    }
    #[cfg(target_os = "macos")]
    if app == AppId::Pi {
        let bytes = super::pi::read(home).map_err(|_| InvalidConfiguration)?;
        let rows = super::pi::declarations(bytes.as_deref()).map_err(|_| InvalidConfiguration)?;
        return Ok((
            rows.into_keys().map(|id| (id, ())).collect(),
            ProviderSelection::NotApplicable,
        ));
    }
    #[cfg(target_os = "macos")]
    if app == AppId::Omp {
        let bytes = super::omp::read(home).map_err(|_| InvalidConfiguration)?;
        let rows = super::omp::declarations(bytes.as_deref()).map_err(|_| InvalidConfiguration)?;
        return Ok((
            rows.into_keys().map(|id| (id, ())).collect(),
            ProviderSelection::NotApplicable,
        ));
    }
    #[cfg(target_os = "macos")]
    if app == AppId::Mcode {
        let bytes = super::mcode::read(home).map_err(|_| InvalidConfiguration)?;
        let rows =
            super::mcode::declarations(bytes.as_deref()).map_err(|_| InvalidConfiguration)?;
        return Ok((
            rows.into_keys().map(|id| (id, ())).collect(),
            ProviderSelection::NotApplicable,
        ));
    }
    #[cfg(target_os = "macos")]
    if app == AppId::Dsh {
        let snapshot = super::dsh::read(home).map_err(|_| InvalidConfiguration)?;
        let state = super::dsh::state(&snapshot).map_err(|_| InvalidConfiguration)?;
        return Ok((
            state.members.into_iter().map(|id| (id, ())).collect(),
            ProviderSelection::NotApplicable,
        ));
    }
    if app != AppId::Kimi {
        return Err(NotImplemented);
    }
    #[cfg(target_os = "macos")]
    {
        if std::env::var_os("KIMI_CODE_HOME").is_some_and(|v| !v.is_empty()) {
            return Err(CustomPathNotSupported);
        }
        let bytes = super::kimi::read(home).map_err(|_| Unreadable)?;
        let rows = super::kimi::declarations(bytes.as_deref()).map_err(|_| InvalidConfiguration)?;
        let selection =
            super::kimi::selection(bytes.as_deref()).map_err(|_| InvalidConfiguration)?;
        Ok((rows.into_keys().map(|id| (id, ())).collect(), selection))
    }
    #[cfg(not(target_os = "macos"))]
    Err(UnsupportedPlatform)
}

pub(super) use app_auth::providers::catalog::read_file;
