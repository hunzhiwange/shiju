//! Additional authorization for existing scoped supplier writes.
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use gateway_app_routing::server::record_resync::{configure, configure_with_context, provider_headers};
