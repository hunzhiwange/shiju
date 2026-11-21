//! Publish the selected skills in the reviewed Stakpak runtime's private HOME.
use super::super::{Error, host::SkillLease, model_source::profile::AcpLaunchProfile};

pub(in crate::acp) fn publish(
    profile: &AcpLaunchProfile,
    view: Option<&impl SkillLease>,
) -> Result<(), Error> {
    publish_at(profile.directory(), view)
}

fn publish_at(home: &std::path::Path, view: Option<&impl SkillLease>) -> Result<(), Error> {
    if let Some(view) = view {
        view.snapshot(&home.join(".stakpak/skills"))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "stakpak_tests.rs"]
mod tests;
