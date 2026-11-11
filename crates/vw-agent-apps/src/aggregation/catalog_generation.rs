//! Host-private Codex catalog and file-login observations. Never probes or restarts clients.
use crate::providers::{
    codex_catalog::{CATALOG_FILENAME, CodexFiles},
    codex_login_material::extract_codex_auth_user_identity,
    live::{
        commit::ProviderTransactions,
        write_guard::{FilePrivacy, WriteError},
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use vw_api_types::{agent_apps::AppId, id::ProjectScope};

pub const HISTORY_FILENAME: &str = "shiju-codex-client-history.json";
const LIMIT: usize = 32;
const MARGIN_MS: u64 = 2_000;
const NONE: &str = "none";

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Generation {
    since_ms: u64,
    fingerprint: String,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct History {
    catalog: Vec<Generation>,
    login: Vec<Generation>,
}

/// Internal observation; only its opaque revision belongs in public DTOs.
/// Fingerprints and native identities deliberately have no Debug/Serialize implementation.
pub struct Snapshot {
    history: History,
    binding: String,
    catalog: String,
    catalog_is_external: bool,
    login: Option<String>,
}
impl Snapshot {
    /// A validated foreign pointer is never followed; the gateway can suppress catalog-only hints.
    pub fn catalog_is_external(&self) -> bool {
        self.catalog_is_external
    }
    pub fn revision(&self) -> String {
        digest(
            &serde_json::to_vec(&(&self.binding, &self.catalog, &self.login))
                .expect("string tuple"),
        )
    }
    #[cfg(test)]
    pub(super) fn generation_counts(&self) -> (usize, usize) {
        (self.history.catalog.len(), self.history.login.len())
    }
    /// Unobserved external changes remain hints; reading never repairs history.
    pub fn unverified(&self) -> bool {
        self.history.catalog.last().is_none_or(|g| g.fingerprint != self.catalog)
            || self.login.as_ref().is_some_and(|login| {
                self.history.login.last().is_none_or(|g| &g.fingerprint != login)
            })
    }
    pub fn catalog_stale_at(&self, started_ms: u64) -> bool {
        stale(&self.history.catalog, &self.catalog, started_ms)
    }
    pub fn login_stale_at(&self, started_ms: u64) -> bool {
        // A first login is a baseline, not evidence of a switch.
        self.login.as_ref().is_some_and(|login| {
            self.history.login.len() > 1 && stale(&self.history.login, login, started_ms)
        })
    }
}

/// Caller authorizes execution host/project/app before obtaining CodexFiles.
/// Retain its physical guard for observation; pending publication is never observed.
pub fn read(
    files: &CodexFiles,
    transactions: &ProviderTransactions,
    scope: &ProjectScope,
) -> Result<Snapshot, WriteError> {
    if files.guard.app() != AppId::Codex {
        return Err(WriteError::InvalidResource);
    }
    transactions
        .state(AppId::Codex)?
        .require_project(scope)
        .map_err(|_| WriteError::InvalidResource)?;
    if transactions.pending(AppId::Codex)?.is_some() {
        return Err(WriteError::Recovered);
    }
    let history = match files.guard.read(OsStr::new(HISTORY_FILENAME))? {
        Some(bytes) => {
            serde_json::from_slice::<History>(&bytes).map_err(|_| WriteError::InvalidResource)?
        }
        None => History::default(),
    };
    for rows in [&history.catalog, &history.login] {
        if rows.len() > LIMIT
            || rows.windows(2).any(|w| w[0].since_ms > w[1].since_ms)
            || rows.iter().any(|g| {
                g.fingerprint != NONE
                    && (g.fingerprint.len() != 64
                        || !g.fingerprint.bytes().all(|c| c.is_ascii_hexdigit()))
            })
        {
            return Err(WriteError::InvalidResource);
        }
    }
    let bytes = files.guard.read(OsStr::new("config.toml"))?.unwrap_or_default();
    let config: toml_edit::DocumentMut = std::str::from_utf8(&bytes)
        .map_err(|_| WriteError::InvalidResource)?
        .parse()
        .map_err(|_| WriteError::InvalidResource)?;
    let mut catalog_is_external = false;
    let catalog = match config.get("model_catalog_json") {
        None => NONE.into(),
        Some(value) => {
            let pointer = value
                .as_str()
                .filter(|s| !s.trim().is_empty() && !s.chars().any(char::is_control))
                .ok_or(WriteError::InvalidResource)?;
            // Only the exact native owner's relative file is managed. Never follow a foreign path.
            if pointer == CATALOG_FILENAME {
                files
                    .guard
                    .read(OsStr::new(CATALOG_FILENAME))?
                    .map(|b| digest(&b))
                    .ok_or(WriteError::InvalidResource)?
            } else {
                catalog_is_external = true;
                NONE.into()
            }
        }
    };
    let login = match config.get("cli_auth_credentials_store") {
        None => file_login(files)?,
        Some(value) => match value.as_str() {
            Some("file") => file_login(files)?,
            Some("keyring" | "auto" | "ephemeral") => None,
            _ => return Err(WriteError::InvalidResource),
        },
    };
    let binding = digest(
        &serde_json::to_vec(&(scope, transactions.execution_host(), AppId::Codex))
            .map_err(|_| WriteError::InvalidResource)?,
    );
    Ok(Snapshot { history, binding, catalog, catalog_is_external, login })
}

/// Called by publication/reconnect lifecycle, never by a state GET.
pub fn observe(
    files: &mut CodexFiles,
    transactions: &ProviderTransactions,
    scope: &ProjectScope,
    now_ms: u64,
) -> Result<Snapshot, WriteError> {
    let mut snapshot = read(files, transactions, scope)?;
    let catalog_changed =
        snapshot.history.catalog.last().is_none_or(|g| g.fingerprint != snapshot.catalog);
    let login_changed = snapshot
        .login
        .as_ref()
        .is_some_and(|login| snapshot.history.login.last().is_none_or(|g| &g.fingerprint != login));
    if !catalog_changed && !login_changed {
        return Ok(snapshot);
    }
    // A clock rollback must not backdate a newly observed state.
    if [&snapshot.history.catalog, &snapshot.history.login]
        .into_iter()
        .filter_map(|rows| rows.last())
        .any(|g| now_ms < g.since_ms)
    {
        return Err(WriteError::Conflict);
    }
    if catalog_changed {
        append(&mut snapshot.history.catalog, &snapshot.catalog, now_ms);
    }
    if login_changed {
        append(
            &mut snapshot.history.login,
            snapshot.login.as_deref().expect("checked login"),
            now_ms,
        );
    }
    let bytes = serde_json::to_vec(&snapshot.history).map_err(|_| WriteError::InvalidResource)?;
    files.guard.write(OsStr::new(HISTORY_FILENAME), &bytes, FilePrivacy::Private)?;
    Ok(snapshot)
}

fn append(rows: &mut Vec<Generation>, fingerprint: &str, since_ms: u64) {
    rows.push(Generation { since_ms, fingerprint: fingerprint.into() });
    rows.drain(..rows.len().saturating_sub(LIMIT));
}

fn file_login(files: &CodexFiles) -> Result<Option<String>, WriteError> {
    let Some(bytes) = files.guard.read(OsStr::new("auth.json"))? else {
        return Ok(None);
    };
    let auth: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| WriteError::InvalidResource)?;
    let account = auth
        .pointer("/tokens/account_id")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.trim().is_empty());
    Ok(account.map(|account| {
        digest(
            &serde_json::to_vec(&(account, extract_codex_auth_user_identity(&auth)))
                .expect("identity tuple"),
        )
    }))
}
fn stale(history: &[Generation], current: &str, started_ms: u64) -> bool {
    history
        .iter()
        .rev()
        .find(|g| g.since_ms.saturating_add(MARGIN_MS) <= started_ms)
        .map_or(current != NONE, |g| g.fingerprint != current)
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}
