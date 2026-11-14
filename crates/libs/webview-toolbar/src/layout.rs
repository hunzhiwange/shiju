//! 独立窗口工具栏与内容区域划分。

pub(crate) const TOOLBAR_REGION_HEIGHT: u32 = 46;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn window_regions(
    width: u32,
    height: u32,
    independent: bool,
) -> (Option<ViewRegion>, ViewRegion) {
    if !independent {
        return (None, ViewRegion { x: 0, y: 0, width, height });
    }

    let toolbar_height = TOOLBAR_REGION_HEIGHT.min(height);
    (
        Some(ViewRegion { x: 0, y: 0, width, height: toolbar_height }),
        ViewRegion {
            x: 0,
            y: toolbar_height,
            width,
            height: height.saturating_sub(toolbar_height),
        },
    )
}
