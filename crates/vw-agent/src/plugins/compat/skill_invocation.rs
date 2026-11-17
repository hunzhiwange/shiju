//! Skill identity methods belong to agent-skills; mention parsing to plugin-install.
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use plugin_install::skill_invocation::reference_text;
