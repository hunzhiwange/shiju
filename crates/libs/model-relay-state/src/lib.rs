//! Transient, owner-scoped model relay identity and reference state.
//! These trusted values are not serialized or reconstructed from HTTP headers.
mod identity;
pub mod reference_scope;

pub use identity::RelayIdentity;
pub use reference_scope::{ApplicationReferenceScope, ReferenceScope, References};
