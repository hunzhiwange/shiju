//! Versioned ShiJu session mount extension (not an ACP standard field).
//!
//! These types validate wire shapes only. Execution-side directory canonicalization,
//! tenant/project/session binding and skill audit remain mandatory before activation.
use std::{
    collections::HashSet,
    path::{Component, Path},
};

use agent_client_protocol::{ExtRequest, McpServer, Meta, SessionId};
use serde::{Deserialize, Serialize, de::Error};
use vw_api_types::mcp_management::{McpCatalogScope, McpRecordRef};

pub const SESSION_MOUNTS_META_KEY: &str = "shiju.mounts";
pub const SESSION_SKILL_MOUNTS_META_KEY: &str = "shiju.skillMounts";
/// Pass this to SDK ext_method; the SDK adds `_` on the wire.
pub const SET_SESSION_MOUNTS_METHOD: &str = "shiju/session/set_mounts";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SessionMountError {
    #[error("unsupported session mounts version")]
    UnsupportedVersion,
    #[error("invalid skill mount name")]
    InvalidName,
    #[error("skill mount path must be an absolute directory path without traversal")]
    InvalidPath,
    #[error("duplicate mount name")]
    DuplicateName,
    #[error("session mounts revision conflict")]
    RevisionConflict,
    #[error("session mounts revision exhausted")]
    RevisionExhausted,
    #[error("peer has not advertised session mount updates")]
    UnsupportedUpdate,
    #[error("unsupported MCP transport")]
    UnsupportedMcpTransport,
}

fn validate_name(name: &str) -> Result<(), SessionMountError> {
    // Preserve existing skill identifiers (including Unicode and underscores),
    // following skills/cli.rs's rejection of traversal and path separators.
    if name.trim().is_empty()
        || name == "."
        || name.contains("..")
        || name.contains(['/', '\\'])
        || name.chars().any(char::is_control)
    {
        return Err(SessionMountError::InvalidName);
    }
    Ok(())
}

