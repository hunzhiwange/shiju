//! Content WebView construction; callers retain context and popup ownership.

use std::{path::PathBuf, rc::Rc};
#[cfg(unix)]
use std::path::Path;
use tao::{event_loop::EventLoopProxy, window::Window};
use webview_core::events::UserEvent;
use webview_ipc::parse_navigation_ipc_message;
use webview_local_content::local_protocol_response;
use webview_popup::PopupWindows;
use webview_toolbar::ViewRegion;
use wry::{WebContext, WebView, WebViewBuilder};
#[cfg(all(unix, not(target_os = "macos")))]
use {tao::platform::unix::WindowExtUnix, wry::WebViewBuilderExtUnix};
use crate::to_wry_rect;

pub fn build_content_view(
    window: &Window,
    web_context: &mut WebContext,
    initial_url: &str,
    init_script: &str,
    #[cfg(target_os = "macos")] user_agent: &'static str,
    #[cfg(unix)] log_path: &Path,
    local_root: Option<PathBuf>,
    independent_mode: bool,
    content_region: ViewRegion,
    proxy: &EventLoopProxy<UserEvent>,
    popups: &Rc<PopupWindows>,
) -> wry::Result<WebView> {
    #[cfg(all(unix, not(target_os = "macos")))]
    let gtk_container = window.default_vbox();
    let mut builder = {
        let page_load_proxy = proxy.clone();
        let navigation_proxy = proxy.clone();
        WebViewBuilder::new_with_web_context(web_context)
            .with_url(initial_url)
            .with_devtools(true)
            .with_clipboard(true)
            .with_accept_first_mouse(true)
            .with_initialization_script(init_script)
            .with_on_page_load_handler(move |_event, url| {
                let _ = page_load_proxy.send_event(UserEvent::NavigationChanged(url));
            })
            .with_ipc_handler(move |request| {
                if let Some(url) = parse_navigation_ipc_message(request.body()) {
                    let _ = navigation_proxy.send_event(UserEvent::NavigationChanged(url));
                }
            })
    };

    #[cfg(target_os = "macos")]
    {
        builder = builder.with_user_agent(user_agent);
    }
    #[cfg(unix)]
    {
        // WebKitGTK shares this download handler with related popup views.
        builder = webview_downloads::with_downloads(builder, log_path);
    }

    builder = popups.with_handler(builder, None, local_root.is_some());

    if let Some(root) = local_root {
        builder = builder.with_custom_protocol("vibe".into(), move |_webview_id, request| {
            local_protocol_response(&root, request)
        });
    }

    if independent_mode {
        builder = builder.with_bounds(to_wry_rect(content_region));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    let webview_result = if independent_mode {
        gtk_container
            .ok_or(wry::Error::UnsupportedWindowHandle)
            .and_then(|container| builder.build_gtk(container))
    } else {
        builder.build(window)
    };
    #[cfg(not(all(unix, not(target_os = "macos"))))]
    let webview_result =
        if independent_mode { builder.build_as_child(window) } else { builder.build(window) };
    webview_result
}
