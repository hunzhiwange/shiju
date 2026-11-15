//! Office document identity and lifecycle. Registration does not grant control permission.
pub use office_control_sessions as sessions;

#[cfg(not(target_arch = "wasm32"))]
pub mod transport;
