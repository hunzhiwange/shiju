//! Reconcile only the selected ordinary prompt; disabled entries never own live files.
use super::{activation::ActivationError, listing::PromptEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshWarning {
    Missing,
    Empty,
    InvalidUtf8,
}

/// Preserve saved content for missing, blank or undecodable external files.
/// Call under the target lock after authorization and ownership validation.
pub fn refresh(
    entries: &mut [PromptEntry], active: Option<&str>, live: Option<&[u8]>,
) -> Result<Option<RefreshWarning>, ActivationError> {
    let Some(active) = active else { return Ok(None); };
    let entry = entries.iter_mut().find(|entry| entry.id == active)
        .ok_or(ActivationError::InvalidSelection)?;
    let Some(live) = live else { return Ok(Some(RefreshWarning::Missing)); };
    let Ok(text) = std::str::from_utf8(live) else {
        return Ok(Some(RefreshWarning::InvalidUtf8));
    };
    if text.trim().is_empty() { return Ok(Some(RefreshWarning::Empty)); }
    entry.content = text.to_owned();
    Ok(None)
}
