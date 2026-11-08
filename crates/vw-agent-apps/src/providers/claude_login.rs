//! Login transaction coordinated with the Claude file transaction, never an account database.
use super::live::{commit::ProviderTransactions, write_guard::WriteError};
use crate::auth::{
    claude_keychain::ClaudeKeychain,
    claude_login_files::{CliLock, Directory},
    claude_rotation::check_generation,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{fs::File, path::Path};
use vw_api_types::{agent_apps::AppId, id::TenantScope};
const ACCOUNT_CACHE_KEYS: &[&str] = &[
    "additionalModelOptionsCache",
    "additionalModelOptionsAnsweredAt",
    "additionalModelCostsCache",
    "modelAccessCache",
    "orgModelDefaultCache",
    "cachedArtifactRoster",
    "artifactRosterDenied",
    "lastSeenOrgDefaultUpdatedAt",
    "clientDataCache",
    "clientDataCacheSlots",
    "autoCompactWindowsCache",
    "cachedUsageUtilization",
    "metricsStatusCache",
    "metricsStatusCacheByPrincipal",
    "githubWebConnectionStatusCache",
    "unusedLimitResetStatusCache",
    "startupPrefetchedAt",
    "cachedExtraUsageDisabledReason",
    "penguinModeOrgEnabled",
];
const JOURNAL: &str = ".shiju-login.json";
#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoginPhase {
    Journal,
    Keychain,
    Fallback,
    Profile,
    Cleared,
}
#[cfg(test)]
thread_local! {
    pub(crate) static LOGIN_CHECKPOINT: std::cell::Cell<Option<LoginPhase>> = const { std::cell::Cell::new(None) };
}
#[cfg(test)]
fn checkpoint(phase: LoginPhase) {
    if LOGIN_CHECKPOINT.get() == Some(phase) {
        std::process::exit(86);
    }
}
/// Configuration-only operations must not bypass the login owner's unfinished journal.
pub(crate) fn configuration_has_pending_login(
    guard: &super::live::write_guard::AppWriteGuard,
) -> Result<bool, WriteError> {
    Ok(guard.read(std::ffi::OsStr::new(JOURNAL))?.is_some())
}
const ACCOUNT_KEYS: &[&str] = &[
    "claudeAiOauth",
    "organizationUuid",
    "trustedDeviceToken",
    "enterpriseGateway",
    "designOauth",
    "trustedDeviceEnrolledAfterRefusalAtMs",
    "cloudSessionsOauth",
];
/// Secret material stays on the execution host, without Debug or wire serialization.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginMaterial {
    secret: Option<Value>,
    keychain: Option<Value>,
    profile: Option<Vec<u8>>,
    fallback: Option<Vec<u8>>,
}
#[derive(Serialize, Deserialize)]
struct Journal {
    host: super::live::commit::ExecutionHostId,
    tenant: TenantScope,
    operation: String,
    before: LoginMaterial,
    after: LoginMaterial,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum SnapshotAction {
    CheckOut,
    ReplaceLive,
}
pub struct LoginHost {
    home: Directory,
    default: Directory,
    _lock: File,
    home_path: std::path::PathBuf,
    _cli_locks: Vec<CliLock>,
    target_locks: std::sync::Mutex<Vec<CliLock>>,
}
impl LoginHost {
    /// The administrator-authorized HOME, never a request path. Lock covers login and file commit.
    pub fn open(home: &Path) -> Result<Self, WriteError> {
        let directory = Directory::open(&home.join(".claude"))?;
        let lock = directory.lock()?;
        let home_directory = Directory::open(home)?;
        let locks = vec![
            directory.cli_lock(".oauth_refresh.lock")?,
            directory.cli_lock(".storage-write.lock")?,
            home_directory.cli_lock(".claude.lock")?,
            home_directory.cli_lock(".claude.json.lock")?,
        ];
        Ok(Self {
            home: home_directory,
            default: directory,
            _lock: lock,
            home_path: home.into(),
            _cli_locks: locks,
            target_locks: std::sync::Mutex::new(Vec::new()),
        })
    }
    pub fn has_journal(&self) -> Result<bool, WriteError> {
        Ok(self.default.read(JOURNAL)?.is_some())
    }
    pub(crate) fn material(
        &self,
        keychain: &dyn ClaudeKeychain,
    ) -> Result<LoginMaterial, WriteError> {
        keychain.authorize_home(&self.home_path).map_err(|_| WriteError::InvalidResource)?;
        LoginMaterial::read(&self.default, &self.home, None, keychain)
    }
    fn install(
        &self,
        keychain: &dyn ClaudeKeychain,
        material: &LoginMaterial,
    ) -> Result<(), WriteError> {
        keychain.authorize_home(&self.home_path).map_err(|_| WriteError::InvalidResource)?;
        keychain.write(None, material.keychain.as_ref()).map_err(|_| WriteError::Io)?;
        #[cfg(test)]
        checkpoint(LoginPhase::Keychain);
        if keychain.read(None).map_err(|_| WriteError::Io)? != material.keychain {
            return Err(WriteError::Io);
        }
        self.default.write(".credentials.json", material.fallback.as_deref())?;
        #[cfg(test)]
        checkpoint(LoginPhase::Fallback);
        self.home.write(".claude.json", material.profile.as_deref())?;
        #[cfg(test)]
        checkpoint(LoginPhase::Profile);
        Ok(())
    }
    /// A shared HOME must never let a different executor consume this login intent.
    pub fn journal_matches_host(&self, tx: &ProviderTransactions) -> Result<bool, WriteError> {
        self.default
            .read(JOURNAL)?
            .map(|bytes| {
                let journal: Journal =
                    serde_json::from_slice(&bytes).map_err(|_| WriteError::InvalidResource)?;
                Ok(&journal.host == tx.execution_host())
            })
            .transpose()
            .map(|matches| matches.unwrap_or(true))
    }
    pub fn recover(
        &self,
        tx: &ProviderTransactions,
        keychain: &dyn ClaudeKeychain,
    ) -> Result<(), WriteError> {
        let Some(bytes) = self.default.read(JOURNAL)? else {
            return Ok(());
        };
        let journal: Journal =
            serde_json::from_slice(&bytes).map_err(|_| WriteError::InvalidResource)?;
        if &journal.host != tx.execution_host() {
            return Err(WriteError::Conflict);
        }
        let matches_operation = |mode: &super::live::commit::ModeState| {
            mode.contract
                .as_ref()
                .and_then(|c| c.claude_login_operation.as_deref())
                .is_some_and(|op| op == journal.operation)
        };
        let current = self.material(keychain)?;
        // Each physical write can survive independently, including during rollback.
        // The effective secret alone cannot distinguish an absent fallback from a later login.
        let Journal { before, after, .. } = &journal;
        let known_keychain =
            current.keychain == before.keychain || current.keychain == after.keychain;
        let known_fallback =
            current.fallback == before.fallback || current.fallback == after.fallback;
        let known_profile = current.profile == before.profile || current.profile == after.profile;
        if !known_keychain || !known_fallback || !known_profile {
            return Err(WriteError::Conflict);
        }
        if let Some(pending) = tx.pending(AppId::Claude)? {
            if pending.tenant != journal.tenant
                || !pending.target.mode.as_ref().is_some_and(matches_operation)
            {
                return Err(WriteError::Conflict);
            }
            // Settle the provider intent first so the durable state decides the direction.
            return Err(WriteError::Recovered);
        }
        let state = tx.state(AppId::Claude)?;
        let forward = state.mode_tenant.as_ref() == Some(&journal.tenant)
            && state.mode.as_ref().is_some_and(matches_operation);
        let expected = if forward { after } else { before };
        if &current != expected {
            self.install(keychain, expected)?;
        }
        self.default.write(JOURNAL, None)
    }
    pub fn begin(
        &self,
        tx: &ProviderTransactions,
        tenant: &TenantScope,
        target: &Path,
        keychain: &dyn ClaudeKeychain,
        archive: impl FnOnce(&LoginMaterial) -> Result<(), WriteError>,
    ) -> Result<Option<String>, WriteError> {
        self.begin_snapshot(tx, tenant, target, keychain, SnapshotAction::CheckOut, archive)
    }
    /// Install new sign-in material only if this managed account already owns the live slot.
    /// The caller holds the account lease, as for a provider-bound checkout.
    pub fn replace_live(
        &self,
        tx: &ProviderTransactions,
        tenant: &TenantScope,
        target: &Path,
        keychain: &dyn ClaudeKeychain,
    ) -> Result<bool, WriteError> {
        if self
            .begin_snapshot(tx, tenant, target, keychain, SnapshotAction::ReplaceLive, |_| Ok(()))?
            .is_none()
        {
            return Ok(false);
        }
        // The snapshot is complete. Unlike checkout, no provider publication is required.
        self.default.write(JOURNAL, None)?;
        #[cfg(test)]
        checkpoint(LoginPhase::Cleared);
        Ok(true)
    }
    fn begin_snapshot(
        &self,
        tx: &ProviderTransactions,
        tenant: &TenantScope,
        target: &Path,
        keychain: &dyn ClaudeKeychain,
        action: SnapshotAction,
        archive: impl FnOnce(&LoginMaterial) -> Result<(), WriteError>,
    ) -> Result<Option<String>, WriteError> {
        if self.has_journal()? || tx.pending(AppId::Claude)?.is_some() {
            return Err(WriteError::Conflict);
        }
        let (dir, locks) = lock_account(target)?;
        *self.target_locks.lock().map_err(|_| WriteError::Conflict)? = locks;
        let before = self.material(keychain)?;
        let read_target = || LoginMaterial::read(&dir, &dir, Some(target), keychain);
        let account = read_target()?;
        let Some(identity) = account.identity()? else {
            return Err(WriteError::InvalidResource);
        };
        let same_account = before.identity()?.as_ref() == Some(&identity);
        if same_account != (action == SnapshotAction::ReplaceLive) {
            return Ok(None);
        }
        let after = merge(&before, &account)?;
        if action == SnapshotAction::CheckOut {
            archive(&before)?;
        }
        // Archival can wait for the OS. Never overwrite a login rotated in that interval.
        if self.material(keychain)? != before || read_target()? != account {
            return Err(WriteError::Conflict);
        }
        let operation = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| WriteError::Io)?
                .as_nanos()
        );
        let journal = Journal {
            host: tx.execution_host().clone(),
            tenant: tenant.clone(),
            operation: operation.clone(),
            before,
            after,
        };
        self.default
            .write(JOURNAL, Some(&serde_json::to_vec(&journal).map_err(|_| WriteError::Io)?))?;
        #[cfg(test)]
        checkpoint(LoginPhase::Journal);
        if let Err(error) = self.install(keychain, &journal.after) {
            // A rejected keychain write can leave everything intact. Avoid a second
            // write to the rejected destination when no rollback is needed.
            if self.material(keychain).is_ok_and(|current| current == journal.before) {
                let _ = self.default.write(JOURNAL, None);
            } else {
                let _ = self.recover(tx, keychain);
            }
            return Err(error);
        }
        Ok(Some(operation))
    }
    pub fn settle(
        &self,
        tx: &ProviderTransactions,
        keychain: &dyn ClaudeKeychain,
    ) -> Result<(), WriteError> {
        self.recover(tx, keychain)
    }
}
impl LoginMaterial {
    pub(crate) fn oauth(&self) -> Option<&Value> {
        self.secret.as_ref()?.get("claudeAiOauth")
    }

