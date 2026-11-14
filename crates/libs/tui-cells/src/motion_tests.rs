use pretty_assertions::assert_eq;

use super::*;

#[test]
// 减少动态效果模式必须严格采用调用方声明的静态回退。
fn reduced_motion_activity_indicator_uses_explicit_fallback() {
    assert_eq!(
        activity_indicator(
            /*start_time*/ None,
            MotionMode::Reduced,
            ReducedMotionIndicator::Hidden,
        ),
        None
    );
    assert_eq!(
        activity_indicator(
            /*start_time*/ None,
            MotionMode::Reduced,
            ReducedMotionIndicator::StaticBullet,
        ),
        Some("•".dim())
    );
}

#[test]
// 关闭动画后闪烁文本保持原文，空输入仍不产生 span。
fn reduced_motion_shimmer_text_is_plain_text() {
    assert_eq!(shimmer_text("Loading", MotionMode::Reduced), vec!["Loading".into()]);
    assert_eq!(shimmer_text("", MotionMode::Reduced), Vec::<Span<'static>>::new());
}
