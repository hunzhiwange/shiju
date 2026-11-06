//! Internal executor login preparation for the Codex supplier transaction (task 30).
//! No public auth-switch endpoint, discovery, OAuth or independent account store.
pub use super::codex_login_policy::AuthTarget;
pub use super::codex_login_policy::STASH_FILENAME;
use super::codex_login_policy::{AuthInput, LoginStash, plan};
use super::live::{
    stage::{FileChange, WholeFile},
    write_guard::{AppWriteGuard, FilePrivacy, WriteError},
};
use serde_json::Value;
use std::ffi::OsStr;

/// Facts come from the existing authorized local owner, never an HTTP payload.
pub struct LoginInput<'a> {
    pub target: AuthTarget<'a>,
    pub leaving_official: Option<&'a Value>,
    pub official_logins: &'a [Value],
    pub third_party_keys: &'a [String],
    pub live_is_managed: bool,
    pub now: i64,
}
/// Contains secrets; intentionally no Serialize or payload-bearing Debug.
/// Append `changes()` to config/catalog changes in the ONE ProviderTransactions commit.
pub struct PreparedLogin {
    auth: WholeFile,
    stash: WholeFile,
    pub login_on_disk: bool,
}
impl std::fmt::Debug for PreparedLogin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreparedLogin([redacted])")
    }
}
impl PreparedLogin {
    pub fn changes(&self) -> [FileChange<'_>; 2] {
        [
            FileChange {
                name: OsStr::new("auth.json"),
                privacy: FilePrivacy::Private,
                patch: &self.auth,
            },
            FileChange {
                name: OsStr::new(STASH_FILENAME),
                privacy: FilePrivacy::Private,
                patch: &self.stash,
            },
        ]
    }
}
/// Caller must authorize the host, lock its fixed .codex directory and recover
/// outstanding intent BEFORE preparing. Only file credential storage is supported.
/// Any recovery/conflict requires fresh preparation. Does not publish anything.
pub fn prepare(guard: &AppWriteGuard, input: LoginInput<'_>) -> Result<PreparedLogin, WriteError> {
    if guard.app() != vw_api_types::agent_apps::AppId::Codex {
        return Err(WriteError::InvalidResource);
    }
    let config = guard.read(OsStr::new("config.toml"))?;
    let config = std::str::from_utf8(config.as_deref().unwrap_or_default())
        .map_err(|_| WriteError::InvalidResource)?;
    let config: toml_edit::DocumentMut = config.parse().map_err(|_| WriteError::InvalidResource)?;
    if config.get("cli_auth_credentials_store").is_some_and(|v| v.as_str() != Some("file")) {
        return Err(WriteError::InvalidResource);
    }
    let auth = guard.read(OsStr::new("auth.json"))?;
    let live: Option<Value> = auth
        .as_deref()
        .map(serde_json::from_slice)
        .transpose()
        .map_err(|_| WriteError::InvalidResource)?;
    let stash_bytes = guard.read(OsStr::new(STASH_FILENAME))?;
    let stash = match stash_bytes.as_deref() {
        Some(bytes) => {
            let stash: LoginStash =
                serde_json::from_slice(bytes).map_err(|_| WriteError::InvalidResource)?;
            if stash.version > 1 {
                return Err(WriteError::InvalidResource);
            }
            stash.loaded(input.official_logins, live.as_ref())
        }
        None => LoginStash::seeded_from_rows(input.official_logins),
    };
    let planned = plan(AuthInput {
        live: live.as_ref(),
        live_is_managed: input.live_is_managed,
        third_party_keys: input.third_party_keys,
        leaving_official: input.leaving_official,
        target: input.target,
        stash,
        now: input.now,
    });
    let next_auth = match planned.auth {
        None => auth.clone(),
        Some(None) => None,
        Some(Some(value)) => {
            Some(serde_json::to_vec_pretty(&value).map_err(|_| WriteError::InvalidResource)?)
        }
    };
    let next_stash = match planned.stash {
        None => stash_bytes.clone(),
        Some(value) => {
            Some(serde_json::to_vec_pretty(&value).map_err(|_| WriteError::InvalidResource)?)
        }
    };
    Ok(PreparedLogin {
        auth: WholeFile::new(auth.as_deref(), next_auth),
        stash: WholeFile::new(stash_bytes.as_deref(), next_stash),
        login_on_disk: planned.login_on_disk,
    })
}
