use std::cell::Cell;

use gtk::{ButtonsType, DialogFlags, MessageDialog, MessageType, ResponseType, gio, prelude::*};
use tao::platform::unix::WindowExtUnix;
use wry::{WebView, WebViewExtUnix};

#[derive(Clone)]
pub(crate) struct Parent(gtk::Window);

impl Parent {
    pub(super) fn from_window(window: &tao::window::Window) -> Option<Self> {
        // X11 child WebViews have an internal GTK host; dialogs belong to the
        // real top-level Tao window, whether the browser is embedded or not.
        let window = window.gtk_window().clone().upcast::<gtk::Window>();
        window.is_visible().then_some(Self(window))
    }

    pub(super) fn from_webview(webview: &WebView) -> Option<Self> {
        let window = webview.webview().toplevel()?.downcast::<gtk::Window>().ok()?;
        window.is_visible().then_some(Self(window))
    }

    pub(super) fn show(
        &self,
        title: &str,
        message: &str,
        confirm: bool,
        done: impl FnOnce(bool) + 'static,
    ) {
        let dialog = MessageDialog::new(
            Some(&self.0),
            DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
            if confirm { MessageType::Question } else { MessageType::Warning },
            if confirm { ButtonsType::None } else { ButtonsType::Ok },
            title,
        );
        dialog.set_title(title);
        dialog.set_secondary_text(Some(message));
        if confirm {
            dialog.add_button("取消", ResponseType::Cancel);
            dialog.add_button("打开", ResponseType::Accept);
            dialog.set_default_response(ResponseType::Cancel);
        }
        let done = Cell::new(Some(done));
        dialog.connect_response(move |dialog, response| {
            let done = done.take();
            dialog.hide();
            // This dialog is owned by the GTK main thread and is no longer used.
            unsafe { dialog.destroy() };
            if let Some(done) = done {
                done(confirm && response == ResponseType::Accept);
            }
        });
        dialog.show_all();
    }

    pub(super) fn open(&self, scheme: &str, url: &str) -> bool {
        // Require an actual registered URI handler. Generic desktop openers
        // can fall back to a web browser and report success without a handler.
        gio::AppInfo::default_for_uri_scheme(scheme)
            .is_some_and(|app| app.launch_uris(&[url], None::<&gio::AppLaunchContext>).is_ok())
    }
}
