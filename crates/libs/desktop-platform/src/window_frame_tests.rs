//! 窗口恢复的坐标边界用例；本轮仅编写，未执行。
use objc2_foundation::{NSPoint, NSRect, NSSize};

use super::constrain_saved_frame;

fn rect(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
}

#[test]
fn desktop_integration_25_preserves_expected_behavior() {
    let visible = rect(-1920.0, 40.0, 1920.0, 1040.0);
    let saved = rect(-1800.0, 100.0, 1000.0, 700.0);
    assert_eq!(constrain_saved_frame(saved, visible), saved);

    // 显示器断开或分辨率降低后，完整窗口仍位于可用区域。
    assert_eq!(
        constrain_saved_frame(rect(4000.0, -300.0, 1000.0, 700.0), visible),
        rect(-1000.0, 40.0, 1000.0, 700.0),
    );
    assert_eq!(constrain_saved_frame(rect(-3000.0, 0.0, 4000.0, 2000.0), visible), visible);
    assert_eq!(constrain_saved_frame(rect(f64::NAN, 0.0, 1000.0, 700.0), visible), visible);
}
