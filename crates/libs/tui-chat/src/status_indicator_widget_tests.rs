use super::*;

#[test]
fn working_spinner_advances_and_wraps() {
    assert_eq!(working_spinner_frame(Duration::ZERO), "⠋");
    assert_eq!(working_spinner_frame(Duration::from_millis(90)), "⠙");
    assert_eq!(working_spinner_frame(Duration::from_millis(900)), "⠋");
}

#[test]
// 同时固定宽屏详情与窄屏截断两种布局快照。
fn status_indicator_keeps_working_and_narrow_layout() {
    let status =
        V3ChatStatus::running("Working", Some("reading copied components".to_string()));
    let widget = StatusIndicatorWidget::new(&status, Duration::ZERO);
    let wide_area = Rect::new(0, 0, 80, 2);
    let mut wide = Buffer::empty(wide_area);
    widget.render(wide_area, &mut wide);
    let narrow_area = Rect::new(0, 0, 24, 3);
    let mut narrow = Buffer::empty(narrow_area);
    widget.render(narrow_area, &mut narrow);

    insta::with_settings!({snapshot_path => concat!(env!("CARGO_MANIFEST_DIR"), "/../../vw-tui/src/snapshots"), prepend_module_to_snapshot => false}, {
        insta::assert_snapshot!("vw_tui__status_indicator_widget__tests__status_indicator_vibe_wide", format!("{wide:?}"));
    });
    insta::with_settings!({snapshot_path => concat!(env!("CARGO_MANIFEST_DIR"), "/../../vw-tui/src/snapshots"), prepend_module_to_snapshot => false}, {
        insta::assert_snapshot!("vw_tui__status_indicator_widget__tests__status_indicator_vibe_narrow", format!("{narrow:?}"));
    });
}

#[test]
fn status_indicator_renders_supplied_elapsed_seconds() {
    let status = V3ChatStatus::running("Working", None);
    let widget = StatusIndicatorWidget::new(&status, Duration::from_secs(7));
    let area = Rect::new(0, 0, 80, 1);
    let mut buffer = Buffer::empty(area);

    widget.render(area, &mut buffer);

    let rendered = format!("{buffer:?}");
    assert!(rendered.contains("• ⠋ Working"));
    assert!(rendered.contains("(7s"));
}
