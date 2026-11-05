//! Claude SDK local plugin manifest over the owned session skill view.
pub(super) fn prepare(
    view: &crate::skills::session_mounts::SessionSkillView,
) -> Result<(), super::Error> {
    view.validate().map_err(super::super::to_api_error)?;
    let manifest = view.root().parent().expect("owned skill view").join(".claude-plugin");
    std::fs::create_dir_all(&manifest).map_err(super::super::to_api_error)?;
    std::fs::write(
        manifest.join("plugin.json"),
        br#"{"name":"shiju-session-skills","version":"1.0.0","skills":"./skills/"}"#,
    )
    .map_err(super::super::to_api_error)?;
    Ok(())
}
#[cfg(test)]
#[path = "claude_tests.rs"]
mod tests;
