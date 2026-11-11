//! Revision-bound acknowledgement; never observes generations or restarts clients.
use super::{catalog_generation, stale_clients::ClientsState};
use crate::providers::{
    codex_catalog::CodexFiles,
    live::{
        commit::ProviderTransactions,
        write_guard::{FilePrivacy, WriteError},
    },
};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use vw_api_types::id::ProjectScope;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Acknowledged {
    revision: String,
}
pub const ACK_FILENAME: &str = "shiju-codex-client-ack.json";
/// Authorize execution host/project first and retain the physical guard throughout.
pub fn acknowledge(
    files: &mut CodexFiles,
    transactions: &ProviderTransactions,
    scope: &ProjectScope,
    expected_revision: &str,
) -> Result<String, WriteError> {
    let revision = catalog_generation::read(files, transactions, scope)?.revision();
    if revision != expected_revision {
        return Err(WriteError::Conflict);
    }
    let bytes = serde_json::to_vec(&Acknowledged { revision: revision.clone() })
        .map_err(|_| WriteError::InvalidResource)?;
    files.guard.write(OsStr::new(ACK_FILENAME), &bytes, FilePrivacy::Private)?;
    Ok(revision)
}
/// Confirmed stale processes remain visible after acknowledgement.
/// Desktop/editor clients require manual quitting and reopening, never automatic restart.
pub fn reminder(
    files: &CodexFiles,
    transactions: &ProviderTransactions,
    scope: &ProjectScope,
    clients: &ClientsState,
) -> Result<Option<&'static str>, WriteError> {
    let revision = catalog_generation::read(files, transactions, scope)?.revision();
    if revision != clients.revision {
        return Err(WriteError::Conflict);
    }
    let acknowledged = match files.guard.read(OsStr::new(ACK_FILENAME))? {
        None => false,
        Some(bytes) => {
            let ack: Acknowledged =
                serde_json::from_slice(&bytes).map_err(|_| WriteError::InvalidResource)?;
            if ack.revision.len() != 64 || !ack.revision.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(WriteError::InvalidResource);
            }
            ack.revision == revision
        }
    };
    Ok((clients.others || (clients.unverified && !acknowledged))
        .then_some("请彻底退出 Codex 桌面客户端并重新打开；编辑器插件请关闭并重开编辑器窗口。"))
}
