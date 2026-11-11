//! In-memory document registry owned by the authenticated Office transport.
//!
//! The transport supplies a trusted control-session owner and host monotonic time,
//! calls disconnect on close/reload/socket loss, and replaces identity on Save As.
//! An entry is neither a pairing credential nor permission to read/write a document.
//! Callers must also recheck ControlHost policy before dispatch and after receiving
//! results. Never dispatch by title or accept a result against a stale handle.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use uuid::Uuid;
use vw_api_types::computer_control::{
    ComputerControlAction, ComputerControlTarget, ControlDocumentId, ControlErrorCode,
    ControlSessionId, ExcelCommand, WordCommand,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficeSurface {
    Excel,
    Word,
}

/// Exact closed command discovery, rather than a broad read/write promise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficeCapability {
    ExcelReadRange,
    ExcelWriteRange,
    WordReadDocument,
    WordFindText,
    WordReplaceText,
    WordInsertText,
}

impl OfficeCapability {
    fn surface(self) -> OfficeSurface {
        match self {
            Self::ExcelReadRange | Self::ExcelWriteRange => OfficeSurface::Excel,
            Self::WordReadDocument
            | Self::WordFindText
            | Self::WordReplaceText
            | Self::WordInsertText => OfficeSurface::Word,
        }
    }

