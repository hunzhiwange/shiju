//! AppKit bridge for URL and document opens.
use std::ffi::{c_char, c_void};
use std::sync::Once;
use objc2::{class, msg_send, sel};
use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2_app_kit::{NSApplication, NSApplicationDelegateReply};
use objc2_foundation::{NSArray, NSString};

#[link(name = "objc")]
unsafe extern "C" {
    fn objc_allocateClassPair(superclass: *const AnyClass, name: *const c_char, extra: usize) -> *mut AnyClass;
    fn objc_registerClassPair(class: *mut AnyClass);
    fn class_addMethod(class: *mut AnyClass, selector: Sel, implementation: *const c_void, types: *const c_char) -> bool;
}

unsafe extern "C" fn receive_url(
    _this: *mut AnyObject,
    _selector: Sel,
    event: *mut AnyObject,
    _reply: *mut AnyObject,
) {
    if event.is_null() { return; }
    // SAFETY: AppKit supplies an NSAppleEventDescriptor; '----' is keyDirectObject.
    unsafe {
        let descriptor: *mut AnyObject = msg_send![event, paramDescriptorForKeyword: 0x2d2d2d2du32];
        if descriptor.is_null() { return; }
        let value: *mut NSString = msg_send![descriptor, stringValue];
        if value.is_null() { return; }
        let url = (&*value).to_string();
        if super::receive_arguments(&[url]).is_err() {
            tracing::error!("failed to retain desktop URL event");
        }
    }
}

unsafe extern "C" fn receive_documents(
    _this: *mut AnyObject,
    _selector: Sel,
    app: &NSApplication,
    filenames: &NSArray<NSString>,
) {
    let paths = (0..filenames.count())
        .map(|index| filenames.objectAtIndex(index).to_string())
        .collect();
    let reply = match super::super::single_instance::queue_open_files(paths) {
        Ok(()) => NSApplicationDelegateReply::Success,
        Err(_) => {
            tracing::error!("failed to retain desktop document event");
            NSApplicationDelegateReply::Failure
        }
    };
    app.replyToOpenOrPrint(reply);
}

/// Must be called on the GUI main thread before entering the event loop.
/// The registered target intentionally lives until process exit.
pub fn install_url_handler() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| unsafe {
        let main_thread = objc2::MainThreadMarker::new().expect("Apple Events require the main thread");
        let _app = NSApplication::sharedApplication(main_thread);
        let class = objc_allocateClassPair(class!(NSObject), c"ShiJuURLReceiver".as_ptr(), 0);
        assert!(!class.is_null(), "could not allocate URL receiver class");
        let selector = sel!(handleGetURL:withReplyEvent:);
        assert!(class_addMethod(class, selector, receive_url as *const () as *const c_void, c"v@:@@".as_ptr()));
        objc_registerClassPair(class);
        let target: *mut AnyObject = msg_send![&*class, new];
        assert!(!target.is_null(), "could not allocate URL receiver");
        let manager: *mut AnyObject = msg_send![class!(NSAppleEventManager), sharedAppleEventManager];
        let _: () = msg_send![manager,
            setEventHandler: target,
            andSelector: selector,
            forEventClass: 0x4755524cu32,
            andEventID: 0x4755524cu32
        ];
    });
}

/// Called after Winit sets the application delegate and before AppKit opens startup documents.
pub fn install_document_handler() {
    let main_thread = objc2::MainThreadMarker::new().expect("AppKit requires the main thread");
    let app = NSApplication::sharedApplication(main_thread);
    let Some(delegate) = app.delegate() else {
        tracing::error!("cannot install desktop document handler without an AppKit delegate");
        return;
    };
    let delegate: &AnyObject = (*delegate).as_ref();
    // SAFETY: The callback matches AppKit's application:openFiles: selector and ABI.
    let installed = unsafe {
        class_addMethod(
            std::ptr::from_ref(delegate.class()).cast_mut(),
            sel!(application:openFiles:),
            receive_documents as *const () as *const c_void,
            c"v@:@@".as_ptr(),
        )
    };
    if !installed {
        tracing::warn!("AppKit delegate already implements application:openFiles:");
    }
}
