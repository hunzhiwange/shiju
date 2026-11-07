//! Agy selects a persistent terminal profile; it has no global current provider.
use super::live::{commit::*, patch_json::JsonPatch, stage::FileChange, write_guard::*};
use crate::apps::launch::{provider::ProviderLaunch, request::LaunchMaterial};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
use vw_api_types::{agent_apps::AppId, app_providers::agy::AgyProfile, id::TenantScope};
#[path = "agy_files.rs"]
mod files;
pub(crate) use files::Lease;
pub use files::catalog_lease;

/// Trusted execution-host context, never deserialized from a request.
pub struct Scope<'a> {
    pub home: &'a Path,
    pub private_root: &'a Path,
    pub host: &'a str,
    pub tenant: &'a TenantScope,
    pub provider: &'a str,
}
impl Scope<'_> {
    pub fn directory(&self) -> PathBuf {
        super::paths::agy_profile_directory(
            self.private_root,
            self.host,
            self.tenant,
            self.provider,
        )
    }
}
pub fn validate(p: &AgyProfile) -> Result<(), String> {
    let invalid = || "invalid Agy profile".to_string();
    if p.name.trim().is_empty() || p.name.len() > 128 || p.name.chars().any(char::is_control) {
        return Err(invalid());
    }
    for (s, max) in p
        .model
        .iter()
        .map(|s| (s, 256))
        .chain(p.api_key.iter().map(|s| (s, 8192)))
        .chain(p.base_url.iter().map(|s| (s, 2048)))
    {
        if s.trim().is_empty() || s.len() > max || s.chars().any(char::is_control) {
            return Err(invalid());
        }
    }
    if p.model.as_ref().is_some_and(|v| v.starts_with('-'))
        || (p.base_url.is_some() && p.api_key.is_none())
    {
        return Err(invalid());
    }
    if p.api_key.as_ref().is_some_and(|s| {
        matches!(s.as_str(), "[redacted]" | "<redacted>") || s.chars().all(|c| c == '*' || c == '•')
    }) {
        return Err(invalid());
    }
    if let Some(url) = &p.base_url {
        super::validation::endpoint(url)?;
    }
    Ok(())
}
pub fn prepare(
    scope: &Scope<'_>,
    p: &AgyProfile,
    m: LaunchMaterial,
) -> Result<ProviderLaunch, String> {
    prepare_checked(scope, p, m, || true)
}
/// Recheck the selected saved row before publication; no client paths or arbitrary link operations.
pub fn prepare_checked(
    scope: &Scope<'_>,
    p: &AgyProfile,
    m: LaunchMaterial,
    unchanged: impl Fn() -> bool,
) -> Result<ProviderLaunch, String> {
    validate(p)?;
    let catalog = Some(catalog_lease(scope, false).map_err(safe)?);
    super::validate_identity(scope.provider)?;
    super::validate_identity(scope.host)?;
    if scope.home.to_str().is_none()
        || scope.private_root.to_str().is_none()
        || !scope.home.is_absolute()
        || !scope.private_root.is_absolute()
        || !m.program.is_absolute()
        || !m.program.is_file()
        || !m.cwd.is_absolute()
        || !m.cwd.is_dir()
        || m.skip_permissions
        || m.args.len() > 32
        || m.args
            .iter()
            .any(|s| s.starts_with('-') || s.len() > 8192 || s.chars().any(char::is_control))
    {
        return Err("invalid Agy launch context".into());
    }
    let mut args = Vec::new();
    let mut env = vec![("HOME", scope.home.to_string_lossy().into_owned())];
    let lease = if let Some(key) = &p.api_key {
        let lease = Lease::open(scope).map_err(safe)?;
        let mut guard = lease.guard().map_err(safe)?;
        let tx = ProviderTransactions::new(
            &scope.directory(),
            ExecutionHostId::new(scope.host).map_err(safe)?,
        )
        .map_err(safe)?;
        lease.ensure_link().map_err(safe)?;
        if tx.recover(&mut guard).map_err(safe)?.is_some() {
            return Err("Agy profile recovered; retry launch".into());
        }
        let pre = guard.read(OsStr::new("settings.json")).map_err(safe)?;
        let mut patch = JsonPatch::default();
        patch.set.push((vec!["modelProvider".into()], serde_json::json!("gemini")));
        match &p.model {
            Some(model) => patch.set.push((vec!["model".into()], serde_json::json!(model))),
            None => patch.remove.push(vec!["model".into()]),
        }
        // Link provisioning is create-only; the transaction owns settings publication and recovery.
        patch.apply(pre.as_deref()).map_err(|_| "invalid Agy settings")?;
        lease.ensure_link().map_err(safe)?;
        let apply =
            |before: Option<&[u8]>| patch.apply(before).map_err(|_| WriteError::InvalidResource);
        tx.commit_checked(
            scope.tenant,
            &mut guard,
            &[FileChange {
                name: OsStr::new("settings.json"),
                privacy: FilePrivacy::Private,
                patch: &apply,
            }],
            PendingTarget::default(),
            || {
                lease.validate()?;
                if !unchanged() {
                    return Err(WriteError::Conflict);
                }
                Ok(())
            },
        )
        .map_err(safe)?;
        args.push(format!(
            "--gemini_dir={}",
            scope.directory().to_str().ok_or("invalid Agy directory")?
        ));
        env.push(("GEMINI_API_KEY", key.clone()));
        if let Some(url) = &p.base_url {
            env.push(("GOOGLE_GEMINI_BASE_URL", url.clone()));
        }
        Some(lease)
    } else {
        if !unchanged() {
            return Err("Agy card changed".into());
        }
        if let Some(model) = &p.model {
            args.push(format!("--model={model}"));
        }
        None
    };
    args.extend(m.args);
    Ok(ProviderLaunch {
        program: m.program,
        cwd: m.cwd,
        args,
        env,
        private: None,
        agy: lease,
        agy_catalog: catalog,
    })
}
fn safe(_: WriteError) -> String {
    "Agy profile unavailable or changed; retry after resolving conflict".into()
}

/// Native Agy settings contain only a display model. Keys remain process-only.
pub fn import(bytes: &[u8]) -> Result<Option<AgyProfile>, String> {
    super::import::reject_placeholders(bytes).map_err(|_| "invalid Agy native configuration")?;
    let doc = super::live::patch_json::object_value(bytes)
        .map_err(|_| "invalid Agy native configuration")?;
    let model = match doc.get("model") {
        None | Some(serde_json::Value::Null) => return Ok(None),
        Some(serde_json::Value::String(s)) if s.trim().is_empty() => return Ok(None),
        Some(serde_json::Value::String(s)) => s.clone(),
        _ => return Err("invalid Agy native model".into()),
    };
    let p =
        AgyProfile { name: "default".into(), model: Some(model), api_key: None, base_url: None };
    validate(&p)?;
    Ok(Some(p))
}
