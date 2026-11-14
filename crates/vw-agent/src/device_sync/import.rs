//! Receiver consumes the data-management projection; it never forwards remote SQL.
pub use crate::agent_apps::shared_snapshot_import::{
    SnapshotImport, apply_shared_snapshot, preview_shared_snapshot,
};
