//! Local administrator-reviewed GNU stdio selections. Supplier metadata grants nothing.

use serde::{Deserialize, Serialize};
use vw_api_mcp_management::{
    McpCatalogRecord, McpCatalogScope, McpConnectionDefinition, McpFailureCode,
    McpLinuxStdioSelection, McpRecordRef, McpRuntimeSelection,
};
use vw_api_transport::ProjectScope;

use super::{
    McpCatalogStore,
    runtime::{StoredSelection, selection_namespace},
};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StdioReview {
    sha256: [u8; 32],
    allowed_tools: Vec<String>,
}

impl McpCatalogStore {
    /// The gateway must authorize both project catalog writes and local host configuration writes.
    pub fn configure_linux_stdio(
        &self,
        project: &ProjectScope,
        record: &McpRecordRef,
        action: McpLinuxStdioSelection,
    ) -> Result<McpCatalogRecord, McpFailureCode> {
        if !cfg!(all(
            target_os = "linux",
            target_env = "gnu",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )) {
            return Err(McpFailureCode::InvalidDefinition);
        }
        let project = catalog_project(project)?;
        let scope = McpCatalogScope::Project { scope: project.clone() };
        let active = self.active_record(&scope, &record.id)?;
        if &active.record.record != record {
            return Err(McpFailureCode::StaleRevision);
        }
        let selection = McpRuntimeSelection { project: project.clone(), record: record.clone() };
        match action {
            McpLinuxStdioSelection::Select { sha256, allowed_tools } => {
                let (command, _) = fixed_command(&active.connection)?;
                validate_local_entry(command)?;
                let allowed_tools = reviewed_tools(allowed_tools)?;
                let review = StdioReview { sha256: digest(&sha256)?, allowed_tools };
                self.select_for_runtime_with_review(&project, &scope, &selection, Some(review))?;
            }
            McpLinuxStdioSelection::Disable => {
                let selected = self.linux_stdio_selection(&project, &record.id.0)?;
                if selected.record != *record {
                    return Err(McpFailureCode::StaleRevision);
                }
                self.disable_for_runtime(&project, &scope, &selection)?;
            }
        }
        Ok(active.record)
    }

    fn linux_stdio_selection(
        &self,
        project: &ProjectScope,
        name: &str,
    ) -> Result<StoredSelection, McpFailureCode> {
        let payload = self
            .0
            .get(project.tenant_scope(), &selection_namespace(project)?, name)
            .map_err(|_| McpFailureCode::StorageUnavailable)?
            .ok_or(McpFailureCode::NotFound)?;
        let selected: StoredSelection =
            serde_json::from_str(&payload).map_err(|_| McpFailureCode::StorageUnavailable)?;
        // Transport scopes omit the authenticated user from serialization. These
        // entries are project-wide, so compare the persisted company and project.
        let scope = McpCatalogScope::Project { scope: catalog_project(project)? };
        if selected.scope != scope || selected.record.id.0 != name || selected.linux_stdio.is_none()
        {
            return Err(McpFailureCode::PermissionDenied);
        }
        Ok(selected)
    }
}

fn catalog_project(project: &ProjectScope) -> Result<ProjectScope, McpFailureCode> {
    ProjectScope::new(
        project.tenant_scope().clone().with_user_id(Default::default()),
        project.project_id().clone(),
    )
    .map_err(|_| McpFailureCode::PermissionDenied)
}

#[cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn validate_local_entry(command: &std::path::Path) -> Result<(), McpFailureCode> {
    use std::{
        io::Read,
        os::unix::fs::{OpenOptionsExt, PermissionsExt},
    };

    let invalid = McpFailureCode::InvalidDefinition;
    if command.canonicalize().ok().as_deref() != Some(command) {
        return Err(invalid);
    }
    let mut entry = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(command)
        .map_err(|_| invalid)?;
    let metadata = entry.metadata().map_err(|_| invalid)?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err(invalid);
    }
    let mut header = [0; 20];
    entry.read_exact(&mut header).map_err(|_| invalid)?;
    let machine = u16::from_le_bytes([header[18], header[19]]);
    #[cfg(target_arch = "x86_64")]
    let expected_machine = 62;
    #[cfg(target_arch = "aarch64")]
    let expected_machine = 183;
    if !header.starts_with(b"\x7fELF\x02\x01") || machine != expected_machine {
        return Err(invalid);
    }
    Ok(())
}

#[cfg(not(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
)))]
fn validate_local_entry(_: &std::path::Path) -> Result<(), McpFailureCode> {
    Err(McpFailureCode::InvalidDefinition)
}

fn fixed_command(
    connection: &McpConnectionDefinition,
) -> Result<(&std::path::Path, &[String]), McpFailureCode> {
    match connection {
        McpConnectionDefinition::Stdio { command, args, env, cwd }
            if std::path::Path::new(command).is_absolute() && env.is_empty() && cwd.is_none() =>
        {
            Ok((std::path::Path::new(command), args))
        }
        _ => Err(McpFailureCode::InvalidDefinition),
    }
}

fn digest(value: &str) -> Result<[u8; 32], McpFailureCode> {
    let invalid = McpFailureCode::InvalidDefinition;
    if value.len() != 64 || !value.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')) {
        return Err(invalid);
    }
    let mut digest = [0; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let pair = std::str::from_utf8(pair).map_err(|_| invalid)?;
        digest[index] = u8::from_str_radix(pair, 16).map_err(|_| invalid)?;
    }
    Ok(digest)
}

fn reviewed_tools(mut tools: Vec<String>) -> Result<Vec<String>, McpFailureCode> {
    if tools.is_empty()
        || tools.len() > 128
        || tools.iter().any(|name| {
            name.is_empty()
                || name.len() > 64
                || !name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
                || !name.bytes().any(|b| b.is_ascii_alphanumeric())
        })
    {
        return Err(McpFailureCode::InvalidDefinition);
    }
    tools.sort_unstable();
    tools.dedup();
    Ok(tools)
}

#[cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod gnu;
#[cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub use gnu::ReviewedProjectStdio;
