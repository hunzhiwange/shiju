//! Create-only inputs for copying application prompts. No activation or file I/O.
use vw_api_types::agent_apps::{AppId, PromptResource};

/// A new identity is issued on the server for every requested target.
/// Name collisions are allowed: callers must insert, never upsert by name.
pub fn new_copy_id(target: AppId, content: &str) -> Result<String, &'static str> {
    PromptResource::Application { app_id: target }
        .validate_application_access()
        .map_err(|_| "unsupported prompt copy target")?;
    if target == AppId::Mcode && super::mcode::validate_content(content).is_err() {
        return Err("MCode prompt exceeds 32768 UTF-8 bytes");
    }
    Ok(uuid::Uuid::new_v4().to_string())
}
