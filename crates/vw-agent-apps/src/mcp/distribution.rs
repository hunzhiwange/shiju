//! Native MCP write dispatch on the authorized host.

use std::path::Path;

use vw_api_types::{
    TenantScope,
    agent_apps::AppId,
    mcp_management::{McpAppTarget, McpConnectionDefinition, McpFailureCode, McpNativeStatus},
};

use super::{
    adapters::{
        agy, claude, codex, dsh, gemini, grokbuild, hermes, kimi, mcode, omp, opencode, pi,
    },
    import::{NativeMcpSource, existing_config},
};
use crate::providers::{
    dsh::DshFiles,
    live::{commit::ProviderTransactions, write_guard::WriteError},
    opencode::OpenCodeFiles,
};

#[derive(Clone, Copy)]
pub enum NativeMcpEdit<'a> {
    Insert(&'a McpConnectionDefinition),
    Update(&'a McpConnectionDefinition),
    Disable,
    Remove,
}

/// One adapter write. Only the catalog owner may select Update, Disable or
/// Remove for an already managed entry.
pub struct NativeMcpWrite<'a> {
    pub tenant: &'a TenantScope,
    pub target: &'a McpAppTarget,
    pub source: NativeMcpSource<'a>,
    pub transactions: &'a ProviderTransactions,
}