fn validate_names<'a>(names: impl IntoIterator<Item = &'a str>) -> Result<(), SessionMountError> {
    let mut seen = HashSet::new();
    for name in names {
        validate_name(name)?;
        if !seen.insert(name) {
            return Err(SessionMountError::DuplicateName);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SkillMountWire")]
pub struct SessionSkillMount {
    pub name: String,
    pub path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillMountWire {
    name: String,
    path: String,
}

impl TryFrom<SkillMountWire> for SessionSkillMount {
    type Error = SessionMountError;
    fn try_from(wire: SkillMountWire) -> Result<Self, Self::Error> {
        let mount = Self { name: wire.name, path: wire.path };
        mount.validate()?;
        Ok(mount)
    }
}

impl SessionSkillMount {
    pub fn validate(&self) -> Result<(), SessionMountError> {
        validate_name(&self.name)?;
        let path = Path::new(&self.path);
        if !path.is_absolute()
            || self.path.chars().any(char::is_control)
            || path.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir))
            || self.path.split(['/', '\\']).any(|c| c == "." || c == "..")
        {
            return Err(SessionMountError::InvalidPath);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SelectionWire")]
pub struct SessionMountSelection {
    pub version: u32,
    pub skills: Vec<SessionSkillMount>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectionWire {
    version: u32,
    skills: Vec<SessionSkillMount>,
}

impl TryFrom<SelectionWire> for SessionMountSelection {
    type Error = SessionMountError;
    fn try_from(wire: SelectionWire) -> Result<Self, Self::Error> {
        validate_version(wire.version)?;
        validate_skills(&wire.skills)?;
        Ok(Self { version: wire.version, skills: wire.skills })
    }
}

fn validate_version(version: u32) -> Result<(), SessionMountError> {
    if version != 1 {
        return Err(SessionMountError::UnsupportedVersion);
    }
    Ok(())
}

fn validate_skills(skills: &[SessionSkillMount]) -> Result<(), SessionMountError> {
    for skill in skills {
        skill.validate()?;
    }
    validate_names(skills.iter().map(|s| s.name.as_str()))
}

impl SessionMountSelection {
    /// Missing key is distinct from an explicitly empty skill collection; null is invalid.
    pub fn from_meta(meta: Option<&Meta>) -> Result<Option<Self>, serde_json::Error> {
        meta.and_then(|m| m.get(SESSION_SKILL_MOUNTS_META_KEY))
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
    }

    /// For new sessions pass None; for load pass the saved directory descriptions.
    /// MCP credentials are deliberately absent from this restoration API.
    pub fn resolve(
        selection: Option<&Self>,
        saved: Option<&[SessionSkillMount]>,
    ) -> Vec<SessionSkillMount> {
        match selection {
            Some(selection) => selection.skills.clone(),
            None => saved.unwrap_or_default().to_vec(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionMountRevision(pub u64);

impl SessionMountRevision {
    /// Validate before preparing an atomic replacement; publish only after it succeeds.
    pub fn next(self, expected: Self) -> Result<Self, SessionMountError> {
        if self != expected {
            return Err(SessionMountError::RevisionConflict);
        }
        self.0.checked_add(1).map(Self).ok_or(SessionMountError::RevisionExhausted)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "CapabilitiesWire", rename_all = "camelCase")]
pub struct SessionMountCapabilities {
    pub version: u32,
    pub skills: bool,
    pub set_mounts: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CapabilitiesWire {
    version: u32,
    skills: bool,
    set_mounts: bool,
}

impl TryFrom<CapabilitiesWire> for SessionMountCapabilities {
    type Error = SessionMountError;
    fn try_from(wire: CapabilitiesWire) -> Result<Self, Self::Error> {
        validate_version(wire.version)?;
        Ok(Self { version: wire.version, skills: wire.skills, set_mounts: wire.set_mounts })
    }
}

impl Default for SessionMountCapabilities {
    fn default() -> Self {
        Self { version: 1, skills: false, set_mounts: false }
    }
}

impl SessionMountCapabilities {
    pub fn from_meta(meta: Option<&Meta>) -> Result<Option<Self>, serde_json::Error> {
        meta.and_then(|m| m.get(SESSION_MOUNTS_META_KEY))
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
    }
}

/// A managed server selected by exact revision. The backend authorizes the
/// requested scope against the session before resolving its connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpCatalogMount {
    pub scope: McpCatalogScope,
    pub record: McpRecordRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "SetMountsWire", rename_all = "camelCase")]
pub struct SetSessionMountsRequest {
    pub session_id: SessionId,
    pub expected_revision: SessionMountRevision,
    pub skills: Vec<SessionSkillMount>,
    pub mcp_servers: Vec<McpServer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_catalog: Vec<McpCatalogMount>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SetMountsWire {
    session_id: SessionId,
    expected_revision: SessionMountRevision,
    skills: Vec<SessionSkillMount>,
    mcp_servers: Vec<McpServer>,
    #[serde(default)]
    mcp_catalog: Vec<McpCatalogMount>,
}

fn mcp_name(server: &McpServer) -> Result<&str, SessionMountError> {
    match server {
        McpServer::Stdio(s) => Ok(&s.name),
        McpServer::Http(s) => Ok(&s.name),
        McpServer::Sse(s) => Ok(&s.name),
        _ => Err(SessionMountError::UnsupportedMcpTransport),
    }
}

impl TryFrom<SetMountsWire> for SetSessionMountsRequest {
    type Error = SessionMountError;
    fn try_from(wire: SetMountsWire) -> Result<Self, Self::Error> {
        validate_skills(&wire.skills)?;
        let mcp_names = wire.mcp_servers.iter().map(mcp_name).collect::<Result<Vec<_>, _>>()?;
        validate_names(mcp_names)?;
        Ok(Self {
            session_id: wire.session_id,
            expected_revision: wire.expected_revision,
            skills: wire.skills,
            mcp_servers: wire.mcp_servers,
            mcp_catalog: wire.mcp_catalog,
        })
    }
}

impl SetSessionMountsRequest {
    /// The sole outgoing builder requires explicit capability negotiation.
    /// Do not call this for third-party agents without their verified contract.
    pub fn to_ext_request(
        &self,
        capabilities: Option<&SessionMountCapabilities>,
    ) -> Result<ExtRequest, serde_json::Error> {
        if !capabilities.is_some_and(|c| c.version == 1 && c.set_mounts) {
            return Err(serde_json::Error::custom(SessionMountError::UnsupportedUpdate));
        }
        // Re-validate because callers may construct public structs directly.
        let value = serde_json::to_value(self)?;
        let _: Self = serde_json::from_value(value)?;
        Ok(ExtRequest::new(
            SET_SESSION_MOUNTS_METHOD,
            serde_json::value::to_raw_value(self)?.into(),
        ))
    }
}

/// Also used in new/load response _meta["shiju.mounts"]. Contains names only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ResponseWire", rename_all = "camelCase")]
pub struct SetSessionMountsResponse {
    pub revision: SessionMountRevision,
    pub skills: Vec<String>,
    pub mcp_servers: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResponseWire {
    revision: SessionMountRevision,
    skills: Vec<String>,
    mcp_servers: Vec<String>,
}

impl TryFrom<ResponseWire> for SetSessionMountsResponse {
    type Error = SessionMountError;
    fn try_from(wire: ResponseWire) -> Result<Self, Self::Error> {
        validate_names(wire.skills.iter().map(String::as_str))?;
        validate_names(wire.mcp_servers.iter().map(String::as_str))?;
        Ok(Self { revision: wire.revision, skills: wire.skills, mcp_servers: wire.mcp_servers })
    }
}

#[cfg(test)]
#[path = "session_mounts_tests.rs"]
mod tests;
