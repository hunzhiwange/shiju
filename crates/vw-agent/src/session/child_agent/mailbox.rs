//! Shared collaboration message contracts.
pub(crate) use session_port::child_agent::{TeamMessageKind, TeamMessage, TeamActivity};

#[cfg(test)]
#[path = "mailbox_tests.rs"]
mod tests;
