use super::*;

#[test]
// 视口尺寸变化后偏移应收敛到新最大值，而合法的小偏移保持不变。
fn scroll_offset_is_clamped_for_resize() {
    assert_eq!(max_scroll_offset(20, 8), 12);
    assert_eq!(clamp_offset(17, 20, 8), 12);
    assert_eq!(clamp_offset(3, 20, 8), 3);
    assert_eq!(clamp_offset(3, 2, 8), 0);
}
