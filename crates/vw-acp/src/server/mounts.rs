//! Mount extension parsing and response metadata; no filesystem or MCP side effects.
use agent_client_protocol::{Error, McpServer, Meta, Result};

use crate::session_mounts::{
    SESSION_MOUNTS_META_KEY, SessionMountCapabilities, SessionMountRevision, SessionMountSelection,
    SetSessionMountsResponse,
};

/// Complete new/load input. `None` restores saved skills on load; `Some([])` clears them.
#[derive(Debug, Clone)]
pub struct BackendMountRequest {
    pub skills: Option<SessionMountSelection>,
    /// Standard ACP MCP list, without conversion or credential persistence.
    pub mcp_servers: Vec<McpServer>,
}

impl BackendMountRequest {
    pub(super) fn parse(meta: Option<&Meta>, mcp_servers: Vec<McpServer>) -> Result<Self> {
        let skills = SessionMountSelection::from_meta(meta).map_err(|_| {
            Error::invalid_params()
                .data(serde_json::json!({"reason":"invalid shiju.skillMounts metadata"}))
        })?;
        Ok(Self { skills, mcp_servers })
    }

    pub(super) fn reject_unsupported_skills(&self) -> std::result::Result<(), super::BackendError> {
        if self.skills.as_ref().is_some_and(|selection| !selection.skills.is_empty()) {
            return Err(super::BackendError::InvalidRequest(
                "skill directory mounts are not supported by this backend".into(),
            ));
        }
        Ok(())
    }

    pub(super) fn legacy_response(
        &self,
    ) -> std::result::Result<SetSessionMountsResponse, super::BackendError> {
        let mut names = Vec::new();
        for server in &self.mcp_servers {
            let name = match server {
                McpServer::Stdio(server) => &server.name,
                McpServer::Http(server) => &server.name,
                McpServer::Sse(server) => &server.name,
                _ => {
                    return Err(super::BackendError::InvalidRequest(
                        "unsupported MCP transport".into(),
                    ));
                }
            };
            names.push(name.clone());
        }
        Ok(SetSessionMountsResponse {
            revision: SessionMountRevision(0),
            skills: Vec::new(),
            mcp_servers: names,
        })
    }
}

pub(super) fn response_meta(response: &SetSessionMountsResponse) -> Result<Meta> {
    let mut meta = Meta::new();
    meta.insert(
        SESSION_MOUNTS_META_KEY.into(),
        serde_json::to_value(response).map_err(|_| Error::internal_error())?,
    );
    Ok(meta)
}

pub(super) fn capabilities_meta() -> Meta {
    let mut meta = Meta::new();
    // Native skill discovery and updates are wired by subsequent tasks.
    meta.insert(
        SESSION_MOUNTS_META_KEY.into(),
        serde_json::json!(SessionMountCapabilities::default()),
    );
    meta
}
