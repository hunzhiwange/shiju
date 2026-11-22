use super::*;
use acp_types::AcpMcpServer;
use std::path::{Path, PathBuf};

/// Owned proof for one project service. Check it again before discovery and execution.
pub struct ReviewedProjectStdio {
    project: ProjectScope,
    selection: StoredSelection,
    executable: PathBuf,
    args: Vec<String>,
    review: StdioReview,
}

impl ReviewedProjectStdio {
    pub fn executable(&self) -> &Path {
        &self.executable
    }
    pub fn args(&self) -> &[String] {
        &self.args
    }
    pub fn sha256(&self) -> [u8; 32] {
        self.review.sha256
    }
    pub fn allowed_tools(&self) -> &[String] {
        &self.review.allowed_tools
    }

    pub fn is_current(
        &self,
        store: &McpCatalogStore,
        owner: &ProjectScope,
    ) -> Result<bool, McpFailureCode> {
        if owner != &self.project {
            return Ok(false);
        }
        let current = match store.linux_stdio_selection(owner, &self.selection.record.id.0) {
            Ok(current) => current,
            Err(McpFailureCode::NotFound | McpFailureCode::PermissionDenied) => return Ok(false),
            Err(error) => return Err(error),
        };
        if current != self.selection || current.disabled {
            return Ok(false);
        }
        match store.connection_for_revision(&current.scope, &current.record) {
            Ok(_) => Ok(true),
            Err(McpFailureCode::NotFound | McpFailureCode::StaleRevision) => Ok(false),
            Err(error) => Err(error),
        }
    }
}

impl McpCatalogStore {
    /// Read from the exact trusted project of the ACP launch. The request can only
    /// narrow the already approved catalog selection to an exact connection match.
    pub fn reviewed_project_stdio(
        &self,
        project: &ProjectScope,
        requested: &[AcpMcpServer],
    ) -> Result<ReviewedProjectStdio, McpFailureCode> {
        let [AcpMcpServer::Stdio(requested)] = requested else {
            return Err(McpFailureCode::PermissionDenied);
        };
        let selection = self.linux_stdio_selection(project, &requested.name)?;
        if selection.disabled {
            return Err(McpFailureCode::PermissionDenied);
        }
        let connection = self.connection_for_revision(&selection.scope, &selection.record)?;
        let (command, args) = fixed_command(&connection)?;
        if requested.command.as_path() != command
            || requested.args.as_slice() != args
            || !requested.env.is_empty()
        {
            return Err(McpFailureCode::PermissionDenied);
        }
        let review = selection.linux_stdio.clone().ok_or(McpFailureCode::PermissionDenied)?;
        let proof = ReviewedProjectStdio {
            project: project.clone(),
            selection,
            executable: command.to_owned(),
            args: args.to_vec(),
            review,
        };
        if !proof.is_current(self, project)? {
            return Err(McpFailureCode::PermissionDenied);
        }
        Ok(proof)
    }
}

#[cfg(test)]
#[path = "gnu_tests.rs"]
mod tests;
