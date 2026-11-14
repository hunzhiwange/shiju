//! GURL Apple Event bridge, installed before Iced starts pumping AppKit events.
use std::ffi::{c_char, c_void};
use std::sync::Once;
use objc2::{class, msg_send, sel};
use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2_foundation::NSString;

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

/// Must be called on the GUI main thread before entering the event loop.
/// The registered target intentionally lives until process exit.
pub fn install_url_handler() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| unsafe {
        assert!(libc::pthread_main_np() != 0, "URL handler requires main thread");
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
