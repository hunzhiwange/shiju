//! 原生视图边界的 Wry 坐标转换测试。

use webview_toolbar::ViewRegion;
use crate::bounds::to_wry_rect;

#[test]
fn view_region_converts_to_logical_wry_bounds() {
    let region = ViewRegion { x: 12, y: 46, width: 876, height: 554 };

    assert_eq!(
        to_wry_rect(region),
        wry::Rect {
            position: wry::dpi::LogicalPosition::new(12, 46).into(),
            size: wry::dpi::LogicalSize::new(876, 554).into(),
        }
    );
}
