//! A native file drag carries the installed application into System Settings.
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSBackingStoreType, NSDragOperation, NSDraggingContext, NSDraggingItem, NSDraggingSession,
    NSDraggingSource, NSEvent, NSImageView, NSPanel, NSScreen, NSTextField, NSWindowStyleMask,
    NSWorkspace,
};
use objc2_foundation::{NSArray, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString, NSURL};
use std::cell::RefCell;

thread_local! {
    static GUIDE: RefCell<Option<Retained<NSPanel>>> = const { RefCell::new(None) };
}

pub(super) fn close() {
    GUIDE.with(|guide| {
        if let Some(panel) = guide.take() {
            panel.close();
        }
    });
}

define_class!(
    // SAFETY: The NSImageView subclass and its URL are only used on the AppKit main thread.
    #[unsafe(super(NSImageView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Retained<NSURL>]
    struct ComputerUseDragIcon;

    unsafe impl NSObjectProtocol for ComputerUseDragIcon {}

    impl ComputerUseDragIcon {
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool { true }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {}

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            let Some(image) = self.image() else { return; };
            let item = NSDraggingItem::initWithPasteboardWriter(
                NSDraggingItem::alloc(), ProtocolObject::from_ref(&**self.ivars()),
            );
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            let frame = NSRect::new(NSPoint::new(point.x - 20.0, point.y - 20.0), NSSize::new(40.0, 40.0));
            // SAFETY: AppKit accepts an NSImage as the drag image contents.
            unsafe { item.setDraggingFrame_contents(frame, Some(&image)) };
            let items = NSArray::from_retained_slice(&[item]);
            self.beginDraggingSessionWithItems_event_source(&items, event, ProtocolObject::from_ref(self));
        }
    }

    unsafe impl NSDraggingSource for ComputerUseDragIcon {
        #[unsafe(method(draggingSession:sourceOperationMaskForDraggingContext:))]
        fn drag_operation(&self, _session: &NSDraggingSession, _context: NSDraggingContext) -> NSDragOperation {
            NSDragOperation::Copy
        }
    }
);

pub(super) fn show(screen_recording: bool) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("授权窗口须在主线程打开")?;
    let path = desktop_config::config_cli_bootstrap::installed_computer_use_app()?;
    let path = NSString::from_str(path.to_str().ok_or("执行应用路径无法显示")?);
    let file = NSURL::initFileURLWithPath_isDirectory(NSURL::alloc(), &path, true);
    let workspace = NSWorkspace::sharedWorkspace();
    let image = workspace.iconForFile(&path);
    let (pane, label) = if screen_recording {
        ("Privacy_ScreenCapture", "拖入「录屏与系统录音」列表")
    } else {
        ("Privacy_Accessibility", "拖入「辅助功能」列表")
    };
    let url = NSString::from_str(&format!(
        "x-apple.systempreferences:com.apple.preference.security?{pane}"
    ));
    let url = NSURL::URLWithString(&url).ok_or("无法定位系统权限设置")?;
    if !workspace.openURL(&url) {
        return Err("无法打开系统权限设置".into());
    }

    let frame = NSRect::new(NSPoint::ZERO, NSSize::new(328.0, 88.0));
    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
        NSPanel::alloc(mtm),
        frame,
        NSWindowStyleMask::Titled
            | NSWindowStyleMask::Closable
            | NSWindowStyleMask::NonactivatingPanel,
        NSBackingStoreType::Buffered,
        false,
    );
    panel.setTitle(&NSString::from_str("电脑操控授权"));
    panel.setFloatingPanel(true);
    panel.setHidesOnDeactivate(false);
    // SAFETY: GUIDE retains the panel until it is closed or replaced.
    unsafe { panel.setReleasedWhenClosed(false) };
    let content = panel.contentView().ok_or("无法显示授权窗口")?;

    let icon_frame = NSRect::new(NSPoint::new(12.0, 12.0), NSSize::new(64.0, 64.0));
    let icon = ComputerUseDragIcon::alloc(mtm).set_ivars(file);
    // SAFETY: The ivars are initialized and NSImageView implements initWithFrame:.
    let icon: Retained<ComputerUseDragIcon> =
        unsafe { msg_send![super(icon), initWithFrame: icon_frame] };
    icon.setImage(Some(&image));
    content.addSubview(&icon);

    let name = NSTextField::labelWithString(&NSString::from_str("ShiJu Computer Use"), mtm);
    name.setFrame(NSRect::new(NSPoint::new(84.0, 48.0), NSSize::new(232.0, 20.0)));
    content.addSubview(&name);
    let hint = NSTextField::labelWithString(&NSString::from_str(label), mtm);
    hint.setFrame(NSRect::new(NSPoint::new(84.0, 22.0), NSSize::new(232.0, 20.0)));
    content.addSubview(&hint);

    if let Some(screen) = NSScreen::mainScreen(mtm) {
        let visible = screen.visibleFrame();
        panel.setFrameOrigin(NSPoint::new(
            visible.origin.x + (visible.size.width - 344.0).max(0.0),
            visible.origin.y + (visible.size.height - 152.0).max(0.0),
        ));
    }
    close();
    GUIDE.with(|guide| guide.replace(Some(panel.clone())));
    panel.orderFrontRegardless();
    Ok(())
}
