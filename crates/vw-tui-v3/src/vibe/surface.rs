//! Minimal public surface activated by the v3 migration shell.

/// Constructible root surface for the VibeWindow TUI v3.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct V3Surface;

impl V3Surface {
    /// Creates an empty v3 surface.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}
