//! Publish the selected view at Qoder's request-private user skill directory.
use super::super::host::SkillLease;
use std::path::Path;

pub(in crate::acp) fn publish(
    directory: &Path,
    view: Option<&impl SkillLease>,
) -> Result<(), super::Error> {
    let Some(view) = view else {
        return Ok(());
    };
    view.validate()?;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    std::os::unix::fs::symlink(view.root(), directory.join("skills"))
        .map_err(super::super::to_api_error)?;
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        let _ = directory;
        return Err(super::super::acp_option_error(
            "Qoder skill mounting requires the reviewed Darwin arm64 runtime",
        ));
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    Ok(())
}

#[cfg(test)]
#[path = "qoder_tests.rs"]
mod tests;
