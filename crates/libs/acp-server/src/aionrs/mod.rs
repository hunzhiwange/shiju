//! Isolated JSON Stream adapter for aionrs revision 8e61a903.
mod backend;
#[cfg(unix)]
mod mcp;
mod process;
mod protocol;
pub use backend::AionrsBackend;
#[cfg(test)]
mod protocol_tests;
