//! Read-only dispatch for native MCP configuration on the authorized host.

use std::{collections::BTreeMap, path::Path};

use vw_api_types::{
    TenantScope,
    agent_apps::AppId,
    app_providers::paths::ConfigPathState,
    mcp_management::{McpAppTarget, McpConnectionDefinition, McpFailureCode},
};

use super::adapters::{
    agy, claude, codex, dsh, gemini, grokbuild, hermes, kimi, mcode, omp, opencode, pi,
};
use crate::{
    auth::credential_lease::CredentialLease,
    providers::{
        self,
        live::write_guard::{AppWriteGuard, WriteError, lock_directory},
        paths::{self, open_beneath},
    },
};

/// Trusted executor input. The caller resolves and authorizes the installation
/// and supplies its host home or an independently authorized Claude account.
#[derive(Clone, Copy)]
pub enum NativeMcpSource<'a> {
    HostHome(&'a Path),
    ClaudeAccount(&'a CredentialLease),
}

/// Host-private definition. Apply catalog credential policy before persistence.
pub struct NativeMcpCandidate {
    pub connection: McpConnectionDefinition,
    pub enabled: bool,
}

pub type NativeMcpCandidates = BTreeMap<String, Result<NativeMcpCandidate, McpFailureCode>>;

/// Read with the selected adapter. Paths and raw native errors stay on the host.
pub fn read(
    tenant: &TenantScope,
    target: &McpAppTarget,
    source: NativeMcpSource<'_>,
) -> Result<NativeMcpCandidates, McpFailureCode> {
    let app = target.app_id;
    match (app, source) {
        (AppId::Claude, NativeMcpSource::ClaudeAccount(account)) => {
            let (owner, host) = account
                .claude_profile_scope()
                .map_err(|_| McpFailureCode::CredentialUnavailable)?;
            if owner != tenant || host != &target.target_host_id {
                return Err(McpFailureCode::PermissionDenied);
            }
            Ok(candidates(claude::read(account).map_err(file_failure)?, always_enabled, invalid))
        }
        (AppId::Claude, _) => Err(McpFailureCode::CredentialUnavailable),
        (AppId::ClaudeDesktop | AppId::OpenClaw, _) | (_, NativeMcpSource::ClaudeAccount(_)) => {
            Err(McpFailureCode::TargetUnavailable)
        }
        (_, NativeMcpSource::HostHome(home)) => read_home(app, home),
    }
}

fn read_home(app: AppId, home: &Path) -> Result<NativeMcpCandidates, McpFailureCode> {
    match app {
        AppId::OpenCode => Ok(candidates(
            opencode::read(home).map_err(file_failure)?,
            |server| (server.connection, server.enabled),
            invalid,
        )),
        AppId::Hermes => {
            let bytes = providers::hermes::read(home).map_err(file_failure)?;
            Ok(candidates(
                hermes::read_servers(bytes.as_deref()).map_err(invalid)?,
                |server| (server.connection, server.enabled),
                invalid,
            ))
        }
        AppId::Dsh => {
            let snapshot = providers::dsh::read_mcp_profiles(home).map_err(file_failure)?;
            Ok(candidates(
                dsh::read_servers(&snapshot).map_err(dsh_failure)?,
                |server| (server.connection, server.enabled),
                dsh_failure,
            ))
        }
        AppId::Agy => {
            let Some(mut file) = agy::AgyMcp::open_existing(home).map_err(file_failure)? else {
                return Ok(BTreeMap::new());
            };
            Ok(candidates(
                file.read().map_err(file_failure)?,
                |server| (server.connection, server.enabled),
                invalid,
            ))
        }
        AppId::Codex
        | AppId::Gemini
        | AppId::GrokBuild
        | AppId::Pi
        | AppId::Mcode
        | AppId::Kimi
        | AppId::Omp => {
            let Some(mut guard) = existing_config(home, app)? else {
                return Ok(BTreeMap::new());
            };
            read_guarded(app, &mut guard)
        }
        AppId::Claude | AppId::ClaudeDesktop | AppId::OpenClaw => {
            Err(McpFailureCode::TargetUnavailable)
        }
    }
}

fn read_guarded(
    app: AppId,
    guard: &mut AppWriteGuard,
) -> Result<NativeMcpCandidates, McpFailureCode> {
    Ok(match app {
        AppId::Codex => candidates(
            codex::read(guard).map_err(|error| match error {
                codex::CodexError::Write(error) => file_failure(error),
                _ => McpFailureCode::InvalidDefinition,
            })?,
            |server| (server.connection, server.enabled),
            invalid,
        ),
        AppId::Gemini => candidates(
            gemini::GeminiMcp::new(guard).and_then(|file| file.read()).map_err(file_failure)?,
            always_enabled,
            invalid,
        ),
        AppId::GrokBuild => candidates(
            grokbuild::read(guard).map_err(|error| match error {
                grokbuild::GrokMcpError::Native(error) => file_failure(error),
                _ => McpFailureCode::InvalidDefinition,
            })?,
            always_enabled,
            invalid,
        ),
        AppId::Pi => candidates(
            pi::PiMcp::new(guard).and_then(|file| file.read()).map_err(file_failure)?,
            |server| (server.connection, server.enabled),
            |error| match error {
                pi::PiMcpError::File(error) => file_failure(error),
                _ => McpFailureCode::InvalidDefinition,
            },
        ),
        AppId::Mcode => candidates(
            mcode::read(guard).map_err(file_failure)?,
            |server| (server.connection, server.enabled),
            invalid,
        ),
        AppId::Kimi => candidates(
            kimi::read(guard).map_err(file_failure)?,
            |server| (server.connection, server.enabled),
            invalid,
        ),
        AppId::Omp => candidates(
            omp::OmpMcp::new(guard).and_then(|file| file.read()).map_err(file_failure)?,
            |server| (server.connection, server.enabled),
            invalid,
        ),
        _ => return Err(McpFailureCode::TargetUnavailable),
    })
}

pub(super) fn existing_config(
    home: &Path,
    app: AppId,
) -> Result<Option<AppWriteGuard>, McpFailureCode> {
    let location = paths::read(home, app, None, &paths::execution_environment())
        .map_err(|_| McpFailureCode::TargetUnavailable)?;
    match open_beneath(home, Path::new(&location.directory), true) {
        Ok(directory) => lock_directory(app, directory).map(Some).map_err(file_failure),
        Err(ConfigPathState::Absent) => Ok(None),
        Err(_) => Err(McpFailureCode::TargetUnavailable),
    }
}

fn always_enabled(connection: McpConnectionDefinition) -> (McpConnectionDefinition, bool) {
    (connection, true)
}

fn candidates<T, E>(
    entries: BTreeMap<String, Result<T, E>>,
    project: impl Fn(T) -> (McpConnectionDefinition, bool),
    failure: impl Fn(E) -> McpFailureCode,
) -> NativeMcpCandidates {
    entries
        .into_iter()
        .map(|(id, entry)| {
            let candidate = entry
                .map(|server| {
                    let (connection, enabled) = project(server);
                    NativeMcpCandidate { connection, enabled }
                })
                .map_err(&failure);
            (id, candidate)
        })
        .collect()
}

fn invalid<E>(_: E) -> McpFailureCode {
    McpFailureCode::InvalidDefinition
}

fn dsh_failure(error: dsh::DshError) -> McpFailureCode {
    match error {
        dsh::DshError::Native(error) => file_failure(error),
        _ => McpFailureCode::InvalidDefinition,
    }
}

fn file_failure(error: WriteError) -> McpFailureCode {
    match error {
        WriteError::InvalidResource => McpFailureCode::InvalidDefinition,
        WriteError::Io => McpFailureCode::TargetUnavailable,
        WriteError::Conflict | WriteError::Recovered => McpFailureCode::Conflict,
    }
}
