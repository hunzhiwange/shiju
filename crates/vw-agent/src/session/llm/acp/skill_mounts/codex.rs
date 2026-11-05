//! Native roots use the reviewed codex-acp bridge, never workspace writable roots.
//! The ACP client requires its versioned capability before creating a mounted session.
pub(super) fn validate() -> Result<(), super::Error> {
    Ok(())
}
#[cfg(test)]
#[path = "codex_tests.rs"]
mod tests;
