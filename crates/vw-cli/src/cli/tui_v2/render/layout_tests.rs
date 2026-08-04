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

    assert_eq!(slots.header, Rect::new(2, 4, 120, 0));
    assert_eq!(slots.bottom_float, None);
    assert_eq!(slots.bottom, Rect::new(2, 34, 120, 10));
    assert_eq!(slots.scrollable, Rect::new(2, 4, 120, 30));
    assert_eq!(slots.scrollable_body, Rect::new(2, 4, 119, 30));
    assert_eq!(slots.scrollbar_track, Some(Rect::new(121, 4, 1, 30)));
    assert!(slots.modal.is_some());
    assert_eq!(slots.message_viewport_capacity(), 30);
}

#[test]
fn compute_fullscreen_layout_omits_bottom_float_on_compact_height() {
    let slots = compute_fullscreen_layout(Rect::new(0, 0, 80, 12), true, false, 8);

    assert_eq!(slots.header.height, 0);
    assert_eq!(slots.scrollable.height, 6);
    assert_eq!(slots.bottom.height, 6);
    assert_eq!(slots.bottom_float, None);
    assert_eq!(slots.modal, None);
    assert_eq!(slots.message_viewport_capacity(), 6);
}

#[test]
fn compute_fullscreen_layout_keeps_wide_transcript_full_width() {
    let slots = compute_fullscreen_layout(Rect::new(0, 0, 140, 50), false, false, 0);

    assert_eq!(slots.scrollable, Rect::new(0, 0, 140, 42));
    assert_eq!(slots.scrollable_body, Rect::new(0, 0, 139, 42));
    assert!(slots.project_context.is_none());
    assert!(slots.modified_files.is_none());
}

#[test]
fn compute_fullscreen_layout_never_allocates_main_screen_sidebars() {
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

    assert_eq!(slots.header, Rect::new(10, 10, 1, 0));
    assert_eq!(slots.scrollable, Rect::new(10, 10, 1, 1));
    assert_eq!(slots.scrollable_body, Rect::new(10, 10, 1, 1));
    assert_eq!(slots.scrollbar_track, None);
    assert_eq!(slots.bottom, Rect::new(10, 11, 1, 1));
    assert_eq!(slots.bottom_float, None);
    assert_eq!(slots.modal, Some(Rect::new(10, 10, 1, 1)));
    assert_eq!(slots.message_viewport_capacity(), 1);
}
