use gtk::prelude::*;
use webkit2gtk::WebViewExt;
use wry::{
    NewWindowFeatures, NewWindowResponse, WebView, WebViewBuilder, WebViewBuilderExtUnix,
    WebViewExtUnix,
};

use crate::DEFAULT_TITLE;

pub(super) struct Host(gtk::Window);

impl Host {
    pub(super) fn build(
        builder: WebViewBuilder<'_>,
        features: NewWindowFeatures,
        on_close: impl Fn() + 'static,
    ) -> wry::Result<(WebView, Self)> {
        let host = Self(gtk::Window::new(gtk::WindowType::Toplevel));
        host.0.set_title(DEFAULT_TITLE);
        host.0.set_default_size(960, 720);
        let opener = features.opener.webview;
        if let Some(parent) = opener.toplevel().and_downcast::<gtk::Window>() {
            host.0.set_transient_for(Some(&parent));
        }
        let webview = builder.with_related_view(opener).build_gtk(&host.0)?;
        crate::media_permissions::install(&webview, &host.0);
        // Wry destroys the native widget on window.close(). GTK also destroys
        // that widget when its containing window closes.
        let native = webview.webview();
        native.connect_destroy(move |_| on_close());
        let window = host.0.downgrade();
        native.connect_ready_to_show(move |_| {
            if let Some(window) = window.upgrade() {
                window.show_all();
            }
        });
        Ok((webview, host))
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        // The widget belongs to the GTK main thread and destruction is idempotent.
        unsafe { self.0.destroy() };
    }
}

pub(super) fn response(webview: &WebView) -> NewWindowResponse {
    NewWindowResponse::Create { webview: webview.webview() }
}
