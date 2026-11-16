//! Await domain owners inline; projection failures never undo a committed receive.
#![cfg(target_os = "macos")]
pub(super) use gateway_app_routing::server::receive_projection::inject;
