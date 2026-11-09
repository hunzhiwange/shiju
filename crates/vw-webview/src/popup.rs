use std::cell::{Cell, RefCell};
use std::collections::HashMap;
#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

use tao::event_loop::EventLoopProxy;
use wry::{NewWindowFeatures, NewWindowResponse, WebView, WebViewBuilder};

use crate::external_protocol::{ExternalProtocols, Navigation, navigation};
use crate::{UserEvent, append_log};

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
    webview: WebView,
    _host: platform::Host,
}

pub(crate) struct PopupWindows {
    windows: RefCell<HashMap<u64, Popup>>,
    next_id: Cell<u64>,
    proxy: EventLoopProxy<UserEvent>,
    external: ExternalProtocols,
    log_path: PathBuf,
    #[cfg(windows)]
    owner: platform::Owner,
}

impl PopupWindows {
    pub(crate) fn new(
        proxy: EventLoopProxy<UserEvent>,
        log_path: PathBuf,
        external: ExternalProtocols,
        #[cfg(windows)] owner: &tao::window::Window,
    ) -> Rc<Self> {
        Rc::new(Self {
            windows: RefCell::default(),
            next_id: Cell::new(0),
            proxy,
            external,
            log_path,
            #[cfg(windows)]
            owner: platform::Owner::new(owner),
        })
    }

    pub(crate) fn with_handler<'a>(
        self: &Rc<Self>,
        builder: WebViewBuilder<'a>,
        popup: Option<u64>,
        local_site: bool,
    ) -> WebViewBuilder<'a> {
        let navigation_windows = Rc::downgrade(self);
        let popup_windows = Rc::downgrade(self);
        builder
            .with_navigation_handler(move |url| match navigation(&url, local_site) {
                Navigation::InWebView => true,
                Navigation::External(target) => {
                    if let Some(windows) = navigation_windows.upgrade() {
                        windows.external.request(target, popup, &windows.proxy);
                    }
                    false
                }
                Navigation::Blocked => false,
            })
            .with_new_window_req_handler(move |url, features| {
                let Some(windows) = popup_windows.upgrade() else {
                    return NewWindowResponse::Deny;
                };
                match navigation(&url, false) {
                    Navigation::InWebView => {}
                    Navigation::External(target) => {
                        windows.external.request(target, popup, &windows.proxy);
                        return NewWindowResponse::Deny;
                    }
                    Navigation::Blocked => return NewWindowResponse::Deny,
                }
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
            WebViewBuilder::new().with_clipboard(true).with_accept_first_mouse(true),
            Some(id),
            false,
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
        #[cfg(windows)]
        crate::external_protocol::block_native_external_launches(&webview, false)?;
        let response = platform::response(&webview);
        self.windows.borrow_mut().insert(id, Popup { webview, _host: host });
        Ok(response)
    }

    pub(crate) fn show_external_confirmation(
        &self,
        id: u64,
        target: crate::external_protocol::ExternalTarget,
        confirmation: crate::external_protocol::Confirmation,
    ) {
        // On Windows the dialog pumps messages. Never hold a borrow across it.
        let parent = self
            .windows
            .borrow()
            .get(&id)
            .and_then(|popup| crate::external_protocol::dialog_parent(&popup.webview));
        if let Some(parent) = parent {
            crate::external_protocol::show_confirmation(parent, target, confirmation);
        }
    }

    pub(crate) fn close(&self, id: u64) {
        // The native callbacks only enqueue close events. Drop outside the map borrow.
        let closed = self.windows.borrow_mut().remove(&id);
        drop(closed);
    }
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
