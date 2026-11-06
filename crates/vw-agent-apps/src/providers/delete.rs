//! Native reference guard for catalog deletion; never removes native members.
use std::path::Path;
use vw_api_types::{agent_apps::AppId, app_providers::catalog::ProviderSelection};

pub fn ensure_unreferenced(home: &Path, app: AppId, id: &str) -> Result<(), String> {
    super::validate_identity(id)?;
    // Additive OpenCode/OpenClaw writes never create a unique direct pointer. Native
    // membership still blocks catalog deletion; removing live is a separate action.
    // Other applications without a complete observation remain fail-closed.
    match app {
        AppId::Kimi | AppId::OpenCode | AppId::OpenClaw => {}
        _ => return Err("application provider references unavailable".into()),
    }
    let (members, selection) = super::catalog::observe(home, app)
        .map_err(|_| "application provider references unavailable")?;
    if members.contains_key(id)
        || matches!(selection, ProviderSelection::Selected { id: selected } if selected == id)
    {
        return Err("application provider is referenced".into());
    }
    Ok(())
}
