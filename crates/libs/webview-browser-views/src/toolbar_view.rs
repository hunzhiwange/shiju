//! Browser toolbar construction and GTK packing.

use tao::{event_loop::EventLoopProxy, window::Window};
use webview_core::events::UserEvent;
use webview_toolbar::{ToolbarAction, ViewRegion, parse_toolbar_ipc_message, toolbar_html};
use wry::{WebView, WebViewBuilder};
#[cfg(all(unix, not(target_os = "macos")))]
use {gtk::prelude::*, tao::platform::unix::WindowExtUnix, wry::{WebViewBuilderExtUnix, WebViewExtUnix}};
use crate::to_wry_rect;

pub fn build_toolbar_view(
    window: &Window,
    region: ViewRegion,
    read_only_address: bool,
    proxy: &EventLoopProxy<UserEvent>,
) -> wry::Result<WebView> {
    #[cfg(all(unix, not(target_os = "macos")))]
    let gtk_container = window.default_vbox();
    let toolbar_proxy = proxy.clone();
    let toolbar_ready_proxy = proxy.clone();
    let toolbar_builder = WebViewBuilder::new()
        .with_html(if read_only_address {
            toolbar_html()
                .replace("type=\"text\"", "type=\"text\" readonly")
                .replace("post('navigate', address.value);", "")
        } else {
            toolbar_html()
        })
        .with_clipboard(true)
        .with_bounds(to_wry_rect(region))
        .with_on_page_load_handler(move |event, _url| {
            if matches!(event, wry::PageLoadEvent::Finished) {
                let _ = toolbar_ready_proxy.send_event(UserEvent::ToolbarReady);
            }
        })
        .with_ipc_handler(move |request| {
            let Some(action) = parse_toolbar_ipc_message(request.body()) else {
                return;
            };
            let event = match action {
                ToolbarAction::Back => UserEvent::Back,
                ToolbarAction::Forward => UserEvent::Forward,
                ToolbarAction::Navigate(url) => UserEvent::OpenUrl(url),
                ToolbarAction::Reload => UserEvent::Reload,
                ToolbarAction::OpenExternal(url) => UserEvent::OpenExternal(url),
                ToolbarAction::Clear => UserEvent::ClearBrowser,
            };
            let _ = toolbar_proxy.send_event(event);
        });

    #[cfg(all(unix, not(target_os = "macos")))]
    let toolbar_result =
        gtk_container.ok_or(wry::Error::UnsupportedWindowHandle).and_then(|container| {
            // GTK must manage input focus for both views. Wrapping independent
            // X11 child windows leaves the address bar without keyboard input.
            let toolbar = toolbar_builder.build_gtk(container)?;
            let native = toolbar.webview();
            native.set_size_request(-1, region.height as i32);
            container.set_child_packing(&native, false, true, 0, gtk::PackType::Start);
            Ok(toolbar)
        });
    #[cfg(not(all(unix, not(target_os = "macos"))))]
    let toolbar_result = toolbar_builder.build_as_child(window);

    toolbar_result
}
