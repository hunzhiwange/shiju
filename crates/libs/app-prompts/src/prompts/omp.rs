//! OMP instructions use the active profile's configuration directory.
use std::{collections::BTreeMap, path::Path};

use vw_api_app_identity::{AppId};

use super::targets::{PromptOwnership, PromptTarget};
use app_config_paths as paths;

/// Resolve the sole source-proven OMP prompt resource, AGENTS.md.
///
/// Reuse the provider directory contract so prompts and models cannot select
/// different profiles. Host overrides and environment are authenticated inputs;
/// the resulting path is an observation, never permission to read or write it.
/// Pi native system files and templates are unsupported for OMP.
pub fn resolve(
    home: &Path,
    override_dir: Option<&str>,
    environment: &BTreeMap<String, String>,
) -> Result<PromptTarget, &'static str> {
    let resolved = paths::read(home, AppId::Omp, override_dir, environment)?;
    // AGENTS.md belongs to the configuration directory, even when XDG moves
    // the profile's data directory elsewhere.
    let file = Path::new(&resolved.directory).join("AGENTS.md");
    let file = paths::bounded(home, file.to_str().ok_or("invalid prompt target path")?)?;
    Ok(PromptTarget {
        app_id: AppId::Omp,
        file,
        ownership: PromptOwnership::PromptLibrary,
        pi_native_resources: false,
    })
}