impl NativeMcpWrite<'_> {
    pub fn validate_target(&self) -> Result<(), McpFailureCode> {
        if !self.transactions.is_for_host(&self.target.target_host_id) {
            return Err(McpFailureCode::PermissionDenied);
        }
        match (self.target.app_id, self.source) {
            (AppId::Claude, NativeMcpSource::ClaudeAccount(account)) => {
                let (tenant, host) = account
                    .claude_profile_scope()
                    .map_err(|_| McpFailureCode::CredentialUnavailable)?;
                if tenant != self.tenant || host != &self.target.target_host_id {
                    return Err(McpFailureCode::PermissionDenied);
                }
                Ok(())
            }
            (AppId::Claude, _) => Err(McpFailureCode::CredentialUnavailable),
            (AppId::ClaudeDesktop | AppId::OpenClaw, _)
            | (_, NativeMcpSource::ClaudeAccount(_)) => Err(McpFailureCode::TargetUnavailable),
            (_, NativeMcpSource::HostHome(home)) if home.is_absolute() => Ok(()),
            _ => Err(McpFailureCode::TargetUnavailable),
        }
    }

    pub fn apply(
        &self,
        id: &str,
        edit: NativeMcpEdit<'_>,
        preflight: impl Fn() -> Result<(), WriteError>,
    ) -> Result<McpNativeStatus, McpFailureCode> {
        self.validate_target()?;
        match self.source {
            NativeMcpSource::ClaudeAccount(account) => {
                use claude::{ClaudeMcpEdit as Edit, ClaudeMcpError as Error};
                let edit = match edit {
                    NativeMcpEdit::Insert(connection) => Edit::Insert(connection),
                    NativeMcpEdit::Update(connection) => Edit::Update(connection),
                    NativeMcpEdit::Disable | NativeMcpEdit::Remove => Edit::Remove,
                };
                let keeps_entry = !matches!(edit, Edit::Remove);
                claude::apply(account, self.transactions, id, edit, preflight).map_err(
                    |error| match error {
                        Error::File(error) => write_failure(error),
                        _ => McpFailureCode::InvalidDefinition,
                    },
                )?;
                Ok(status(keeps_entry))
            }
            NativeMcpSource::HostHome(home) => self.apply_home(home, id, edit, preflight),
        }
    }

    fn apply_home(
        &self,
        home: &Path,
        id: &str,
        edit: NativeMcpEdit<'_>,
        preflight: impl Fn() -> Result<(), WriteError>,
    ) -> Result<McpNativeStatus, McpFailureCode> {
        use NativeMcpEdit::{Disable, Insert, Remove, Update};
        let (app, tx, tenant) = (self.target.app_id, self.transactions, self.tenant);
        let keeps_entry = match app {
            AppId::Agy => {
                use agy::{AgyMcpEdit as Edit, AgyMcpError as Error};
                let Some(mut file) = agy::AgyMcp::open_existing(home).map_err(write_failure)?
                else {
                    return absent(edit);
                };
                let (edit, keeps_entry) = match edit {
                    Insert(connection) => (Edit::Insert(connection), true),
                    Update(connection) => (Edit::Update(connection), true),
                    Disable => {
                        (Edit::Disable, file.read().map_err(write_failure)?.contains_key(id))
                    }
                    Remove => (Edit::Remove, false),
                };
                file.apply(tx, tenant, id, edit, preflight).map_err(|error| match error {
                    Error::File(error) => write_failure(error),
                    _ => McpFailureCode::InvalidDefinition,
                })?;
                keeps_entry
            }
            AppId::OpenCode => {
                use opencode::{Edit, OpenCodeMcpError as Error};
                let mut files = OpenCodeFiles::open(home).map_err(write_failure)?;
                let edit = match edit {
                    Insert(connection) => Edit::Insert(connection),
                    Update(connection) => Edit::Enable(connection),
                    Disable => Edit::SetEnabled(false),
                    Remove => Edit::Remove,
                };
                let keeps_entry = !matches!(edit, Edit::Remove);
                opencode::apply(&mut files, tx, tenant, id, edit, preflight).map_err(|error| {
                    match error {
                        Error::File(error) => write_failure(error),
                        _ => McpFailureCode::InvalidDefinition,
                    }
                })?;
                keeps_entry
            }
            AppId::Dsh => {
                use dsh::{DshError as Error, ServerEdit as Edit};
                let mut files = DshFiles::open(home).map_err(write_failure)?;
                let (edit, keeps_entry) = match edit {
                    Insert(connection) => (Edit::Insert(connection), true),
                    Update(connection) => (Edit::Update(connection), true),
                    Disable => {
                        let snapshot = files.snapshot().map_err(write_failure)?;
                        let entries =
                            dsh::read_servers(&snapshot).map_err(|error| match error {
                                Error::Native(error) => write_failure(error),
                                _ => McpFailureCode::InvalidDefinition,
                            })?;
                        (Edit::Disable, entries.contains_key(id))
                    }
                    Remove => (Edit::Remove, false),
                };
                dsh::sync_and_commit(&mut files, tx, tenant, id, edit, preflight, || Ok(()))
                    .map_err(|error| match error {
                        Error::Native(error) => write_failure(error),
                        _ => McpFailureCode::InvalidDefinition,
                    })?;
                keeps_entry
            }
            AppId::Claude | AppId::ClaudeDesktop | AppId::OpenClaw => {
                return Err(McpFailureCode::TargetUnavailable);
            }
            _ => return self.apply_guarded(home, id, edit, preflight),
        };
        Ok(status(keeps_entry))
    }

    fn apply_guarded(
        &self,
        home: &Path,
        id: &str,
        edit: NativeMcpEdit<'_>,
        preflight: impl Fn() -> Result<(), WriteError>,
    ) -> Result<McpNativeStatus, McpFailureCode> {
        use NativeMcpEdit::{Disable, Insert, Remove, Update};
        let (app, tx, tenant) = (self.target.app_id, self.transactions, self.tenant);
        let Some(mut guard) = existing_config(home, app)? else {
            return absent(edit);
        };
        let keeps_entry = match app {
            AppId::Codex => {
                use codex::{CodexEdit as Edit, CodexError as Error};
                let edit = match edit {
                    Insert(connection) => Edit::Insert(connection),
                    Update(connection) => Edit::Update(connection),
                    Disable | Remove => Edit::Remove,
                };
                let keeps_entry = !matches!(edit, Edit::Remove);
                codex::apply(&mut guard, tx, tenant, id, edit, preflight).map_err(|error| {
                    match error {
                        Error::Write(error) => write_failure(error),
                        _ => McpFailureCode::InvalidDefinition,
                    }
                })?;
                keeps_entry
            }
            AppId::Gemini => {
                use gemini::{GeminiMcp, GeminiMcpEdit as Edit, GeminiMcpError as Error};
                let edit = match edit {
                    Insert(connection) => Edit::Insert(connection),
                    Update(connection) => Edit::Update(connection),
                    Disable | Remove => Edit::Remove,
                };
                let keeps_entry = !matches!(edit, Edit::Remove);
                GeminiMcp::new(&mut guard)
                    .map_err(write_failure)?
                    .apply(tx, tenant, id, edit, preflight)
                    .map_err(|error| match error {
                        Error::File(error) => write_failure(error),
                        _ => McpFailureCode::InvalidDefinition,
                    })?;
                keeps_entry
            }
            AppId::GrokBuild => {
                use grokbuild::{GrokMcpEdit as Edit, GrokMcpError as Error};
                let edit = match edit {
                    Insert(connection) => Edit::Insert(connection),
                    Update(connection) => Edit::Update(connection),
                    Disable | Remove => Edit::Remove,
                };
                let keeps_entry = !matches!(edit, Edit::Remove);
                grokbuild::apply(&mut guard, tx, tenant, id, edit, preflight).map_err(|error| {
                    match error {
                        Error::Native(error) => write_failure(error),
                        _ => McpFailureCode::InvalidDefinition,
                    }
                })?;
                keeps_entry
            }
            AppId::Hermes => {
                use hermes::{ServerEdit as Edit, ServerPatch};
                let file = hermes::file(&mut guard).map_err(write_failure)?;
                let before = file.read().map_err(write_failure)?;
                let (edit, keeps_entry) = match edit {
                    Insert(connection) => (Edit::Insert(connection), true),
                    Update(connection) => (Edit::Enable(connection), true),
                    Disable => {
                        let entries = hermes::read_servers(before.as_deref())
                            .map_err(|_| McpFailureCode::InvalidDefinition)?;
                        (Edit::Disable, entries.contains_key(id))
                    }
                    Remove => (Edit::Remove, false),
                };
                let patch =
                    ServerPatch::new(id, edit).map_err(|_| McpFailureCode::InvalidDefinition)?;
                file.commit(tx, tenant, before.as_deref(), &patch, preflight)
                    .map_err(write_failure)?;
                keeps_entry
            }
            AppId::Pi => {
                use pi::{PiMcp, PiMcpEdit as Edit, PiMcpError as Error, PiMcpServer};
                let file = PiMcp::new(&mut guard).map_err(write_failure)?;
                let server;
                let (edit, keeps_entry) = match edit {
                    Insert(connection) => {
                        server = PiMcpServer {
                            connection: connection.clone(),
                            enabled: true,
                            options: Default::default(),
                        };
                        (Edit::Insert(&server), true)
                    }
                    Update(connection) => (Edit::Enable(connection), true),
                    Disable => (
                        Edit::SetEnabled(false),
                        file.read().map_err(write_failure)?.contains_key(id),
                    ),
                    Remove => (Edit::Remove, false),
                };
                file.apply(tx, tenant, id, edit, preflight).map_err(|error| match error {
                    Error::File(error) => write_failure(error),
                    _ => McpFailureCode::InvalidDefinition,
                })?;
                keeps_entry
            }
            AppId::Mcode => {
                use mcode::{McodeMcpError as Error, ServerEdit as Edit, ServerPatch};
                let edit = match edit {
                    Insert(connection) => Edit::Insert(connection),
                    Update(connection) => Edit::Update(connection),
                    Disable => Edit::Disable,
                    Remove => Edit::Remove,
                };
                let keeps_entry = matches!(edit, Edit::Insert(_) | Edit::Update(_));
                mcode::apply(
                    &mut guard,
                    tx,
                    tenant,
                    ServerPatch { id, edit },
                    preflight,
                    || Ok(()),
                )
                .map_err(|error| match error {
                    Error::File(error) => write_failure(error),
                    _ => McpFailureCode::InvalidDefinition,
                })?;
                keeps_entry
            }
            AppId::Kimi => {
                use kimi::{Edit, KimiMcpError as Error};
                let edit = match edit {
                    Insert(connection) => Edit::Insert(connection),
                    Update(connection) => Edit::Update(connection),
                    Disable | Remove => Edit::Remove,
                };
                let keeps_entry = !matches!(edit, Edit::Remove);
                kimi::apply(&mut guard, tx, tenant, id, edit, preflight).map_err(|error| {
                    match error {
                        Error::File(error) => write_failure(error),
                        _ => McpFailureCode::InvalidDefinition,
                    }
                })?;
                keeps_entry
            }
            AppId::Omp => {
                use omp::{OmpMcp, OmpMcpEdit as Edit, OmpMcpError as Error};
                let edit = match edit {
                    Insert(connection) => Edit::Insert(connection),
                    Update(connection) => Edit::Update { connection, enabled: true },
                    Disable => Edit::Disable,
                    Remove => Edit::Remove,
                };
                // A deny entry is also managed and must be revoked on deletion.
                let keeps_entry = !matches!(edit, Edit::Remove);
                OmpMcp::new(&mut guard)
                    .map_err(write_failure)?
                    .apply(tx, tenant, id, edit, preflight)
                    .map_err(|error| match error {
                        Error::File(error) => write_failure(error),
                        _ => McpFailureCode::InvalidDefinition,
                    })?;
                keeps_entry
            }
            _ => return Err(McpFailureCode::TargetUnavailable),
        };
        Ok(status(keeps_entry))
    }
}

fn absent(edit: NativeMcpEdit<'_>) -> Result<McpNativeStatus, McpFailureCode> {
    match edit {
        NativeMcpEdit::Insert(_) | NativeMcpEdit::Update(_) => {
            Err(McpFailureCode::TargetUnavailable)
        }
        NativeMcpEdit::Disable | NativeMcpEdit::Remove => Ok(McpNativeStatus::Missing),
    }
}

fn status(keeps_entry: bool) -> McpNativeStatus {
    if keeps_entry { McpNativeStatus::Present } else { McpNativeStatus::Missing }
}

fn write_failure(error: WriteError) -> McpFailureCode {
    match error {
        WriteError::Conflict | WriteError::Recovered => McpFailureCode::Conflict,
        WriteError::Io => McpFailureCode::NativeWriteFailed,
        WriteError::InvalidResource => McpFailureCode::InvalidDefinition,
    }
}
