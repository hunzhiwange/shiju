use gtk::glib::translate::ToGlibPtr;
use gtk::{ButtonsType, DialogFlags, MessageDialog, MessageType, ResponseType, prelude::*};
use url::{Host, Url};
use webkit2gtk::{
    PermissionRequestExt, SettingsExt, UserMediaPermissionRequest, UserMediaPermissionRequestExt,
    WebViewExt,
};
use wry::WebViewExtUnix;

// WebKitGTK 4.1 provides this since 2.34, but webkit2gtk 2.0.2 omits the binding.
unsafe extern "C" {
    fn webkit_user_media_permission_is_for_display_device(
        request: *mut webkit2gtk::ffi::WebKitUserMediaPermissionRequest,
    ) -> gtk::glib::ffi::gboolean;
}

pub(crate) fn install(webview: &wry::WebView, parent: &gtk::Window) {
    let native = webview.webview();
    if let Some(settings) = WebViewExt::settings(&native) {
        settings.set_enable_media_stream(true);
    }
    let parent = parent.downgrade();
    native.connect_permission_request(move |view, request| {
        let Some(media) = request.downcast_ref::<UserMediaPermissionRequest>() else {
            return false;
        };
        // A display request may also include audio. Leave it to WebKit's default policy.
        if unsafe { webkit_user_media_permission_is_for_display_device(media.to_glib_none().0) }
            != 0
        {
            return false;
        }
        let device = match (media.is_for_audio_device(), media.is_for_video_device()) {
            (true, true) => "摄像头和麦克风",
            (true, false) => "麦克风",
            (false, true) => "摄像头",
            (false, false) => return false,
        };
        let Some((parent, uri, origin)) = parent.upgrade().and_then(|parent| {
            let uri = view.uri()?;
            let origin = secure_origin(&uri)?;
            Some((parent, uri, origin))
        }) else {
            request.deny();
            return true;
        };

        let dialog = MessageDialog::new(
            Some(&parent),
            DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
            MessageType::Question,
            ButtonsType::None,
            "媒体设备访问",
        );
        dialog.set_title("媒体设备访问");
        // GTK exposes the top-level URI only. WebKit enforces secure contexts,
        // frame delegation and request lifetime in the engine.
        dialog.set_secondary_text(Some(&format!("允许 {origin} 或其授权的内嵌网页使用{device}？")));
        dialog.add_button("拒绝", ResponseType::Cancel);
        dialog.add_button("允许", ResponseType::Accept);
        dialog.set_default_response(ResponseType::Cancel);
        let view = view.downgrade();
        let request = request.clone();
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept
                && view.upgrade().and_then(|view| view.uri()).as_ref() == Some(&uri)
            {
                request.allow();
            } else {
                request.deny();
            }
            // The response completes this one-shot dialog on the GTK main thread.
            unsafe { dialog.destroy() };
        });
        dialog.show_all();
        true
    });
}

fn secure_origin(uri: &str) -> Option<String> {
    let url = Url::parse(uri).ok()?;
    let host = url.host()?;
    let loopback = match host {
        Host::Domain(domain) => domain == "localhost" || domain.ends_with(".localhost"),
        Host::Ipv4(ip) => ip.is_loopback(),
        Host::Ipv6(ip) => ip.is_loopback(),
    };
    if url.scheme() == "https" || (url.scheme() == "http" && loopback) {
        return Some(url.origin().ascii_serialization());
    }
    None
}
