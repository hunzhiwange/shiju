//! Pure authorization policy. Callers must supply the authenticated local OS user,
//! recheck this live policy before execution, and persist the returned config through
//! the existing user-private configuration store before acknowledging a settings edit.
//! Authorization does not prove backend readiness, system permission or lock support.

use vw_api_types::computer_control::{ComputerControlTarget, ControlErrorCode, PermissionVersion};
use vw_config_types::computer_control::{ComputerControlConfig, GrantDecision};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFeature {
    NativeApps,
    Chrome,
    Excel,
    Word,
    LockedUse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantScope {
    Application(String),
    ChromeOrigin(String),
    ExcelDocument(String),
    WordDocument(String),
}

pub struct ComputerControlPolicy {
    config: ComputerControlConfig,
}

impl ComputerControlPolicy {
    pub fn new(config: ComputerControlConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &ComputerControlConfig {
        &self.config
    }

    pub fn version(&self) -> PermissionVersion {
        PermissionVersion(self.config.permission_version)
    }

    /// `local_user` must come from the authenticated host session, not a tool argument.
    /// A grant authorizes only this target; navigation destinations and changed focus
    /// must be checked again by the executing backend before any action is applied.
    pub fn authorize(
        &self,
        local_user: &str,
        target: &ComputerControlTarget,
        locked: bool,
        version: PermissionVersion,
    ) -> Result<(), ControlErrorCode> {
        if version != self.version() {
            return Err(ControlErrorCode::StalePermission);
        }
        if !valid_identity(local_user) {
            return Err(ControlErrorCode::PermissionDenied);
        }
        let user = self.config.users.get(local_user).ok_or(ControlErrorCode::PermissionDenied)?;
        if locked && !user.locked_use {
            return Err(ControlErrorCode::PermissionDenied);
        }
        let allowed = match target {
            ComputerControlTarget::NativeApp { bundle_id, process_id } => {
                if !valid_bundle_id(bundle_id) || *process_id == 0 || specialized_app(bundle_id) {
                    false
                } else {
                    match user.applications.get(bundle_id) {
                        Some(GrantDecision::Deny) => false,
                        Some(GrantDecision::Allow) => true,
                        None => user.native_apps,
                    }
                }
            }
            ComputerControlTarget::ChromeTab { origin, .. } => {
                user.chrome
                    && valid_origin(origin)
                    && user.chrome_origins.get(origin) == Some(&GrantDecision::Allow)
            }
            ComputerControlTarget::ExcelWorkbook { document_id } => {
                user.excel
                    && valid_identity(&document_id.0)
                    && user.excel_documents.get(&document_id.0) == Some(&GrantDecision::Allow)
            }
            ComputerControlTarget::WordDocument { document_id } => {
                user.word
                    && valid_identity(&document_id.0)
                    && user.word_documents.get(&document_id.0) == Some(&GrantDecision::Allow)
            }
        };
        if allowed { Ok(()) } else { Err(ControlErrorCode::PermissionDenied) }
    }

    pub fn set_enabled(
        &mut self,
        local_user: &str,
        feature: ControlFeature,
        enabled: bool,
    ) -> Result<PermissionVersion, &'static str> {
        self.validate_edit(local_user)?;
        let user = self.config.users.entry(local_user.to_owned()).or_default();
        let setting = match feature {
            ControlFeature::NativeApps => &mut user.native_apps,
            ControlFeature::Chrome => &mut user.chrome,
            ControlFeature::Excel => &mut user.excel,
            ControlFeature::Word => &mut user.word,
            ControlFeature::LockedUse => &mut user.locked_use,
        };
        *setting = enabled;
        self.config.permission_version += 1;
        Ok(self.version())
    }

    /// `None` deletes a persistent entry. To forbid an application while the broad
    /// native-app switch is on, store `Deny` rather than removing the entry.
    pub fn set_grant(
        &mut self,
        local_user: &str,
        scope: GrantScope,
        decision: Option<GrantDecision>,
    ) -> Result<PermissionVersion, &'static str> {
        self.validate_edit(local_user)?;
        let valid = match &scope {
            GrantScope::Application(id) => valid_bundle_id(id) && !specialized_app(id),
            GrantScope::ChromeOrigin(origin) => valid_origin(origin),
            GrantScope::ExcelDocument(id) | GrantScope::WordDocument(id) => valid_identity(id),
        };
        if !valid {
            return Err("invalid computer-control grant identity");
        }
        let user = self.config.users.entry(local_user.to_owned()).or_default();
        let (grants, key) = match scope {
            GrantScope::Application(id) => (&mut user.applications, id),
            GrantScope::ChromeOrigin(origin) => (&mut user.chrome_origins, origin),
            GrantScope::ExcelDocument(id) => (&mut user.excel_documents, id),
            GrantScope::WordDocument(id) => (&mut user.word_documents, id),
        };
        match decision {
            Some(decision) => {
                grants.insert(key, decision);
            }
            None => {
                grants.remove(&key);
            }
        }
        self.config.permission_version += 1;
        Ok(self.version())
    }

    fn validate_edit(&self, local_user: &str) -> Result<(), &'static str> {
        if !valid_identity(local_user) {
            return Err("invalid local user identity");
        }
        if self.config.permission_version == u64::MAX {
            return Err("computer-control permission version exhausted");
        }
        Ok(())
    }
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.chars().any(char::is_control)
        && value.trim() == value
}

fn valid_bundle_id(value: &str) -> bool {
    value.len() <= 256
        && value.contains('.')
        && value.split('.').all(|part| {
            !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

// The generic native route must never bypass website or Office document grants.
fn specialized_app(bundle_id: &str) -> bool {
    bundle_id.eq_ignore_ascii_case("com.microsoft.Excel")
        || bundle_id.eq_ignore_ascii_case("com.microsoft.Word")
        || bundle_id.eq_ignore_ascii_case("com.google.Chrome")
        || bundle_id.to_ascii_lowercase().starts_with("com.google.chrome.")
}

fn valid_origin(value: &str) -> bool {
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https")
        && url.username().is_empty()
        && url.password().is_none()
        && url.origin().ascii_serialization() == value
}
