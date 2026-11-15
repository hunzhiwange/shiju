//! Minimal public surface activated by the v3 migration shell.
//! v3 迁移壳当前启用的最小公开表面。

/// Constructible root surface for the VibeWindow TUI v3.
/// 可直接构造的 VibeWindow TUI v3 根表面。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct V3Surface;

impl V3Surface {
    /// Creates an empty v3 surface.
    /// 创建一个不持有运行时资源的空 v3 表面。
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}
