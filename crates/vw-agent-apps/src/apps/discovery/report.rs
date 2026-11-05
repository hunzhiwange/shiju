//! Read-only aggregation of observations collected by the authorized execution host.
use vw_api_types::agent_apps::{
    AppInstallation, AppInstallationEnvironment, AppInstallationReport,
    AppInstallationRuntimeStatus, AppInstallationSource,
};

/// Build a single application's report in one host/environment scope.
/// Real paths must already be resolved by that execution host. This function
/// neither consults the local filesystem nor grants authority to these DTOs.
pub fn build(installations: Vec<AppInstallation>) -> Result<AppInstallationReport, &'static str> {
    let mut result: Vec<AppInstallation> = Vec::new();
    for mut installation in installations {
        if let Some(first) = result.first()
            && (first.app_id != installation.app_id
                || first.target_host_id != installation.target_host_id
                || first.environment != installation.environment)
        {
            return Err("installation report requires one application, host and environment");
        }
        installation.path = normalize(&installation.path, &installation.environment)?;
        installation.real_path = installation
            .real_path
            .as_ref()
            .map(|path| normalize(path, &installation.environment))
            .transpose()?;
        let identity = installation.real_path.as_ref().unwrap_or(&installation.path);
        if let Some(existing) = result
            .iter_mut()
            .find(|existing| existing.real_path.as_ref().unwrap_or(&existing.path) == identity)
        {
            // Preserve the actual PATH entry and its identity when aliases converge.
            if installation.is_path_default && !existing.is_path_default {
                *existing = installation;
            }
        } else {
            result.push(installation);
        }
    }
    for installation in &mut result {
        if installation.source == AppInstallationSource::Unknown {
            installation.source = infer_source(installation);
        }
    }
    result.sort_by_key(|installation| !installation.is_path_default);
    Ok(AppInstallationReport { installations: result })
}

fn normalize(path: &str, environment: &AppInstallationEnvironment) -> Result<String, &'static str> {
    let windows = matches!(environment, AppInstallationEnvironment::Windows);
    let path = if windows { path.replace('\\', "/").to_ascii_lowercase() } else { path.to_owned() };
    let bytes = path.as_bytes();
    let absolute = if windows {
        path.starts_with("//")
            || (bytes.len() > 3
                && bytes[0].is_ascii_alphabetic()
                && bytes[1] == b':'
                && bytes[2] == b'/')
    } else {
        path.starts_with('/')
    };
    if !absolute || path.contains('\0') || path.split('/').any(|part| part == "..") {
        return Err("absolute resolved host installation path required");
    }
    // Preserve leading // for UNC paths; dots and duplicate separators are aliases.
    let prefix = if path.starts_with("//") && windows {
        "//"
    } else if path.starts_with('/') {
        "/"
    } else {
        ""
    };
    Ok(format!(
        "{prefix}{}",
        path.split('/')
            .filter(|part| !part.is_empty() && *part != ".")
            .collect::<Vec<_>>()
            .join("/")
    ))
}

fn infer_source(installation: &AppInstallation) -> AppInstallationSource {
    use AppInstallationSource::*;
    let real = installation.real_path.as_deref().unwrap_or(&installation.path).to_ascii_lowercase();
    if real.contains("/cellar/") || real.contains("/caskroom/") {
        return Homebrew;
    }
    let path = installation.path.to_ascii_lowercase();
    for (fragment, source) in [
        ("/.nvm/", Nvm),
        ("/homebrew/", Homebrew),
        ("/cellar/", Homebrew),
        ("/caskroom/", Homebrew),
        ("/.volta/", Volta),
        ("/volta/", Volta),
        ("fnm_multishells", Fnm),
        ("/mise/", Mise),
        ("/.bun/", Bun),
        ("/pnpm/", Pnpm),
        ("/scoop/", Scoop),
        ("/library/python", Pip),
        ("/scripts/", Pip),
        ("/site-packages/", Pip),
    ] {
        if path.contains(fragment) {
            return source;
        }
    }
    // Unrecognized paths remain unknown; never imply a mutation recipe.
    Unknown
}

/// Confirmation is required even when all instances run the same version.
pub fn requires_upgrade_confirmation(report: &AppInstallationReport) -> bool {
    report.installations.len() > 1
}

/// Mirrors upstream thresholds, including known/unknown version divergence.
pub fn is_conflicting(report: &AppInstallationReport) -> bool {
    let Some(first) = report.installations.first() else {
        return false;
    };
    let runnable = |installation: &AppInstallation| {
        matches!(installation.runtime_status, AppInstallationRuntimeStatus::Runnable)
    };
    report.installations.iter().skip(1).any(|installation| {
        installation.version != first.version || runnable(installation) != runnable(first)
    })
}
