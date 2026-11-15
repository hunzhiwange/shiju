//! Concrete host binding for session integration tests.
pub(crate) use gateway_voice::voice_handoff::{Binding, HandoffCall};
pub(crate) type Handoffs = gateway_voice::voice_handoff::Handoffs<super::voice::Host>;
