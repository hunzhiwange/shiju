use std::num::NonZeroIsize;
use tao::platform::windows::WindowExtWindows;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DestroyWindow, IsWindow, SW_SHOW, ShowWindow, WM_NCDESTROY,
    WS_CLIPCHILDREN, WS_OVERLAPPEDWINDOW,
};
use windows::core::{Error, w};
use wry::raw_window_handle::{HandleError, HasWindowHandle, Win32WindowHandle, WindowHandle};
use wry::{
    NewWindowFeatures, NewWindowResponse, WebView, WebViewBuilder, WebViewBuilderExtWindows,
    WebViewExtWindows,
};

#[derive(Clone, Copy)]
pub(super) struct Owner(HWND);

impl Owner {
    pub(super) fn new(window: &tao::window::Window) -> Self {
        Self(HWND(window.hwnd() as _))
    }
}

pub(super) struct Host(HWND);

impl HasWindowHandle for Host {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let handle = NonZeroIsize::new(self.0.0 as isize).ok_or(HandleError::Unavailable)?;
        // The host owns the HWND for the duration of WebView construction.
        Ok(unsafe { WindowHandle::borrow_raw(Win32WindowHandle::new(handle).into()) })
    }
}

impl Host {
    pub(super) fn build<F: Fn() + 'static>(
        builder: WebViewBuilder<'_>,
        features: NewWindowFeatures,
        on_close: F,
        owner: Owner,
    ) -> wry::Result<(WebView, Self)> {
        // Wry dispatches NewWindowRequested after the WebView2 COM callback
        // returns, on the thread that owns the opener and this HWND.
        let (width, height) = features.size.map_or((960, 720), |size| {
            (size.width.max(320.0) as i32, size.height.max(240.0) as i32)
        });
        let host = Self(unsafe {
            CreateWindowExW(
                Default::default(),
                w!("STATIC"),
                w!("Vibe Window WebView"),
                WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width,
                height,
                Some(owner.0),
                None,
                None,
                None,
            )?
        });
        let webview = builder.with_environment(features.opener.environment).build(&host)?;
        // The WebView2 close event destroys Wry's container HWND. Watch that
        // HWND so both script close and closing the host frame free the entry.
        let mut child = HWND::default();
        unsafe { webview.controller().ParentWindow(&mut child).map_err(std::io::Error::other)? };
        let data = Box::into_raw(Box::new(on_close));
        if !unsafe { SetWindowSubclass(child, Some(close_subclass::<F>), 1, data as usize) }
            .as_bool()
        {
            unsafe { drop(Box::from_raw(data)) };
            return Err(std::io::Error::other(Error::from_win32()).into());
        }
        let _ = unsafe { ShowWindow(host.0, SW_SHOW) };
        Ok((webview, host))
    }
}

unsafe extern "system" fn close_subclass<F: Fn()>(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    data: usize,
) -> LRESULT {
    if message == WM_NCDESTROY {
        // Windows delivers the terminal notification once for this HWND.
        let callback = unsafe { Box::from_raw(data as *mut F) };
        callback();
    }
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

impl Drop for Host {
    fn drop(&mut self) {
        // The native close button may already have destroyed the host HWND.
        if unsafe { IsWindow(Some(self.0)) }.as_bool() {
            let _ = unsafe { DestroyWindow(self.0) };
        }
    }
}

pub(super) fn response(webview: &WebView) -> NewWindowResponse {
    NewWindowResponse::Create { webview: webview.webview() }
}
