//! Model transport stream observation, with host converter integration tests.
pub(crate) use model_transport::stream_observer::*;

#[cfg(test)]
#[path = "stream_observer_tests.rs"]
pub(crate) mod tests;
