//! Guarded publication for native MCP adapters on the authorized execution host.
use crate::providers::live::{
    commit::{PendingTarget, ProviderTransactions},
    patch_json::{JsonPatch, object_value},
    stage::{FileChange, FilePatch},
    write_guard::{AppWriteGuard, FilePrivacy, WriteError},
};
use serde_json::{Map, Value};
use std::ffi::OsStr;
use vw_api_types::id::TenantScope;

/// A direct child selected by adapter code under the same guard as provider writes.
/// The executor must acquire the guard from its authorized app/profile paths.
pub struct NativeFile<'a> {
    guard: &'a mut AppWriteGuard,
    name: &'a OsStr,
}

impl<'a> NativeFile<'a> {
    pub fn new(guard: &'a mut AppWriteGuard, name: &'a OsStr) -> Self {
        Self { guard, name }
    }

    /// The snapshot may contain credentials. Keep it on the host.
    pub fn read(&self) -> Result<Option<Vec<u8>>, WriteError> {
        self.guard.read(self.name)
    }

    /// The pure patch operates on the expected file snapshot. The executor's
    /// preflight rechecks saved ownership and catalog revisions before publication.
    pub fn commit(
        self,
        transactions: &ProviderTransactions,
        tenant: &TenantScope,
        expected: Option<&[u8]>,
        patch: &dyn FilePatch,
        preflight: impl Fn() -> Result<(), WriteError>,
    ) -> Result<(), WriteError> {
        let patch = SnapshotPatch { expected, patch };
        transactions.commit_checked(
            tenant,
            self.guard,
            &[FileChange { name: self.name, privacy: FilePrivacy::Private, patch: &patch }],
            PendingTarget::default(),
            preflight,
        )
    }
}

struct SnapshotPatch<'a> {
    expected: Option<&'a [u8]>,
    patch: &'a dyn FilePatch,
}

impl FilePatch for SnapshotPatch<'_> {
    fn apply(&self, before: Option<&[u8]>) -> Result<Option<Vec<u8>>, WriteError> {
        // Publication can call this again if a noncooperating writer changed the file.
        if before != self.expected {
            return Err(WriteError::Conflict);
        }
        self.patch.apply(before)
    }
}

/// The adapter selects one server in its fixed JSON/JSONC server map.
pub struct JsonServerPatch<'a> {
    pub section: &'a str,
    pub id: &'a str,
    pub edit: JsonServerEdit<'a>,
}

pub enum JsonServerEdit<'a> {
    /// An existing entry with the same ID conflicts with a new binding.
    Insert(&'a Map<String, Value>),
    /// Use only for a binding whose ownership the executor has verified.
    Update { set: &'a Map<String, Value>, remove: &'a [&'a str] },
    /// Use only when revoking a binding whose ownership the executor has verified.
    Remove,
}

impl FilePatch for JsonServerPatch<'_> {
    fn apply(&self, before: Option<&[u8]>) -> Result<Option<Vec<u8>>, WriteError> {
        let root =
            object_value(before.unwrap_or(b"{}")).map_err(|_| WriteError::InvalidResource)?;
        let current = match root.get(self.section) {
            Some(section) => section.as_object().ok_or(WriteError::InvalidResource)?.get(self.id),
            None => None,
        };
        let path = vec![self.section.to_owned(), self.id.to_owned()];
        let mut patch = JsonPatch::default();
        match self.edit {
            JsonServerEdit::Insert(fields) => {
                if current.is_some() {
                    return Err(WriteError::Conflict);
                }
                patch.set.push((path, Value::Object(fields.clone())));
            }
            JsonServerEdit::Update { set, remove } => {
                let server = current.ok_or(WriteError::Conflict)?;
                if !server.is_object() {
                    return Err(WriteError::InvalidResource);
                }
                let field_path = |field: &str| {
                    let mut path = path.clone();
                    path.push(field.to_owned());
                    path
                };
                patch.set.extend(set.iter().map(|(key, value)| (field_path(key), value.clone())));
                patch.remove.extend(remove.iter().map(|key| field_path(key)));
            }
            JsonServerEdit::Remove => {
                if current.is_none() {
                    return Ok(before.map(<[u8]>::to_vec));
                }
                patch.remove.push(path);
            }
        }
        patch.apply(before).map(Some).map_err(|_| WriteError::InvalidResource)
    }
}
