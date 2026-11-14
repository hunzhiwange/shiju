//! Hermes' two native memory files on the authorized executor.
//! Authorization and application-root resolution must precede this boundary.
use crate::workspace_paths::{PathError, WorkspaceArea, WorkspaceDirectory};
use std::path::Path;
use vw_api_types::hermes_memory::{HermesMemoryKind, HermesMemoryResponse};

/// Restricts the shared descriptor-relative IO boundary to Hermes memories.
#[derive(Debug)]
pub struct HermesMemory {
    directory: Option<WorkspaceDirectory>,
}

impl HermesMemory {
    /// Pass the trusted executor HOME and resolved Hermes application root.
    /// `create` must only be true after write authorization. Reads never need it.
    pub fn open(
        authorized_host: &str,
        executor_host: &str,
        home: &Path,
        resolved_root: &Path,
        create: bool,
    ) -> Result<Self, PathError> {
        Ok(Self {
            directory: WorkspaceDirectory::open(
                authorized_host,
                executor_host,
                home,
                resolved_root,
                WorkspaceArea::HermesMemories,
                create,
            )?,
        })
    }

    /// Missing files have empty content; errors are preserved, never hidden.
    pub fn read_memory(&self, kind: HermesMemoryKind) -> Result<HermesMemoryResponse, PathError> {
        let content = match &self.directory {
            Some(directory) => directory.read(filename(kind))?,
            None => None,
        };
        Ok(HermesMemoryResponse {
            kind,
            exists: content.is_some(),
            content: content.unwrap_or_default(),
        })
    }

    /// Atomically replaces only the selected file, independent of enable flags
    /// and budgets. Open with `create = true` after write authorization first.
    pub fn write_memory(&self, kind: HermesMemoryKind, content: &str) -> Result<(), PathError> {
        self.directory.as_ref().ok_or(PathError::Io)?.write(filename(kind), content)
    }
}

fn filename(kind: HermesMemoryKind) -> &'static str {
    match kind {
        HermesMemoryKind::Memory => "MEMORY.md",
        HermesMemoryKind::User => "USER.md",
    }
}