    fn for_action(action: &ComputerControlAction) -> Result<Self, ControlErrorCode> {
        match action {
            ComputerControlAction::Excel { command: ExcelCommand::ReadRange { .. } } => {
                Ok(Self::ExcelReadRange)
            }
            ComputerControlAction::Excel { command: ExcelCommand::WriteRange { .. } } => {
                Ok(Self::ExcelWriteRange)
            }
            ComputerControlAction::Word { command: WordCommand::ReadDocument } => {
                Ok(Self::WordReadDocument)
            }
            ComputerControlAction::Word { command: WordCommand::FindText { .. } } => {
                Ok(Self::WordFindText)
            }
            ComputerControlAction::Word { command: WordCommand::ReplaceText { .. } } => {
                Ok(Self::WordReplaceText)
            }
            ComputerControlAction::Word { command: WordCommand::InsertText { .. } } => {
                Ok(Self::WordInsertText)
            }
            _ => Err(ControlErrorCode::Unsupported),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficeHandle {
    pub surface: OfficeSurface,
    pub document_id: ControlDocumentId,
    /// Host-minted connection epoch; new even after registry recreation/reload.
    pub generation: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficeSession {
    pub handle: OfficeHandle,
    /// Display only. No paths, document contents or credentials are required.
    pub title: String,
    pub capabilities: Vec<OfficeCapability>,
}

struct Entry {
    owner: ControlSessionId,
    session: OfficeSession,
    last_heartbeat: Instant,
}

pub struct OfficeSessions {
    timeout: Duration,
    entries: HashMap<Uuid, Entry>,
}

impl OfficeSessions {
    pub fn new(timeout: Duration) -> Result<Self, ControlErrorCode> {
        if timeout.is_zero() {
            return Err(ControlErrorCode::InvalidRequest);
        }
        Ok(Self { timeout, entries: HashMap::new() })
    }

    /// Only called after transport pairing. Re-registration of the same owner,
    /// surface and document invalidates its previous connection immediately.
    pub fn register(
        &mut self,
        owner: ControlSessionId,
        surface: OfficeSurface,
        document_id: ControlDocumentId,
        title: String,
        capabilities: Vec<OfficeCapability>,
        now: Instant,
    ) -> Result<OfficeHandle, ControlErrorCode> {
        Self::validate(&owner, surface, &document_id, &title, &capabilities)?;
        self.expire(now);
        self.entries.retain(|_, entry| {
            entry.owner != owner
                || entry.session.handle.surface != surface
                || entry.session.handle.document_id != document_id
        });
        let handle = OfficeHandle { surface, document_id, generation: Uuid::new_v4() };
        self.entries.insert(
            handle.generation,
            Entry {
                owner,
                session: OfficeSession { handle: handle.clone(), title, capabilities },
                last_heartbeat: now,
            },
        );
        Ok(handle)
    }

    /// A changed document identity cannot keep its old connection epoch.
    pub fn replace_document(
        &mut self,
        owner: &ControlSessionId,
        handle: &OfficeHandle,
        document_id: ControlDocumentId,
        title: String,
        now: Instant,
    ) -> Result<OfficeHandle, ControlErrorCode> {
        self.expire(now);
        let capabilities = self.entry(owner, handle)?.session.capabilities.clone();
        // Fail closed even if the replacement metadata turns out to be invalid.
        self.entries.remove(&handle.generation);
        self.register(owner.clone(), handle.surface, document_id, title, capabilities, now)
    }

    pub fn heartbeat(
        &mut self,
        owner: &ControlSessionId,
        handle: &OfficeHandle,
        now: Instant,
    ) -> Result<(), ControlErrorCode> {
        self.expire(now);
        self.entry(owner, handle)?;
        let entry =
            self.entries.get_mut(&handle.generation).ok_or(ControlErrorCode::Disconnected)?;
        // A caller cannot move the host clock backward to revive a connection.
        if now < entry.last_heartbeat {
            return Err(ControlErrorCode::InvalidRequest);
        }
        entry.last_heartbeat = now;
        Ok(())
    }

    pub fn disconnect(
        &mut self,
        owner: &ControlSessionId,
        handle: &OfficeHandle,
    ) -> Result<(), ControlErrorCode> {
        self.entry(owner, handle)?;
        self.entries.remove(&handle.generation);
        Ok(())
    }

    /// Called when a controlling session stops or its pairing is revoked.
    pub fn disconnect_owner(&mut self, owner: &ControlSessionId) {
        self.entries.retain(|_, entry| &entry.owner != owner);
    }

    pub fn discover(&mut self, owner: &ControlSessionId, now: Instant) -> Vec<OfficeSession> {
        self.expire(now);
        self.entries
            .values()
            .filter(|entry| &entry.owner == owner)
            .map(|entry| entry.session.clone())
            .collect()
    }

    /// Recheck before sending a command and before accepting its result. The
    /// returned snapshot must never be cached as execution authority.
    pub fn authorize(
        &mut self,
        owner: &ControlSessionId,
        handle: &OfficeHandle,
        target: &ComputerControlTarget,
        action: &ComputerControlAction,
        now: Instant,
    ) -> Result<OfficeSession, ControlErrorCode> {
        self.expire(now);
        let entry = self.entry(owner, handle)?;
        let matches = match (handle.surface, target) {
            (OfficeSurface::Excel, ComputerControlTarget::ExcelWorkbook { document_id })
            | (OfficeSurface::Word, ComputerControlTarget::WordDocument { document_id }) => {
                document_id == &handle.document_id
            }
            _ => false,
        };
        if !matches {
            return Err(ControlErrorCode::TargetChanged);
        }
        let capability = OfficeCapability::for_action(action)?;
        if !entry.session.capabilities.contains(&capability) {
            return Err(ControlErrorCode::Unsupported);
        }
        Ok(entry.session.clone())
    }

    fn entry(
        &self,
        owner: &ControlSessionId,
        handle: &OfficeHandle,
    ) -> Result<&Entry, ControlErrorCode> {
        let entry = self.entries.get(&handle.generation).ok_or(ControlErrorCode::Disconnected)?;
        if &entry.owner != owner {
            return Err(ControlErrorCode::PermissionDenied);
        }
        if &entry.session.handle != handle {
            return Err(ControlErrorCode::TargetChanged);
        }
        Ok(entry)
    }

    fn expire(&mut self, now: Instant) {
        self.entries.retain(|_, entry| {
            now.checked_duration_since(entry.last_heartbeat)
                .is_some_and(|elapsed| elapsed < self.timeout)
        });
    }

    fn validate(
        owner: &ControlSessionId,
        surface: OfficeSurface,
        document_id: &ControlDocumentId,
        title: &str,
        capabilities: &[OfficeCapability],
    ) -> Result<(), ControlErrorCode> {
        if owner.0.trim().is_empty()
            || owner.0.len() > 256
            || document_id.0.trim().is_empty()
            || document_id.0.len() > 256
            || title.len() > 512
            || title.chars().any(char::is_control)
            || capabilities.len() > 6
            || capabilities
                .iter()
                .enumerate()
                .any(|(index, cap)| cap.surface() != surface || capabilities[..index].contains(cap))
        {
            return Err(ControlErrorCode::InvalidRequest);
        }
        Ok(())
    }
}
