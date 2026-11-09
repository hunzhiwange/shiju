use tao::platform::windows::WindowExtWindows;
use webview2_com::{
    LaunchingExternalUriSchemeEventHandler, Microsoft::Web::WebView2::Win32::ICoreWebView2_18,
    NavigationStartingEventHandler, take_pwstr,
};
use windows::{
    Win32::{
        Foundation::HWND,
        UI::{
            Shell::{SEE_MASK_FLAG_NO_UI, SHELLEXECUTEINFOW, ShellExecuteExW},
            WindowsAndMessaging::{
                GA_ROOT, GetAncestor, IDYES, IsWindowVisible, MB_DEFBUTTON2, MB_ICONQUESTION,
                MB_ICONWARNING, MB_OK, MB_YESNO, MessageBoxW, SW_SHOWNORMAL,
            },
        },
    },
    core::{Interface, PCWSTR, PWSTR},
};
use wry::{WebView, WebViewExtWindows};

use super::{Navigation, navigation};

#[derive(Clone)]
pub(crate) struct Parent(HWND);

impl Parent {
    pub(super) fn from_window(window: &tao::window::Window) -> Option<Self> {
        let hwnd = HWND(window.hwnd() as _);
        window.is_visible().then_some(Self(hwnd))
    }

    pub(super) fn from_webview(webview: &WebView) -> Option<Self> {
        let mut child = HWND::default();
        // The controller and the window are owned by this UI thread.
        unsafe {
            webview.controller().ParentWindow(&mut child).ok()?;
            let owner = GetAncestor(child, GA_ROOT);
            IsWindowVisible(owner).as_bool().then_some(Self(owner))
        }
    }

    pub(super) fn show(
        &self,
        title: &str,
        message: &str,
        confirm: bool,
        done: impl FnOnce(bool) + 'static,
    ) {
        let title = wide(title);
        let message = wide(message);
        let flags = if confirm {
            MB_YESNO | MB_DEFBUTTON2 | MB_ICONQUESTION
        } else {
            MB_OK | MB_ICONWARNING
        };
        // Both strings remain null-terminated for the synchronous native dialog.
        let answer = unsafe {
            MessageBoxW(Some(self.0), PCWSTR(message.as_ptr()), PCWSTR(title.as_ptr()), flags)
        };
        done(confirm && answer == IDYES);
    }

    pub(super) fn open(&self, _scheme: &str, url: &str) -> bool {
        let url = wide(url);
        let mut request = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_FLAG_NO_UI,
            hwnd: self.0,
            lpFile: PCWSTR(url.as_ptr()),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        // The URL is a single UTF-16 string and no command parameters are supplied.
        unsafe { ShellExecuteExW(&mut request).is_ok() }
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

pub(crate) fn block_native_external_launches(
    webview: &WebView,
    local_site: bool,
) -> wry::Result<()> {
    let webview = webview.webview();
    // Wry's NavigationStarting is for the top-level view. Frames must not
    // bypass host confirmation, including on older WebView2 runtimes.
    unsafe {
        webview.add_FrameNavigationStarting(
            &NavigationStartingEventHandler::create(Box::new(move |_, args| {
                if let Some(args) = args {
                    let mut uri = PWSTR::null();
                    args.Uri(&mut uri)?;
                    let uri = take_pwstr(uri);
                    args.SetCancel(!matches!(navigation(&uri, local_site), Navigation::InWebView))?;
                }
                Ok(())
            })),
            &mut 0,
        )
    }
    .map_err(std::io::Error::other)?;

    // Recent runtimes additionally expose the native external-launch event.
    // All top-level requests we support are already routed by Wry before it.
    if let Ok(webview) = webview.cast::<ICoreWebView2_18>() {
        unsafe {
            webview.add_LaunchingExternalUriScheme(
                &LaunchingExternalUriSchemeEventHandler::create(Box::new(|_, args| {
                    if let Some(args) = args {
                        args.SetCancel(true)?;
                    }
                    Ok(())
                })),
                &mut 0,
            )
        }
        .map_err(std::io::Error::other)?;
    }
    Ok(())
}
