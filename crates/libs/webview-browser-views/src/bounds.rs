//! Wry 坐标转换，保留在原生视图边界。

use webview_toolbar::ViewRegion;

pub fn to_wry_rect(region: ViewRegion) -> wry::Rect {
    wry::Rect {
        position: wry::dpi::LogicalPosition::new(region.x, region.y).into(),
        size: wry::dpi::LogicalSize::new(region.width, region.height).into(),
    }
}
