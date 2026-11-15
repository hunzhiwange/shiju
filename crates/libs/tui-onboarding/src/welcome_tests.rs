// 快照覆盖紧凑与动画断点两种布局，确保复制来的间距约定不被破坏。
use super::*;

#[test]
// 紧凑区域只渲染品牌欢迎文案，并保持上游控件的空白布局。
fn welcome_renders_vibewindow_copy_with_upstream_spacing() {
    let area = Rect::new(0, 0, 72, 2);
    let mut buf = Buffer::empty(area);
    let widget = WelcomeWidget::new();
    (&widget).render_ref(area, &mut buf);

    insta::assert_snapshot!(format!("{buf:?}"));
}

#[test]
// 达到宽高阈值时应同时渲染通用字符画与 VibeWindow 文案。
fn welcome_renders_copied_generic_frame_at_upstream_breakpoint() {
    let area = Rect::new(0, 0, MIN_ANIMATION_WIDTH, MIN_ANIMATION_HEIGHT);
    let mut buf = Buffer::empty(area);
    let widget = WelcomeWidget::new();
    (&widget).render_ref(area, &mut buf);

    let rendered = format!("{buf:?}");
    assert!(rendered.contains("Welcome to ShiJu"));
    insta::assert_snapshot!(rendered);
}