    pub fn identity(&self) -> Result<Option<String>, WriteError> {
        if !self
            .oauth()
            .and_then(|oauth| oauth.get("refreshToken"))
            .and_then(Value::as_str)
            .is_some_and(|token| !token.is_empty())
        {
            return Ok(None);
        }
        let (id, org) = self.profile_identity()?.ok_or(WriteError::InvalidResource)?;
        if self
            .secret
            .as_ref()
            .and_then(|v| v.get("organizationUuid"))
            .and_then(Value::as_str)
            .is_some_and(|s| s != org)
        {
            return Err(WriteError::InvalidResource);
        }
        Ok(Some(format!("{id}:{org}")))
    }
    fn profile_identity(&self) -> Result<Option<(String, String)>, WriteError> {
        let Some(profile) = self.profile.as_deref() else { return Ok(None) };
        let profile: Value =
            serde_json::from_slice(profile).map_err(|_| WriteError::InvalidResource)?;
        let Some(account) =
            profile.as_object().ok_or(WriteError::InvalidResource)?.get("oauthAccount")
        else {
            return Ok(None);
        };
        let valid = |id: &str| !id.contains(':') && !id.chars().any(char::is_control);
        let id = account
            .get("accountUuid")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty() && valid(id))
            .ok_or(WriteError::InvalidResource)?;
        let org = account.get("organizationUuid").and_then(Value::as_str).unwrap_or("");
        if !valid(org) {
            return Err(WriteError::InvalidResource);
        }
        Ok(Some((id.to_owned(), org.to_owned())))
    }
    pub fn from_account(path: &Path, keychain: &dyn ClaudeKeychain) -> Result<Self, WriteError> {
        let dir = Directory::open(path)?;
        Self::read(&dir, &dir, Some(path), keychain)
    }
    fn read(
        credentials: &Directory,
        profile: &Directory,
        keychain_path: Option<&Path>,
        keychain: &dyn ClaudeKeychain,
    ) -> Result<Self, WriteError> {
        let fallback = credentials.read(".credentials.json")?;
        let native = keychain.read(keychain_path).map_err(|_| WriteError::Io)?;
        let file_secret = fallback
            .as_deref()
            .map(serde_json::from_slice)
            .transpose()
            .map_err(|_| WriteError::InvalidResource)?;
        let secret = native.clone().or(file_secret);
        if secret.as_ref().is_some_and(|value| !value.is_object()) {
            return Err(WriteError::InvalidResource);
        }
        Ok(Self { secret, keychain: native, profile: profile.read(".claude.json")?, fallback })
    }
    /// The caller holds the scoped account lease. Recheck the destination after
    /// taking Claude's locks because its CLI can change the directory independently.
    pub fn archive(&self, path: &Path, keychain: &dyn ClaudeKeychain) -> Result<(), WriteError> {
        let owner = self.identity()?.ok_or(WriteError::InvalidResource)?;
        let (directory, _locks) = lock_account(path)?;
        let before = Self::read(&directory, &directory, Some(path), keychain)?;
        let existing = match before.identity()? {
            Some(identity) => Some(identity),
            None => before.profile_identity()?.map(|(id, org)| format!("{id}:{org}")),
        };
        if existing.as_ref().is_some_and(|identity| identity != &owner) {
            return Err(WriteError::Conflict);
        }
        check_generation(before.oauth(), self.oauth())?;
        let after = merge(&before, self)?;
        if Self::read(&directory, &directory, Some(path), keychain)? != before {
            return Err(WriteError::Conflict);
        }
        if existing.is_none() {
            // Establish ownership before publishing credentials so an interrupted
            // first archive can be retried against a recognizable destination.
            directory.write(".claude.json", after.profile.as_deref())?;
        }
        if before.keychain != after.secret {
            keychain.write(Some(path), after.secret.as_ref()).map_err(|_| WriteError::Io)?;
            if keychain.read(Some(path)).map_err(|_| WriteError::Io)? != after.secret {
                return Err(WriteError::Io);
            }
        }
        if before.fallback.is_some() {
            directory.write(".credentials.json", None)?;
        }
        Ok(())
    }
}
fn lock_account(path: &Path) -> Result<(Directory, Vec<CliLock>), WriteError> {
    let directory = Directory::open(path)?;
    let parent = Directory::open(path.parent().ok_or(WriteError::InvalidResource)?)?;
    let legacy = format!(
        "{}.lock",
        path.file_name().and_then(|name| name.to_str()).ok_or(WriteError::InvalidResource)?
    );
    let locks = vec![
        directory.cli_lock(".oauth_refresh.lock")?,
        directory.cli_lock(".storage-write.lock")?,
        directory.cli_lock(".claude.json.lock")?,
        parent.cli_lock(&legacy)?,
    ];
    Ok((directory, locks))
}
fn merge(base: &LoginMaterial, account: &LoginMaterial) -> Result<LoginMaterial, WriteError> {
    let mut secret = base.secret.clone().unwrap_or(json!({}));
    let map = secret.as_object_mut().ok_or(WriteError::InvalidResource)?;
    for key in ACCOUNT_KEYS {
        map.remove(*key);
        if let Some(value) = account.secret.as_ref().and_then(|v| v.get(*key)) {
            map.insert((*key).into(), value.clone());
        }
    }
    let mut profile: Value = serde_json::from_slice(base.profile.as_deref().unwrap_or(b"{}"))
        .map_err(|_| WriteError::InvalidResource)?;
    let target: Value = serde_json::from_slice(account.profile.as_deref().unwrap_or(b"{}"))
        .map_err(|_| WriteError::InvalidResource)?;
    let object = profile.as_object_mut().ok_or(WriteError::InvalidResource)?;
    object.remove("oauthAccount");
    if let Some(value) = target.get("oauthAccount") {
        object.insert("oauthAccount".into(), value.clone());
    }
    for key in ACCOUNT_CACHE_KEYS {
        object.remove(*key);
    }
    Ok(LoginMaterial {
        secret: Some(secret.clone()),
        keychain: Some(secret),
        profile: Some(serde_json::to_vec(&profile).map_err(|_| WriteError::InvalidResource)?),
        fallback: None,
    })
}
