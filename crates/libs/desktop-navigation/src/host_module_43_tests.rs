//! Layout ownership smoke coverage; real scroll and window contracts stay in the host.
use super::*;
use iced::widget::Space;

#[test]
fn notification_layer_keeps_page_and_panel() {
    let view = with_notifications::<()>(Space::new().into(), Space::new().into());
    assert_eq!(view.as_widget().children().len(), 2);
}

#[test]
fn toast_layer_stays_mounted_for_empty_content() {
    for bottom in [44, 98] {
        let view = with_toast::<()>(Space::new().into(), Space::new().into(), bottom);
        assert_eq!(view.as_widget().children().len(), 2);
    }
}

#[test]
fn absent_wallpaper_keeps_original_widget() {
    let original: iced::Element<'_, ()> = Space::new().into();
    let tag = original.as_widget().tag();
    let view = with_wallpaper(original, None);
    assert_eq!(view.as_widget().tag(), tag);
}
