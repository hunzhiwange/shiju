//! AppKit drag destinations do not receive ordinary cursor-moved events.

/// Query the destination window's content coordinates at the native drop instead
/// of using Iced's possibly absent or stale mouse position.
pub(super) fn cursor_position() -> Option<iced::Point> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSEvent, NSWindow};

    let main_thread = MainThreadMarker::new()?;
    let number = NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(
        NSEvent::mouseLocation(),
        0,
        main_thread,
    );
    let window = NSApplication::sharedApplication(main_thread).windowWithWindowNumber(number)?;
    let view = window.contentView()?;
    let position = view.convertPoint_fromView(window.mouseLocationOutsideOfEventStream(), None);
    let bounds = view.bounds();
    let x = position.x - bounds.origin.x;
    let y = if view.isFlipped() {
        position.y - bounds.origin.y
    } else {
        bounds.origin.y + bounds.size.height - position.y
    };
    // AppKit points and Iced logical coordinates both exclude Retina scaling.
    Some(iced::Point::new(x as f32, y as f32))
}
