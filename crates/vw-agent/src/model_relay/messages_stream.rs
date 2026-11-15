//! Protocol implementation lives in model-protocol.
pub(crate) use model_protocol::messages_stream::*;

#[cfg(test)]
#[path = "messages_stream_tests.rs"]
mod tests;
