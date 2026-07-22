use ratatui::layout::Rect;

use super::layout::{FullscreenLayoutSlots, compute_fullscreen_layout};

#[test]
fn compute_fullscreen_layout_returns_default_for_empty_area() {
    assert_eq!(
        compute_fullscreen_layout(Rect::new(0, 0, 0, 20), true, true, 0),
        FullscreenLayoutSlots::default()
    );
    assert_eq!(
        compute_fullscreen_layout(Rect::new(0, 0, 80, 0), false, false, 0),
        FullscreenLayoutSlots::default()
    );
}

#[test]
fn compute_fullscreen_layout_places_sections_and_modal() {
    let slots = compute_fullscreen_layout(Rect::new(2, 4, 120, 40), true, true, 2);

    assert_eq!(slots.header, Rect::new(2, 4, 120, 3));
    assert_eq!(slots.bottom_float, Some(Rect::new(2, 32, 120, 3)));
    assert_eq!(slots.bottom, Rect::new(2, 35, 120, 9));
    assert_eq!(slots.scrollable.y, 7);
    assert_eq!(slots.scrollable.height, 25);
    assert!(slots.modal.is_some());
    assert_eq!(slots.message_viewport_capacity(), 23);
}

#[test]
fn compute_fullscreen_layout_omits_bottom_float_on_compact_height() {
    let slots = compute_fullscreen_layout(Rect::new(0, 0, 80, 12), true, false, 8);

    assert_eq!(slots.header.height, 2);
    assert_eq!(slots.scrollable.height, 1);
    assert_eq!(slots.bottom.height, 9);
    assert_eq!(slots.bottom_float, None);
    assert_eq!(slots.modal, None);
    assert_eq!(slots.message_viewport_capacity(), 0);
}

#[test]
fn compute_fullscreen_layout_splits_wide_scrollable_area_into_sidebars() {
    let slots = compute_fullscreen_layout(Rect::new(0, 0, 140, 50), false, false, 0);

    let project = slots.project_context.expect("wide layouts should include project context");
    let modified = slots.modified_files.expect("wide layouts should include modified files");
    assert_eq!(project.x, slots.scrollable.x + slots.scrollable.width);
    assert_eq!(project.width, modified.width);
    assert_eq!(project.height, 16);
    assert_eq!(modified.y, project.y + project.height);
    assert_eq!(project.height + modified.height, slots.scrollable.height);
}

#[test]
fn compute_fullscreen_layout_skips_sidebars_when_width_or_height_is_too_small() {
    let narrow = compute_fullscreen_layout(Rect::new(0, 0, 87, 40), false, false, 0);
    assert!(narrow.project_context.is_none());
    assert!(narrow.modified_files.is_none());

    let short = compute_fullscreen_layout(Rect::new(0, 0, 120, 22), false, false, 0);
    assert!(short.project_context.is_none());
    assert!(short.modified_files.is_none());
}

#[test]
fn compute_fullscreen_layout_preserves_minimum_slots_in_two_row_area() {
    let slots = compute_fullscreen_layout(Rect::new(10, 10, 1, 2), true, true, 0);

    assert_eq!(slots.header, Rect::new(10, 10, 1, 1));
    assert_eq!(slots.scrollable, Rect::new(10, 11, 1, 1));
    assert_eq!(slots.bottom, Rect::new(10, 12, 1, 1));
    assert_eq!(slots.bottom_float, None);
    assert_eq!(slots.modal, Some(Rect::new(10, 10, 1, 1)));
    assert_eq!(slots.message_viewport_capacity(), 0);
}
