//! Forward-only fixture import for cross-domain model-routing host contracts.
#[path = "../../../../libs/desktop-settings-providers/src/model_test_fixture.rs"]
mod shared;
pub(in crate::app::components) use shared::{model, provider};
