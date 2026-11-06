//! Native reference guard for catalog deletion; never removes native members.
use std::path::Path;
use vw_api_types::{agent_apps::AppId, app_providers::catalog::ProviderSelection};

pub fn ensure_unreferenced(home: &Path, app: AppId, id: &str) -> Result<(), String> {
    super::validate_identity(id)?;
    // These two native adapters have no managed direct pointer or proxy-route writer:
    // the compiled owner only persists catalog records (upstream is not built).
    // Do not infer absence from catalog's NotImplemented projection. When managed
    // state writers are introduced, their real references must join this guard.
    // Other applications lack a complete native observation and must fail closed.
    match app {
        AppId::Kimi | AppId::OpenCode => {}
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
