//! Receiver consumes the data-management projection; it never forwards remote SQL.
pub use agent_app_services::shared_snapshot_import::{
    SnapshotImport, apply_shared_snapshot, preview_shared_snapshot,
};
