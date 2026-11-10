//! Native reader for the standard OMO variant.
use serde_json::{Map, Value};
use std::path::Path;

#[cfg(target_os = "macos")]
use crate::providers::{live::patch_json::object_value, paths, snippets, validate_identity};

#[derive(Debug, PartialEq, Eq)]
pub enum OmoReadError {
    UnsupportedTarget,
    Forbidden,
    InvalidConfiguration,
    TargetFailed,
}

#[derive(Debug, PartialEq, Eq)]
pub struct OmoRead {
    pub current_provider_id: Option<String>,
    pub agents: Option<Map<String, Value>>,
    pub categories: Option<Map<String, Value>>,
    pub other_fields: Option<Map<String, Value>>,
}

#[cfg(target_os = "macos")]
const LEGACY_NAMES: [&str; 4] = [
    "oh-my-openagent.jsonc",
    "oh-my-openagent.json",
    "oh-my-opencode.jsonc",
    "oh-my-opencode.json",
];

/// Read on the authorized execution host. The caller supplies the saved OpenCode
/// directory override and the current standard OMO provider from the same scope.
/// A unified file without an OpenCode section represents an unconfigured OMO.
#[cfg(target_os = "macos")]
pub fn read(
    home: &Path,
    directory_override: Option<&str>,
    current_provider_id: Option<&str>,
) -> Result<OmoRead, OmoReadError> {
    if !home.is_absolute() {
        return Err(OmoReadError::TargetFailed);
    }
    if let Some(id) = current_provider_id {
        validate_identity(id).map_err(|_| OmoReadError::InvalidConfiguration)?;
    }
    let directory = directory_override
        .map(|raw| paths::bounded(home, raw).map_err(|_| OmoReadError::Forbidden))
        .transpose()?
        .unwrap_or_else(|| home.join(".config/opencode"));
    let mut config = match read_first(home, &home.join(".omo"), &["omo.jsonc", "omo.json"])? {
        Some(mut root) => match root.remove("[opencode]") {
            None => Map::new(),
            Some(Value::Object(section)) => section,
            Some(_) => return Err(OmoReadError::InvalidConfiguration),
        },
        None => read_first(home, &directory, &LEGACY_NAMES)?.unwrap_or_default(),
    };
    let agents = take_object(&mut config, "agents")?;
    let categories = take_object(&mut config, "categories")?;
    remove_credentials(&mut config);
    Ok(OmoRead {
        current_provider_id: current_provider_id.map(str::to_owned),
        agents,
        categories,
        other_fields: (!config.is_empty()).then_some(config),
    })
}

#[cfg(not(target_os = "macos"))]
pub fn read(
    _home: &Path,
    _directory_override: Option<&str>,
    _current_provider_id: Option<&str>,
) -> Result<OmoRead, OmoReadError> {
    Err(OmoReadError::UnsupportedTarget)
}

#[cfg(target_os = "macos")]
pub(super) fn take_object(
    config: &mut Map<String, Value>,
    key: &str,
) -> Result<Option<Map<String, Value>>, OmoReadError> {
    match config.remove(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(mut value)) => {
            remove_credentials(&mut value);
            Ok(Some(value))
        }
        Some(_) => Err(OmoReadError::InvalidConfiguration),
    }
}

#[cfg(target_os = "macos")]
pub(super) fn remove_credentials(fields: &mut Map<String, Value>) {
    fn visit(value: &mut Value) {
        match value {
            Value::Object(fields) => remove_credentials(fields),
            Value::Array(items) => items.iter_mut().for_each(visit),
            _ => {}
        }
    }
    fields.retain(|key, _| !snippets::sensitive(key));
    fields.values_mut().for_each(visit);
}

#[cfg(target_os = "macos")]
fn read_first(
    home: &Path,
    directory: &Path,
    names: &[&str],
) -> Result<Option<Map<String, Value>>, OmoReadError> {
    for name in names {
        let Some(bytes) = read_candidate(home, &directory.join(name))? else { continue };
        let Value::Object(object) =
            object_value(&bytes).map_err(|_| OmoReadError::InvalidConfiguration)?
        else {
            return Err(OmoReadError::InvalidConfiguration);
        };
        return Ok(Some(object));
    }
    Ok(None)
}

#[cfg(target_os = "macos")]
pub(super) fn read_candidate(home: &Path, path: &Path) -> Result<Option<Vec<u8>>, OmoReadError> {
    use std::io::Read;
    use vw_api_types::app_providers::paths::ConfigPathState;

    const MAX_BYTES: u64 = 1024 * 1024;
    let file = match paths::open_beneath(home, path, false) {
        Ok(file) => file,
        Err(ConfigPathState::Absent) => return Ok(None),
        Err(_) => return Err(OmoReadError::TargetFailed),
    };
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes).map_err(|_| OmoReadError::TargetFailed)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(OmoReadError::InvalidConfiguration);
    }
    Ok(Some(bytes))
}
