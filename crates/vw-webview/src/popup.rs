use std::cell::{Cell, RefCell};
use std::collections::HashMap;
#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

use tao::event_loop::EventLoopProxy;
use wry::{NewWindowFeatures, NewWindowResponse, WebView, WebViewBuilder};

use crate::{UserEvent, append_log, is_page_link_url};

#[cfg(target_os = "macos")]
#[path = "popup/macos.rs"]
mod platform;
#[cfg(target_os = "windows")]
#[path = "popup/windows.rs"]
mod platform;
#[cfg(unix)]
#[cfg(not(target_os = "macos"))]
#[path = "popup/unix.rs"]
mod platform;

struct Popup {
    // Drop the WebView while its native host is still alive.
    _webview: WebView,
    _host: platform::Host,
}

pub(crate) struct PopupWindows {
    windows: RefCell<HashMap<u64, Popup>>,
    next_id: Cell<u64>,
    proxy: EventLoopProxy<UserEvent>,
    log_path: PathBuf,
    #[cfg(windows)]
    owner: platform::Owner,
}

impl PopupWindows {
    pub(crate) fn new(
        proxy: EventLoopProxy<UserEvent>,
        log_path: PathBuf,
        #[cfg(windows)] owner: &tao::window::Window,
    ) -> Rc<Self> {
        Rc::new(Self {
            windows: RefCell::default(),
            next_id: Cell::new(0),
            proxy,
            log_path,
            #[cfg(windows)]
            owner: platform::Owner::new(owner),
        })
    }

    pub(crate) fn with_handler<'a>(
        self: &Rc<Self>,
        builder: WebViewBuilder<'a>,
    ) -> WebViewBuilder<'a> {
        let windows = Rc::downgrade(self);
        builder.with_new_window_req_handler(move |url, features| {
            if !allows_navigation(&url) {
                return NewWindowResponse::Deny;
            }
            let Some(windows) = windows.upgrade() else {
                return NewWindowResponse::Deny;
            };
            windows.open(features).unwrap_or_else(|_| {
                // Native errors can include request data. Keep URLs and credentials out of logs.
                append_log(&windows.log_path, "WebView could not create a popup window.");
                NewWindowResponse::Deny
            })
        })
    }

    fn open(self: &Rc<Self>, features: NewWindowFeatures) -> wry::Result<NewWindowResponse> {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        let proxy = self.proxy.clone();
        let on_close = move || {
            let _ = proxy.send_event(UserEvent::PopupClosed(id));
        };

        // The browser still owns the navigation. Creating this view with a URL
        // would race the pending request and discard its method and body.
        let builder = self.with_handler(
            WebViewBuilder::new()
                .with_clipboard(true)
                .with_accept_first_mouse(true)
                .with_navigation_handler(|url| allows_navigation(&url)),
        );
        let (webview, host) = platform::Host::build(
            builder,
            features,
            on_close,
            #[cfg(target_os = "macos")]
            &self.log_path,
            #[cfg(windows)]
            self.owner,
        )?;
        let response = platform::response(&webview);
        self.windows.borrow_mut().insert(id, Popup { _webview: webview, _host: host });
        Ok(response)
    }

    pub(crate) fn close(&self, id: u64) {
        // The native callbacks only enqueue close events. Drop outside the map borrow.
        let closed = self.windows.borrow_mut().remove(&id);
        drop(closed);
    }
}

fn allows_navigation(url: &str) -> bool {
    url == "about:blank" || is_page_link_url(url)
}

#[cfg(unix)]
pub(crate) fn with_downloads<'a>(
    builder: WebViewBuilder<'a>,
    log_path: &Path,
) -> WebViewBuilder<'a> {
    let log_path = log_path.to_owned();
    builder.with_download_started_handler(|_, _| true).with_download_completed_handler(
        move |_, _, success| {
            if !success {
                append_log(&log_path, "WebView download failed.");
            }
        },
    )
}
